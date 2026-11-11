// Dylib injectée par DYLD_INSERT_LIBRARIES : tient le rôle du proxy de démarrage sous macOS.
//
// À son chargement (avant main), elle essaie dans l'hôte ce dont le portage a besoin :
//   1. de la mémoire exécutable pour les trampolines (MAP_JIT, ou une page RW passée RX) ;
//   2. une accroche par pointeur : l'import `dlopen` de l'exe, le pendant de l'accroche de
//      LoadLibrary dans la table d'imports (startup_proxy/src/chargeur.rs) ;
//   3. un patch en place d'une fonction du __TEXT de l'exe, par plusieurs méthodes.
// Puis, quand l'hôte charge son bundle par dlopen, l'accroche refait l'essai 3 sur une fonction
// du bundle (le pendant des `Site` de GameCore).
//
// Chaque tentative tourne dans un processus fils (fork) : un refus du noyau qui tue le processus
// (SIGKILL « Code Signature Invalid », SIGBUS) devient un verdict au lieu d'arrêter l'essai. La
// première méthode qui passe est ensuite appliquée dans le processus lui-même, et l'hôte constate
// le résultat de bout en bout.

#include <dlfcn.h>
#include <errno.h>
#include <libkern/OSCacheControl.h>
#include <mach-o/dyld.h>
#include <mach-o/loader.h>
#include <mach-o/nlist.h>
#include <mach/mach.h>
#include <mach/mach_vm.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#include "commun.h"

// Non déclarée dans le SDK : état de la signature de code du processus.
extern int csops(pid_t pid, unsigned int ops, void *useraddr, size_t usersize);

// Code écrit à la place du début de la fonction visée : elle rend alors 42.
#if defined(__arm64__)
static const uint32_t PATCH[2] = {0x52800540, 0xd65f03c0}; // mov w0, #42 ; ret
#else
static const uint8_t PATCH[6] = {0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3}; // mov eax, 42 ; ret
#endif

// Codes de sortie d'un fils.
enum { OK = 0, REFUSE = 2, SANS_EFFET = 3 };

static int refus(const char *etape, long code) {
    dprintf(1, "  détail : %s refusé (code %ld : %s)\n", etape, code, code > 0 && code < 107 ? strerror((int)code) : "retour Mach");
    return REFUSE;
}

// ---------------------------------------------------------------------------------------------
// État de la signature

static void etat_signature(const char *quand) {
    uint32_t f = 0;
    if (csops(getpid(), 0 /* CS_OPS_STATUS */, &f, sizeof f) != 0) {
        bge_res(quand, "csops refusé (errno %d)", errno);
        return;
    }
    bge_res(quand, "%#x%s%s%s%s%s", f, f & 0x1 ? " VALID" : " invalide", f & 0x100 ? " HARD" : "", f & 0x200 ? " KILL" : "",
            f & 0x1000 ? " ENFORCEMENT" : "", f & 0x10000 ? " RUNTIME" : "");
}

// ---------------------------------------------------------------------------------------------
// Lancement d'une tentative dans un fils

static int lancer(const char *essai, int (*f)(void *), void *arg) {
    pid_t p = fork();
    if (p < 0) {
        bge_res(essai, "fork impossible (errno %d)", errno);
        return -1;
    }
    if (p == 0) {
        _exit(f(arg));
    }
    int st = 0;
    if (waitpid(p, &st, 0) < 0) {
        bge_res(essai, "waitpid impossible (errno %d)", errno);
        return -1;
    }
    if (WIFSIGNALED(st)) {
        bge_res(essai, "processus tué (signal %d, %s)", WTERMSIG(st), strsignal(WTERMSIG(st)));
        return -1;
    }
    switch (WEXITSTATUS(st)) {
    case OK:
        bge_res(essai, "OK");
        return 0;
    case REFUSE:
        bge_res(essai, "refusé par le système");
        return REFUSE;
    case SANS_EFFET:
        bge_res(essai, "écrit, mais le code exécuté n'a pas changé");
        return SANS_EFFET;
    default:
        bge_res(essai, "sortie inattendue (%d)", WEXITSTATUS(st));
        return -1;
    }
}

// ---------------------------------------------------------------------------------------------
// 1. Mémoire exécutable pour les trampolines

// Bascule de la page MAP_JIT du fil courant entre écriture et exécution (Apple Silicon ; sans
// effet sur Intel).
static void jit_ecriture(int ecriture) {
    if (__builtin_available(macOS 11.0, *)) {
        pthread_jit_write_protect_np(!ecriture);
    }
}

static int enfant_jit(void *inutile) {
    (void)inutile;
    size_t ps = (size_t)getpagesize();
    void *p = mmap(NULL, ps, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANON | MAP_JIT, -1, 0);
    if (p == MAP_FAILED) {
        return refus("mmap MAP_JIT", errno);
    }
    jit_ecriture(1);
    memcpy(p, PATCH, sizeof PATCH);
    jit_ecriture(0);
    sys_icache_invalidate(p, sizeof PATCH);
    return ((int (*)(void))p)() == 42 ? OK : SANS_EFFET;
}

static int enfant_rw_rx(void *inutile) {
    (void)inutile;
    size_t ps = (size_t)getpagesize();
    void *p = mmap(NULL, ps, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
    if (p == MAP_FAILED) {
        return refus("mmap", errno);
    }
    memcpy(p, PATCH, sizeof PATCH);
    if (mprotect(p, ps, PROT_READ | PROT_EXEC) != 0) {
        return refus("mprotect RX", errno);
    }
    sys_icache_invalidate(p, sizeof PATCH);
    return ((int (*)(void))p)() == 42 ? OK : SANS_EFFET;
}

// ---------------------------------------------------------------------------------------------
// 3. Patch en place : les méthodes

// Pages couvertes par le patch : (début, longueur).
static void pages(void *fn, uintptr_t *debut, size_t *len) {
    uintptr_t ps = (uintptr_t)getpagesize();
    *debut = (uintptr_t)fn & ~(ps - 1);
    *len = (size_t)((((uintptr_t)fn + sizeof PATCH + ps - 1) & ~(ps - 1)) - *debut);
}

// mprotect en écriture, puis retour en exécution.
static int m_mprotect(void *fn) {
    uintptr_t debut;
    size_t len;
    pages(fn, &debut, &len);
    if (mprotect((void *)debut, len, PROT_READ | PROT_WRITE) != 0) {
        return refus("mprotect RW", errno);
    }
    memcpy(fn, PATCH, sizeof PATCH);
    if (mprotect((void *)debut, len, PROT_READ | PROT_EXEC) != 0) {
        return refus("mprotect RX", errno);
    }
    sys_icache_invalidate(fn, sizeof PATCH);
    return OK;
}

// mach_vm_protect avec VM_PROT_COPY : copie privée de la page du fichier, comme le font Frida
// et Dobby.
static int m_vm_protect_copy(void *fn) {
    uintptr_t debut;
    size_t len;
    pages(fn, &debut, &len);
    kern_return_t kr = mach_vm_protect(mach_task_self(), debut, len, FALSE, VM_PROT_READ | VM_PROT_WRITE | VM_PROT_COPY);
    if (kr != KERN_SUCCESS) {
        return refus("mach_vm_protect RW|COPY", kr + 1000);
    }
    memcpy(fn, PATCH, sizeof PATCH);
    kr = mach_vm_protect(mach_task_self(), debut, len, FALSE, VM_PROT_READ | VM_PROT_EXECUTE);
    if (kr != KERN_SUCCESS) {
        return refus("mach_vm_protect RX", kr + 1000);
    }
    sys_icache_invalidate(fn, sizeof PATCH);
    return OK;
}

// Page lisible, inscriptible et exécutable à la fois.
static int m_rwx(void *fn) {
    uintptr_t debut;
    size_t len;
    pages(fn, &debut, &len);
    if (mprotect((void *)debut, len, PROT_READ | PROT_WRITE | PROT_EXEC) != 0) {
        return refus("mprotect RWX", errno);
    }
    memcpy(fn, PATCH, sizeof PATCH);
    sys_icache_invalidate(fn, sizeof PATCH);
    return OK;
}

// Remplace les pages par une copie déjà patchée, posée par-dessus (`mach_vm_remap`).
static int remplacer_par(uintptr_t debut, size_t len, mach_vm_address_t copie) {
    mach_vm_address_t dest = debut;
    vm_prot_t cur = 0, max = 0;
    kern_return_t kr = mach_vm_remap(mach_task_self(), &dest, len, 0, VM_FLAGS_FIXED | VM_FLAGS_OVERWRITE, mach_task_self(), copie, TRUE,
                                     &cur, &max, VM_INHERIT_COPY);
    if (kr != KERN_SUCCESS) {
        return refus("mach_vm_remap", kr + 1000);
    }
    if (!(cur & VM_PROT_EXECUTE)) {
        return refus("mach_vm_remap (page non exécutable)", 1000);
    }
    return OK;
}

// Copie anonyme patchée, passée RX, puis posée sur l'original.
static int m_remap(void *fn) {
    uintptr_t debut;
    size_t len;
    pages(fn, &debut, &len);
    mach_vm_address_t copie = 0;
    kern_return_t kr = mach_vm_allocate(mach_task_self(), &copie, len, VM_FLAGS_ANYWHERE);
    if (kr != KERN_SUCCESS) {
        return refus("mach_vm_allocate", kr + 1000);
    }
    memcpy((void *)copie, (void *)debut, len);
    memcpy((void *)(copie + ((uintptr_t)fn - debut)), PATCH, sizeof PATCH);
    kr = mach_vm_protect(mach_task_self(), copie, len, FALSE, VM_PROT_READ | VM_PROT_EXECUTE);
    if (kr != KERN_SUCCESS) {
        return refus("mach_vm_protect RX de la copie", kr + 1000);
    }
    int r = remplacer_par(debut, len, copie);
    if (r == OK) {
        sys_icache_invalidate(fn, sizeof PATCH);
    }
    return r;
}

// Même chose avec une copie en mémoire MAP_JIT, que le droit `allow-jit` du jeu autorise.
static int m_remap_jit(void *fn) {
    uintptr_t debut;
    size_t len;
    pages(fn, &debut, &len);
    void *copie = mmap(NULL, len, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANON | MAP_JIT, -1, 0);
    if (copie == MAP_FAILED) {
        return refus("mmap MAP_JIT", errno);
    }
    jit_ecriture(1);
    memcpy(copie, (void *)debut, len);
    memcpy((char *)copie + ((uintptr_t)fn - debut), PATCH, sizeof PATCH);
    jit_ecriture(0);
    int r = remplacer_par(debut, len, (mach_vm_address_t)(uintptr_t)copie);
    if (r == OK) {
        sys_icache_invalidate(fn, sizeof PATCH);
    }
    return r;
}

static const struct {
    const char *nom;
    int (*m)(void *);
} METHODES[] = {
    {"mprotect RW puis RX", m_mprotect},
    {"mach_vm_protect avec VM_PROT_COPY", m_vm_protect_copy},
    {"mprotect RWX", m_rwx},
    {"copie anonyme posée par mach_vm_remap", m_remap},
    {"copie MAP_JIT posée par mach_vm_remap", m_remap_jit},
};
#define N_METHODES (sizeof METHODES / sizeof METHODES[0])

struct tentative {
    int (*m)(void *);
    void *fn;
};

static int enfant_patch(void *arg) {
    struct tentative *t = arg;
    int r = t->m(t->fn);
    if (r != OK) {
        return r;
    }
    return ((int (*)(void))t->fn)() == 42 ? OK : SANS_EFFET;
}

// Essaie chaque méthode dans un fils, puis applique ici la première qui a passé.
static void essais_patch(const char *quoi, void *fn) {
    char essai[160];
    int premiere = -1;
    for (size_t i = 0; i < N_METHODES; i++) {
        snprintf(essai, sizeof essai, "patch du code, %s : %s", quoi, METHODES[i].nom);
        struct tentative t = {METHODES[i].m, fn};
        if (lancer(essai, enfant_patch, &t) == 0 && premiere < 0) {
            premiere = (int)i;
        }
    }
    snprintf(essai, sizeof essai, "patch du code, %s : appliqué dans le processus", quoi);
    if (premiere < 0) {
        bge_res(essai, "non, aucune méthode ne passe");
    } else if (METHODES[premiere].m(fn) == OK) {
        bge_res(essai, "oui, par « %s »", METHODES[premiere].nom);
    } else {
        bge_res(essai, "échec inattendu de « %s » hors du fils", METHODES[premiere].nom);
    }
    snprintf(essai, sizeof essai, "état de la signature après le patch, %s", quoi);
    etat_signature(essai);
}

// ---------------------------------------------------------------------------------------------
// 2. Accroche par pointeur d'import

// Remplace, dans l'image `mh`, chaque pointeur d'import du symbole `nom` (`_dlopen`) par
// `remplacement`. Rend le nombre de pointeurs modifiés, ou -1 si les tables sont introuvables.
// Les pointeurs sont dans __DATA ou __DATA_CONST : des données, pas du code signé.
// Dernière case d'import remplacée, relue à la fin du processus (`fin`).
static void **case_dlopen;

static int accrocher_import(const struct mach_header_64 *mh, intptr_t slide, const char *nom, void *remplacement) {
    const struct symtab_command *st = NULL;
    const struct dysymtab_command *dt = NULL;
    const struct segment_command_64 *linkedit = NULL;
    const struct load_command *lc = (const struct load_command *)(mh + 1);
    for (uint32_t i = 0; i < mh->ncmds; i++) {
        if (lc->cmd == LC_SYMTAB) {
            st = (const struct symtab_command *)lc;
        } else if (lc->cmd == LC_DYSYMTAB) {
            dt = (const struct dysymtab_command *)lc;
        } else if (lc->cmd == LC_SEGMENT_64 && strcmp(((const struct segment_command_64 *)lc)->segname, "__LINKEDIT") == 0) {
            linkedit = (const struct segment_command_64 *)lc;
        }
        lc = (const struct load_command *)((const char *)lc + lc->cmdsize);
    }
    if (!st || !dt || !linkedit) {
        return -1;
    }
    uintptr_t base = (uintptr_t)slide + linkedit->vmaddr - linkedit->fileoff;
    const struct nlist_64 *syms = (const struct nlist_64 *)(base + st->symoff);
    const char *noms = (const char *)(base + st->stroff);
    const uint32_t *indirects = (const uint32_t *)(base + dt->indirectsymoff);
    uintptr_t ps = (uintptr_t)getpagesize();

    int n = 0;
    lc = (const struct load_command *)(mh + 1);
    for (uint32_t i = 0; i < mh->ncmds; i++) {
        if (lc->cmd == LC_SEGMENT_64) {
            const struct segment_command_64 *seg = (const struct segment_command_64 *)lc;
            const struct section_64 *sect = (const struct section_64 *)(seg + 1);
            for (uint32_t j = 0; j < seg->nsects; j++, sect++) {
                uint32_t type = sect->flags & SECTION_TYPE;
                if (type != S_LAZY_SYMBOL_POINTERS && type != S_NON_LAZY_SYMBOL_POINTERS) {
                    continue;
                }
                void **cases = (void **)((uintptr_t)slide + sect->addr);
                dprintf(1, "  détail : section de pointeurs %.16s,%.16s, %llu cases\n", sect->segname, sect->sectname,
                        (unsigned long long)(sect->size / sizeof(void *)));
                for (uint64_t k = 0; k < sect->size / sizeof(void *); k++) {
                    uint32_t ix = indirects[sect->reserved1 + k];
                    if (ix & (INDIRECT_SYMBOL_ABS | INDIRECT_SYMBOL_LOCAL) || ix >= st->nsyms) {
                        continue;
                    }
                    if (strcmp(noms + syms[ix].n_un.n_strx, nom) != 0) {
                        continue;
                    }
                    uintptr_t page = (uintptr_t)&cases[k] & ~(ps - 1);
                    if (mprotect((void *)page, ps, PROT_READ | PROT_WRITE) != 0 &&
                        mach_vm_protect(mach_task_self(), page, ps, FALSE, VM_PROT_READ | VM_PROT_WRITE | VM_PROT_COPY) != KERN_SUCCESS) {
                        dprintf(1, "  détail : section %.16s non inscriptible (errno %d)\n", sect->sectname, errno);
                        continue;
                    }
                    dprintf(1, "  détail : case %p, avant %p (dlopen du système : %p)\n", (void *)&cases[k], cases[k], (void *)dlopen);
                    cases[k] = remplacement;
                    case_dlopen = &cases[k];
                    dprintf(1, "  détail : import %s remplacé dans %.16s,%.16s\n", nom, sect->segname, sect->sectname);
                    n++;
                }
            }
        }
        lc = (const struct load_command *)((const char *)lc + lc->cmdsize);
    }
    return n;
}

// Le dlopen que l'exe appelle désormais. Celui de cette dylib ne passe pas par l'accroche : ses
// propres imports ne sont pas modifiés.
static void *mon_dlopen(const char *chemin, int mode) {
    void *h = dlopen(chemin, mode);
    const char *fichier = chemin ? (strrchr(chemin, '/') ? strrchr(chemin, '/') + 1 : chemin) : "(nul)";
    bge_res("accroche de l'import dlopen de l'exe", "appelée pour %s", fichier);
    if (h && strstr(fichier, "GameCore")) {
        void *fn = dlsym(h, "DllCreateGameContext");
        if (fn) {
            essais_patch("bundle GameCore", fn);
        } else {
            bge_res("patch du code, bundle GameCore", "DllCreateGameContext introuvable");
        }
    }
    return h;
}

// ---------------------------------------------------------------------------------------------

// À la fin du processus : la case d'import porte-t-elle toujours l'accroche ? Si dyld l'a
// réécrite après le constructeur, l'accroche est à poser plus tard ou autrement.
__attribute__((destructor)) static void fin(void) {
    if (case_dlopen && getenv("BGE_VARIANTE")) {
        bge_res("case de l'import dlopen à la fin du processus", "%s (%p)",
                *case_dlopen == (void *)mon_dlopen ? "porte toujours l'accroche" : "réécrite", *case_dlopen);
    }
}

__attribute__((constructor)) static void demarrage(void) {
    // Les fils de fork ne doivent pas refaire l'essai, et l'injection ne se propage pas plus loin.
    unsetenv("DYLD_INSERT_LIBRARIES");
    dprintf(1, "INJECTION : dylib chargée dans le processus %d\n", getpid());
    bge_res("injection par DYLD_INSERT_LIBRARIES", "OK");
    etat_signature("état de la signature au démarrage");

    lancer("mémoire exécutable : MAP_JIT", enfant_jit, NULL);
    lancer("mémoire exécutable : page RW passée RX", enfant_rw_rx, NULL);

    // L'exécutable principal se reconnaît à son type : l'image 0 n'est pas toujours lui (en arm64
    // sous macOS 26, c'est la dylib insérée, et l'accroche se posait sur nos propres imports).
    uint32_t exe = 0;
    for (uint32_t i = 0; i < _dyld_image_count(); i++) {
        const struct mach_header *h = _dyld_get_image_header(i);
        if (h && h->filetype == MH_EXECUTE) {
            exe = i;
            break;
        }
    }
    dprintf(1, "  détail : exécutable principal = image %u (%s)\n", exe, _dyld_get_image_name(exe));
    int n = accrocher_import((const struct mach_header_64 *)_dyld_get_image_header(exe), _dyld_get_image_vmaddr_slide(exe), "_dlopen", (void *)mon_dlopen);
    if (n > 0) {
        bge_res("accroche de l'import dlopen de l'exe", "posée (%d pointeur%s)", n, n > 1 ? "s" : "");
    } else {
        bge_res("accroche de l'import dlopen de l'exe", "%s", n == 0 ? "import introuvable ou non inscriptible" : "tables de symboles introuvables");
    }

    void *fn = dlsym(RTLD_MAIN_ONLY, "cible_hote");
    if (fn) {
        essais_patch("exe", fn);
    } else {
        bge_res("patch du code, exe", "cible_hote introuvable");
    }
}

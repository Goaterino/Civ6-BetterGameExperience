// Hôte de l'essai : tient le rôle de Civ6_Exe_Child.
//
// Comme le jeu, il a une fonction dans son propre __TEXT (`cible_hote`, le pendant de
// luaL_loadbuffer ou de la lecture XML), et il charge un bundle par dlopen puis y cherche
// `DllCreateGameContext` par dlsym. essai.sh le signe de plusieurs façons (voir README.md) et le
// lance avec DYLD_INSERT_LIBRARIES=bge.dylib.
//
// Usage : hote <chemin du bundle>

#include <dlfcn.h>
#include <unistd.h>

#include "commun.h"

// Lue à chaque appel : la fonction n'est ni repliée ni réduite à deux instructions.
volatile int compteur_hote = 0;

__attribute__((noinline, used, visibility("default"))) int cible_hote(void) {
    return compteur_hote + 1;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        dprintf(2, "usage : hote <bundle>\n");
        return 2;
    }
    int (*volatile f)(void) = cible_hote;
    dprintf(1, "hote : cible_hote() = %d à l'entrée de main\n", f());

    dprintf(1, "hote : appel de dlopen\n");
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) {
        dprintf(1, "hote : dlopen impossible : %s\n", dlerror());
        return 1;
    }
    int (*volatile g)(void) = (int (*)(void))dlsym(h, "DllCreateGameContext");
    if (!g) {
        dprintf(1, "hote : DllCreateGameContext introuvable\n");
        return 1;
    }
    int a = f(), b = g();
    bge_res("bout en bout : fonction de l'exe", "%s (rend %d)", a == 42 ? "détournée" : "intacte", a);
    bge_res("bout en bout : fonction du bundle GameCore", "%s (rend %d)", b == 42 ? "détournée" : "intacte", b);
    return 0;
}

// Commun à l'hôte et à la dylib injectée : étiquette d'architecture et lignes de résultat.
//
// Chaque constat sort sur une ligne `RESULTAT|variante|architecture|essai|verdict`, que essai.sh
// rassemble en tableau. Tout passe par dprintf : pas de tampon stdio à vider avant un fork.
#ifndef BGE_ESSAI_COMMUN_H
#define BGE_ESSAI_COMMUN_H

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/sysctl.h>
#include <sys/types.h>

#if defined(__arm64__)
#define BGE_ARCH "arm64"
#else
#define BGE_ARCH "x86_64"
#endif

// 1 si le processus est traduit par Rosetta (x86_64 sur Apple Silicon).
__attribute__((unused)) static int bge_rosetta(void) {
    int v = 0;
    size_t n = sizeof v;
    if (sysctlbyname("sysctl.proc_translated", &v, &n, NULL, 0) != 0) {
        return 0;
    }
    return v == 1;
}

__attribute__((unused, format(printf, 2, 3))) static void bge_res(const char *essai, const char *fmt, ...) {
    const char *variante = getenv("BGE_VARIANTE");
    char verdict[256];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(verdict, sizeof verdict, fmt, ap);
    va_end(ap);
    dprintf(1, "RESULTAT|%s|%s%s|%s|%s\n", variante ? variante : "?", BGE_ARCH, bge_rosetta() ? " (Rosetta)" : "", essai, verdict);
}

#endif

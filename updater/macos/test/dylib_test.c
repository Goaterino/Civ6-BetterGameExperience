/* Fausse dylib de BetterGameExperience, pour tester l'installeur macOS (voir README.md) : porte la
 * marque que l'installeur cherche (MARQUE), et note dans injection.log, à côté du dossier dylib,
 * qu'elle a été chargée et dans quel exécutable. */
#include <mach-o/dyld.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

__attribute__((used)) const char marque[] = MARQUE;

__attribute__((constructor)) static void chargee(void) {
    const char *dossier = getenv("BGE_DYLIBS");
    char exe[1024];
    uint32_t n = sizeof exe;
    if (_NSGetExecutablePath(exe, &n) != 0) strcpy(exe, "?");
    char journal[1200];
    snprintf(journal, sizeof journal, "%s/../injection.log", dossier ? dossier : "/tmp");
    FILE *f = fopen(journal, "a");
    if (f) {
        fprintf(f, "%s chargée dans %s (BGE_DYLIBS=%s)\n", NOM, exe, dossier ? dossier : "absent");
        fclose(f);
    }
}

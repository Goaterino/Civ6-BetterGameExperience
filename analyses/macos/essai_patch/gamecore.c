// Bundle de l'essai : tient le rôle de MacOS/GameCore_XP2.dll, un bundle Mach-O à deux exports
// que l'exe charge par dlopen. `DllCreateGameContext` est la fonction que la dylib injectée tente
// de patcher, le pendant d'un `Site` du proxy.

volatile int compteur_gamecore = 0;

__attribute__((noinline, used, visibility("default"))) int DllCreateGameContext(void) {
    return compteur_gamecore + 1;
}

__attribute__((noinline, used, visibility("default"))) void DllDestroyGameContext(void) {
    compteur_gamecore = 0;
}

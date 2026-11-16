/* Faux Civ6_Exe_Child : l'exécutable que Steam lance par %command%, pour tester l'injection. */
#include <stdio.h>

int main(int argc, char **argv) {
    printf("Faux Civilization VI lancé avec %d argument(s).\n", argc - 1);
    return 0;
}

# Test du salon multijoueur : fiche joueur

But : comprendre pourquoi des réglages de salon et des changements d'emplacement (swaps)
s'appliquent mal chez certains joueurs. Un petit outil (Frida) note ce que le jeu reçoit et fait. Il
**ne modifie rien** au jeu et ne touche à aucune partie.

Durée : 30 à 45 min. Il faut l'hôte et 2 ou 3 autres joueurs.

## Avant (5 min, une seule fois)

1. **Python 3.9 ou plus** doit être installé. Sinon, l'installer depuis python.org en cochant
   « Add python.exe to PATH ».
2. Dézipper `kit-test-salon.zip` dans un dossier.
3. Ouvrir PowerShell dans ce dossier et vérifier l'installation :

   ```
   powershell -ExecutionPolicy Bypass -File test_salon.ps1 -Nom TonPseudo -Verifier
   ```

   Il doit finir par `Python et frida 17.25.2 prêts.` Si Python n'est pas trouvé, ajouter
   `-Python "C:\chemin\vers\python.exe"`.
4. Mêmes mods que l'hôte : MPH, BSM, BBG. Si possible, la même langue de jeu, sauf pour le test 5.
5. Horloge Windows à l'heure : Paramètres > Heure > « Synchroniser maintenant ».
6. **Lancement en DX12.** Choisir « DX12 » dans le lanceur 2K ne suffit pas si les options de
   lancement Steam forcent l'exe, une astuce courante pour sauter le lanceur. Si Propriétés > Général
   > Options de lancement contient `…\CivilizationVI.exe" %command%`, le jeu démarre toujours en DX11.
   Pour le test, remplacer par :

   ```
   "C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI\Base\Binaries\Win64Steam\CivilizationVI_DX12.exe" %command%
   ```

   Adapter le début du chemin si le jeu est installé ailleurs. On peut remettre l'ancienne ligne
   après le test.

## Pendant

1. Lancer Civ6 en **DX12**, jusqu'au menu principal.
2. Lancer le script, **sans** `-Verifier` :

   ```
   powershell -ExecutionPolicy Bypass -File test_salon.ps1 -Nom TonPseudo
   ```

   Il doit écrire `CivilizationVI_DX12.exe accroché`. S'il écrit `octets inattendus`, prévenir l'hôte :
   c'est le mauvais exe (DX11) ou une autre version du jeu.
3. Laisser la fenêtre ouverte et suivre les tests annoncés par l'hôte. Quand l'hôte dit « photo »,
   appuyer sur **Entrée** dans la fenêtre du script.
4. À la fin :
   1. taper `fin` puis Entrée (pas Ctrl+C) ;
   2. **fermer Civ6 sans le relancer**, car il réécrit ses journaux au démarrage ;
   3. appuyer encore sur Entrée.

   L'archive `salon-TonPseudo-<date>.zip` est sur le Bureau : l'envoyer à l'hôte.

## Les tests (annoncés par l'hôte)

Noter l'heure de chaque essai. En cas de surprise, faire une capture d'écran de la liste des
joueurs.

| # | ce qu'on fait | qui |
|---|---|---|
| 1 | L'hôte choisit le preset MPH « CWC », puis ajoute 2 merveilles exclues. A rejoint. Photo partout, et tout le monde compare la liste des merveilles exclues. | hôte, A |
| 2 | A demande un emplacement IA ou ouvert **au moment où** B clique sur Rejoindre (l'hôte compte « 3, 2, 1 »). 10 essais : B quitte et revient entre chaque. Photo après chaque essai. | A, B |
| 3 | A demande l'emplacement de B (B ne répond pas), puis B quitte le salon. A dit s'il a été déplacé tout seul. | A, B |
| 4 | Au même « top » : l'hôte change l'équipe de A, et A change de civilisation. Photo partout. | hôte, A |
| 5 | Mode anonyme MPH, l'hôte en français et A en anglais. Ne rien toucher pendant 30 s. | hôte, A |
| 6 | Référence : régler un salon normal, tout le monde prêt, lancer. Photo juste avant le lancement. | tous |

Ce qui nous intéresse :
- des listes de joueurs différentes d'une machine à l'autre ;
- un joueur affiché au mauvais emplacement, ou un emplacement « pris » vide ;
- un « prêt » qui saute ;
- un swap qui se fait tout seul.

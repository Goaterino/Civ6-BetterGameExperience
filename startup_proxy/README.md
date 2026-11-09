# Proxy de démarrage : winmm.dll

Le proxy de GameCore ([proxy](../proxy/README.md)) n'est chargé qu'au lancement d'une partie
Gathering Storm. Ce second proxy est chargé au démarrage de l'exe, avant le menu principal. C'est
lui qui fait charger le proxy de GameCore à la place de l'original (voir
[Le chargement du proxy de GameCore](#le-chargement-du-proxy-de-gamecore)). Il ajoute à l'écran
d'options du jeu un onglet **BetterGameExperience**, qui édite les réglages du proxy de GameCore
(`rollover_proxy.cfg`). Dans le salon multijoueur, il annonce sa version aux autres joueurs, et
l'hôte établit si tous ont le proxy (voir [Le salon multijoueur](#le-salon-multijoueur--qui-a-le-proxy-)).
Au menu principal, il propose les nouvelles versions (voir
[La notification de mise à jour](#la-notification-de-mise-à-jour)). En partie, le menu Échap gagne
un bouton qui envoie un rapport de bug (voir [Le signalement d'un bug](#le-signalement-dun-bug)).

## Pourquoi winmm

Aucun fichier du dépôt Steam ne doit être remplacé : Steam remet les originaux à chaque
vérification des fichiers ou mise à jour du jeu. Il en traite une avant de télécharger un mod du
Workshop : rejoindre un salon qui demande un mod suffisait à désinstaller les proxys jusqu'à la
v0.3.0, qui remplaçaient `DatabaseDLL_FinalRelease.dll` et `GameCore_XP2_FinalRelease.dll`.
Steam ne supprime pas, en revanche, les fichiers qui ne sont pas dans son dépôt.

Les deux exe (DX11 et DX12) importent statiquement, depuis System32, des DLL de Windows qui ne sont
pas des KnownDLLs et que le jeu ne livre pas : `WINMM.dll`, `WINHTTP.dll`, `IPHLPAPI.DLL`,
`dbghelp.dll` et `VERSION.dll`. Le chargeur les cherche d'abord dans le dossier de l'exe.
- `VERSION.dll` est écartée : le moteur de compatibilité la charge depuis System32 avant l'exe (voir
  le README du proxy de GameCore, « Pistes écartées »).
- `winmm.dll` est chargée depuis `Win64Steam` par les deux exe, lancés directement ou par Steam :
  le moteur de compatibilité ne la précharge pas, et elle ne touche pas au graphisme.

Validé le 2026-11-05 avec un prototype qui ne faisait qu'écrire dans le journal : chargé par les
deux exe et par un lancement Steam, et toujours en place, même SHA-256, après une vérification de
l'intégrité des fichiers par Steam. La mise à jour du même jour (build 15296837 → 26407122) avait
remis les originaux des deux anciens proxys.

## Principe

- Le proxy est posé dans `Base\Binaries\Win64Steam` sous le nom `winmm.dll`. Les noms des fichiers
  posés sont dans [proxy/src/noms.rs](../proxy/src/noms.rs), communs aux proxys et à l'installeur.
- **Les relais.** Les 180 exports nommés de la winmm de Windows (liste dans
  [src/winmm_exports.txt](src/winmm_exports.txt)) et son ordinal 2, sans nom, sont des relais
  `jmp [rip+case]` générés par [build.rs](build.rs). Au début de `DllMain`, [src/relais.rs](src/relais.rs)
  charge la winmm de System32 par son chemin complet (`GetSystemDirectoryW` : le nom seul désignerait
  ce proxy, déjà chargé) et remplit les cases. Les modules qui importent winmm sont initialisés après
  elle. Un export absent de la winmm de cette version de Windows rend 0, et le journal le compte.
- Des renvois PE (*forwarders*) n'ont pas suffi : leur cible est un nom de module ou un chemin figé
  dans la DLL (`C:\Windows\System32\winmm`), faux si Windows est installé ailleurs.
- Au chargement, le proxy écrit `démarrage : proxy winmm chargé dans <exe>` puis
  `démarrage : exports relayés vers <winmm de System32>` dans le journal commun,
  `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log`.

## Le chargement du proxy de GameCore

[src/chargeur.rs](src/chargeur.rs). L'exe charge `GameCore_XP2_FinalRelease.dll` au lancement d'une
partie Gathering Storm ; son nom vient de `Expansion2.modinfo`. Il n'importe que `LoadLibraryW` et
`LoadLibraryA` pour charger des DLL.

- Dès `DllMain`, le proxy remplace ces deux entrées de la table d'imports de l'exe.
- Si le fichier demandé est `GameCore_XP2_FinalRelease.dll` (casse ignorée, avec ou sans `.dll`) et
  que `BGD_GameCore_XP2.dll` est posé dans le même dossier, c'est lui qui est chargé. Le proxy de
  GameCore charge ensuite l'original sous son vrai nom, par ses propres imports, qui ne passent pas
  par ce détour.
- Sans `BGD_GameCore_XP2.dll`, ou s'il ne se charge pas, l'original est chargé tel quel.
- Le journal le dit : `chargeur : <chemin> demandée, BGD_GameCore_XP2.dll chargé à sa place`.
- Les autres proxys cherchent celui de démarrage par son chemin complet, à côté de l'exe
  (`proxy/src/pol.rs`) : la winmm de System32 porte le même nom de module. Celui de démarrage cherche
  celui de GameCore sous le nom `BGD_GameCore_XP2.dll`.

## L'onglet BetterGameExperience

[src/options.rs](src/options.rs). L'écran d'options est le même contexte `Options.xml` /
`Options.lua` au menu principal et en jeu.

- **Les accroches.** Dès `DllMain`, le proxy remplace l'entrée `luaL_loadbuffer` de la table
  d'imports de l'exe et détourne sa lecture XML. Ce sont les accroches du proxy de GameCore, dans
  le module commun [proxy/src/exe.rs](../proxy/src/exe.rs), inclus par `#[path]`. L'entrée doit déjà
  viser HavokScript : sinon rien n'est posé, et le journal le dit.
- **Le XML.** Après `Options.xml`, on ajoute le bouton d'onglet
  ([src/options_proxy_tab.xml](src/options_proxy_tab.xml)) sous `AppTab`, et le panneau
  ([src/options_proxy_panel.xml](src/options_proxy_panel.xml)) avant `KeyBindings`.
- **Le Lua.** [src/options_proxy.lua](src/options_proxy.lua) est ajouté à la fin d'`Options.lua`,
  pour lire `m_tabs`. Il construit une liste déroulante par réglage, avec les instances du fichier
  (`PullDownParameterInstance`, `KeyBindingCategory` pour les titres). En tête, l'interrupteur
  général `actif` : sur « Désactivé », les autres listes sont grisées, mais leurs valeurs restent
  dans le fichier.
- **Les options avancées**, cachées par défaut comme les options vidéo avancées du jeu. Les
  sections dont le titre porte `avance = true` (performances du passage de tour, diagnostic) vont
  dans la pile `ProxyAdvancedStack`, qu'un bouton « Afficher les options avancées » montre ou cache.
  Restent visibles le confort et l'interface : général, interface, correctifs, mods d'interface
  intégrés, crédits. L'état du bouton est gardé d'une ouverture à l'autre de l'écran.
- **Réinitialiser BGE.** Un bouton rouge `ProxyResetButton`, ajouté au XML juste après
  `ConfirmButton`, n'apparaît que sur l'onglet BGE : `OnSelectTab` est enveloppé, et sur cet
  onglet le bouton se place à gauche et Confirmer se décale à droite, côte à côte ; ailleurs,
  Confirmer reste seul et centré. Le clic ouvre le pop-up du jeu (Oui / Non). Oui retire du
  fichier les lignes de toutes les clés de l'onglet (`Civ6Proxy_ReinitialiserReglages`), qui
  reprennent leur valeur par défaut ; commentaires et clés inconnues restent. Les choix en
  attente sont abandonnés. Le message « redémarrage nécessaire » suit si une valeur change et que
  le proxy de GameCore a déjà lu le fichier, ou qu'elle n'est lue qu'au lancement du jeu.
- **Les fonctions natives**, posées dans la table globale du contexte avant sa compilation :
  `Civ6Proxy_LireReglages()` rend le texte du fichier, 1 si le proxy de GameCore est installé, et 1
  s'il a déjà lu le fichier ; `Civ6Proxy_EcrireReglages(lignes)` fusionne des lignes `clé=valeur`
  dans le fichier, et rend aussi ce dernier indicateur, comme
  `Civ6Proxy_ReinitialiserReglages(cles)`, qui en retire les lignes de ces clés. Seules les
  lignes changées sont réécrites, et les doublons d'une clé changée sont retirés : `read_config`
  garde la dernière occurrence. HavokScript n'exporte ni `lua_pushlstring` ni `lua_pushboolean` :
  les chaînes passent par `lua_pushfstring("%s")`, les booléens par `lua_pushinteger`.
- **Redémarrage seulement s'il le faut.** Le proxy de GameCore ne lit le fichier qu'à son
  chargement, au lancement de la première partie. `gamecore_config_read` le demande à son export
  `RolloverProxyStatus` (bit 2, originale chargée), si le module est chargé.
  - Pas encore lu : **Confirmer** écrit le fichier sans autre message, et l'avertissement dit que
    les réglages prendront effet à la prochaine partie.
  - Déjà lu : **Confirmer** écrit le fichier puis lève `_PromptRestartApp`. Le jeu affiche son
    propre message « redémarrage nécessaire » (`LOC_OPTIONS_CHANGES_REQUIRE_APP_RESTART`), et
    l'avertissement le dit aussi.
  - **Annuler** n'écrit rien. Chaque ouverture relit le fichier et met l'avertissement à jour.
- **Langue.** Textes dans la langue du jeu, parmi les 11 de BBG ([textes/](../textes/README.md)).
  Le nom et l'aide d'un réglage sont les clés `LOC_BGD_OPT_<CLÉ>_NOM` et `LOC_BGD_OPT_<CLÉ>_AIDE` :
  le test `textes_de_l_onglet` vérifie qu'elles existent pour chaque réglage.
- **Un mod qui remplace `Options.xml`** : sans le bouton et le panneau, le bloc Lua ne fait rien.
- **Coexistence avec le proxy de GameCore.** L'accroche Lua s'enchaîne d'elle-même : il lit notre
  accroche comme fonction d'origine. Pour la lecture XML, il trouve notre `jmp [rip+0]` au lieu du
  prologue et le recopie dans son trampoline. À son déchargement, il remet notre saut.
- **Le journal.** `options : accroches posées`, puis à l'ouverture des options
  `options : …Options.xml : onglet des proxys ajouté` et `…Options.lua : …, compilation réussie`,
  et `options : rollover_proxy.cfg enregistré (clé=valeur, …)` à chaque confirmation.

**Limiteur d'images.** Dans l'onglet Vidéo du jeu, le limiteur ne propose que 20, 30 et 60 images/s
(`tickInterval_options` d'`Options.lua`). Le proxy y ajoute, avant de compiler le script :
- **les entrées** : 45, 75, 90 et 100 images/s ;
- **les valeurs** : 21, 12, 10 et 9 ms de `TickIntervalInMS` (`AppOptions.txt`).

L'exe fait `Sleep` du reste de l'intervalle, en millisecondes entières, avec un minuteur réglé à
1 ms. Une image dure donc l'intervalle plus 0,5 à 1,5 ms. D'où, comme les valeurs du jeu (49, 32,
16), 1000 / n − 1, arrondi. Mesuré à 75 FPS (12 ms) : médiane 13,4 ms, soit 75 images/s (moyenne
71, à cause des accrocs que le jeu a aussi sans limite).
Le choix prend effet tout de suite et s'enregistre dans `AppOptions.txt` comme les autres.

Pas de réglage dans l'onglet BetterGameExperience : rien ne change tant qu'on ne choisit pas l'une de ces
entrées. Si la liste du jeu change de forme, elle reste telle quelle (journal :
`liste du limiteur d'images introuvable`). Mesures : [rollover](../rollover/README.md).

## Le profil des zones de l'exe (`profil_zones`)

Réglage de diagnostic, désactivé par défaut (onglet BetterGameExperience, section Diagnostic, ou
`proxy/install.ps1 -ProfilZones on`). Il prend effet au lancement suivant du jeu. Il n'existe que
dans les builds locaux : la release est compilée sans la feature `diag`
(voir [release/README.md](../release/README.md#diagnostic)). Code :
[src/zones.rs](src/zones.rs).

- **Ce que l'exe chronomètre.** Environ 1 170 zones nommées : image, interface, contextes Lua,
  événements de GameCore… Chaque zone est un objet de portée, ouvert par `140998450` (DX12) et fermé
  par `140998480` (`14098a7b0` en DX11). La fermeture n'écrit dans `Profile.csv` que les zones plus
  longues que leur seuil, et 500 fois au plus par zone.
- **Ce que fait le proxy.** Il détourne la fermeture dès DllMain, avant que l'exe exécute son code. Il
  la retrouve par une signature de 24 octets, identique dans les deux exes. Chaque zone est notée dans
  un tampon circulaire de 65 536 entrées : thread, nom (47 octets au plus), début et fin en QPC.
- **Le fichier.** Un thread d'écriture vide le tampon toutes les 100 ms dans
  `%LOCALAPPDATA%\Civ6RolloverProxy\zones\zones-<pid>-<horodatage>.bin`, environ 1 Mo par seconde.
  Le format est décrit en tête de `zones.rs`. Les zones perdues parce que le tampon a débordé sont
  comptées dans le fichier.
- **L'analyse.** `rollover/capture.ps1` copie le fichier de la session dans la capture, et
  `rollover/analyze.py` (avec [rollover/zones.py](../rollover/zones.py)) en tire le temps propre de
  chaque zone. Les zones et la trace WPR partagent la même horloge, QPC : `analyze.py` attribue
  aussi les échantillons CPU à la zone ouverte sur leur thread à cet instant.
- **Le journal.** `zones : détour posé en exe+0x…`, puis `zones : écriture dans …`.

**Toute nouvelle fonctionnalité des proxys doit avoir son réglage dans cet onglet.** Le test
`cles_communes_avec_le_proxy` échoue si une clé lue par `read_config` (proxy/src/lib.rs) manque à
`PROXY_REGLAGES` dans `options_proxy.lua`, ou l'inverse. Le libellé, l'aide et les choix se
donnent en français et en anglais.

## Le salon multijoueur : qui a le proxy ?

[src/salon.rs](src/salon.rs) et [src/salon.lua](src/salon.lua). Certains correctifs changent la
simulation : chaque machine recalcule la partie (lockstep), donc un joueur sans le proxy, ou d'une
autre version, se désynchroniserait. Ce sont les **correctifs POL** (« proxy-only lobbies ») : ils
ne s'activent que si tous les humains ont BetterGameExperience à la même version. Les autres correctifs
(interface, performances sans effet sur la partie) n'en dépendent pas.

- **La version** est dans [proxy/src/version.rs](../proxy/src/version.rs), commune aux deux
  proxys : l'augmenter à chaque version qui change la simulation ou cet échange. Elle n'est pas
  réglable, et seule elle est échangée : les correctifs POL n'ont pas de réglage.
- **L'accroche.** La même accroche `luaL_loadbuffer` que l'onglet ajoute `salon.lua` à la fin de
  `StagingRoom.lua`, ou de son remplaçant du même nom (celui de MPH, par exemple), après avoir posé
  `Civ6Proxy_Version()`, `Civ6Proxy_Jeton()`, `Civ6Proxy_Journal(texte)` et
  `Civ6Proxy_FixerPol(actif, raison)`. Le bloc ne s'appuie que sur les événements du jeu et sur
  `StartLaunchCountdown`, présents dans les deux versions. Il ne fait rien hors de la période où le
  salon est ouvert : de l'ouverture au chargement de la partie (`LoadScreenContentReady`) ou au
  départ (`LeaveGameComplete`). Le contexte reste chargé en partie et y reçoit encore les
  événements ; `ContextPtr:IsHidden()` ne suffit pas, le salon s'affichant en différé
  (`QueuePopup`).
- **Qui s'annonce.** Seule une machine qui appliquera les correctifs POL. `Civ6Proxy_Version()` rend
  nil et la raison si le proxy de GameCore n'est pas installé, si `actif=off`, ou, une fois chargé,
  s'il n'a pas posé ses correctifs POL (bit 16 de `RolloverProxyStatus`). Elle ne s'annonce pas, et
  compte donc comme sans BetterGameExperience.
- **Les jetons.** Les configurations de joueur sont enregistrées avec la partie : une sauvegarde
  rechargée rapporte les annonces et le verdict d'une session précédente, peut-être chez des
  joueurs qui n'ont plus le proxy. Chaque ouverture du salon, et chaque migration d'hôte, tire donc
  un jeton pour l'hôte et un pour chaque joueur.
- **L'annonce.** Chaque joueur écrit dans sa configuration de joueur `BGD_VERSION =
  <version>@<jeton de l'hôte>#<son jeton>`, puis la diffuse (`Network.BroadcastPlayerInfo`, sans
  effet sur le statut « prêt »). Une annonce qui ne porte pas le jeton actuel de l'hôte ne compte
  pas.
- **Le verdict** est celui de l'hôte, le même pour toutes les machines : `BGD_VERDICT =
  <version>@<jeton de l'hôte>:<1|0>:<jetons>` dans sa configuration de joueur. 1 si tous les
  humains connectés annoncent sa version avec son jeton ; les jetons de tous ceux dont il a reçu une
  annonce de ce salon, quelle que soit leur version. Un client ne croit le verdict que s'il y trouve
  son propre jeton du moment : un verdict sorti d'une sauvegarde, chez un hôte qui n'a plus le
  proxy, ne peut pas le contenir. Les IA et les joueurs déconnectés ne comptent pas. Hôte sans
  proxy : pas de verdict valable, donc pas de correctifs POL. Ni la configuration de partie (sa
  diffusion remet tout le monde « pas prêt »), ni le hotseat, ni PlayByCloud (pas de salon commun)
  ne servent.
- **Les messages.** Au démarrage du compte à rebours de lancement, dans le chat, pour tous, une fois
  par contenu, en anglais ; chaque machine équipée les affiche dans sa langue (`BGD_TxtChat`, voir
  [textes/](../textes/README.md)). Leur sens, en français :
  - l'hôte, en rouge : `BGE - Sans BetterGameExperience : Bob. BetterGameExperience d'une autre version que
    0.2.0 : Zoé (0.1.0). Correctifs POL désactivés pour cette partie.` ;
  - l'hôte, en vert : `BGE - Tous les joueurs ont BetterGameExperience 0.2.0 : correctifs POL actifs.` ;
  - si l'hôte n'a pas reçu notre annonce, le premier client équipé, par ordre d'identifiant, en
    rouge : `BGE - L'hôte n'a pas BetterGameExperience, ou pas une version compatible : correctifs POL
    désactivés pour cette partie.` Équipé : son annonce a changé depuis l'ouverture du salon, ce
    qu'une annonce restée d'une sauvegarde ne fait pas. Un client arrivé après l'annonce d'un autre
    ne la voit pas changer : les deux peuvent alors écrire le même message, mais il n'en manque
    jamais.
- **L'état lu par le proxy de GameCore.** `BgdPolActifs`, export de ce proxy, rend 1 si les
  correctifs POL sont actifs pour la partie en cours. L'interface le tient à jour par
  `Civ6Proxy_FixerPol` :
  - le salon le met à 0 à chaque ouverture, puis à sa conclusion du verdict après chaque annonce ;
  - au chargement d'une partie qui n'est pas en réseau (solo, hotseat), `chargement.lua`, ajouté à
    la fin de `LoadScreen.lua`, le met à 1 : une seule machine, rien à désynchroniser ;
  - en réseau, rien d'autre ne le change jusqu'au prochain salon.
- **En cours de partie : reconnexions.** Un joueur qui rejoint ou se reconnecte repasse par le salon
  et tire un nouveau jeton, que l'hôte, en jeu, ne verrait pas. `partie.lua`, ajouté à la fin de
  `InGame.lua` (ou de son remplaçant, celui de MPH), le relaie : quand un humain connecté annonce la
  version et le jeton de l'hôte, celui-ci ajoute son jeton au verdict, sans en changer l'état. Le
  nouveau venu conclut comme les autres. Si les correctifs POL sont actifs et qu'un humain se met
  prêt sans s'être annoncé, l'hôte écrit en rouge : `BGE - Léo : arrivée sans BetterGameExperience (ou
  d'une autre version) alors que les correctifs POL sont actifs. Sa partie va se désynchroniser.`
- **Le journal** : `salon : bloc chargé`, `salon : pas d'annonce : <raison>`,
  `pol : correctifs POL actifs|désactivés (<raison>)` à chaque changement de l'état,
  `partie : verdict relayé : …`, `salon : annonce …, verdict …`,
  `salon : message : …`, `salon : lancement : correctifs POL actifs|désactivés` (la conclusion de
  chaque machine), et toute erreur du bloc, qui n'est jamais propagée au salon.
- **Correctif : cités-États d'une partie chargée.** Le salon de MPH affiche tout emplacement sous
  le maximum de joueurs de la carte (plafonné à 50, contre 12 dans le jeu de base), et ce maximum
  n'est pas recalculé au chargement d'une partie. Les cités-États reçoivent alors un menu de
  dirigeant. Leur dirigeant (`LEADER_MINOR_CIV_*`) n'est pas dans la liste et ne peut plus être
  changé : erreur `MissingDomainValue`, « Opt. joueur invalides. », et Prêt grisé pour tous
  jusqu'à ce qu'un changement d'option reconstruise la liste. Le bloc enveloppe
  `GetPlayerParameterError` et ignore cette erreur-là pour tout joueur qui n'est pas une
  civilisation majeure (`salon : joueur N (pas une civilisation majeure) : dirigeant hors liste
  ignoré`). Un diagnostic note dans le journal chaque apparition et levée d'une erreur de
  paramètre (`salon : diag : …`, avec le joueur, son dirigeant, la liste et l'état de la partie).

**Les correctifs POL** à ce jour : le choix des cases travaillées
([citoyens/README.md](../citoyens/README.md)).

**Pas encore fait :**
- un joueur sans le proxy qui arrive en cours de partie alors que les correctifs POL sont actifs se
  désynchronise ; l'hôte le dit seulement. Les couper pour tous demande de le faire au même point
  de la simulation sur chaque machine, par une commande du jeu ;
- après une migration d'hôte en cours de partie, le nouvel hôte n'a pas le verdict : un joueur qui
  se reconnecte ensuite conclut « désactivés » alors que les autres les appliquent ;
- un joueur qui rejoint en cours de partie doit recevoir le verdict relayé avant de quitter le salon
  pour le chargement.

**Risque connu :** au premier salon d'une session, le proxy de GameCore n'est pas encore chargé. La
machine s'annonce si le fichier de réglages le permet ; si, au lancement, le détour ne peut pas être
posé (jeu mis à jour sans le proxy), elle n'appliquera pas les correctifs que les autres
appliquent. Le journal le dit (`citoyens : version inattendue`).

## La notification de mise à jour

[src/maj.rs](src/maj.rs) et [src/maj.lua](src/maj.lua).

- **La vérification.** Au premier script Lua chargé par l'exe, bien avant le menu principal, un
  thread lit `releases/latest/download/manifest.json` (le `curl.exe` de Windows, sans fenêtre, 20 s
  au plus) et compare sa `version` à celle gravée dans ces DLL par le workflow de release
  (`BGD_RELEASE`, `RELEASE` dans [proxy/src/version.rs](../proxy/src/version.rs)). Une compilation
  locale n'a pas de version de release : elle ne vérifie rien. Le manifeste n'est pas vérifié ici :
  il ne sert qu'à prévenir, et l'installeur vérifie sa signature avant d'installer.
- **Le canal.** Elle suit la case « Pré-release » de l'installeur (`%LOCALAPPDATA%\BetterGameExperience\canal.txt`).
  Cochée, elle lit à la place la liste des releases de l'API GitHub et prend la release publiée au
  plus grand numéro `vX.Y.Z`, pré-releases comprises. Le popup dit si la version proposée est une
  release ou une pré-release (titre et texte), et rappelle pour une pré-release que c'est la case de
  l'installeur qui la fait recevoir. Sans installeur, le bouton ouvre la page de cette version.
- **Le réglage.** `verif_maj=on|off` (on par défaut), dans l'onglet BetterGameExperience, jamais grisé
  par l'interrupteur général, qui ne concerne que le proxy de GameCore. Lu au lancement du jeu.
- **L'accroche.** La même accroche `luaL_loadbuffer` ajoute `maj.lua` à la fin de `MainMenu.lua`,
  après avoir posé `Civ6Proxy_MiseAJour([action])` et `Civ6Proxy_Journal(texte)`. Le bloc enveloppe
  le gestionnaire d'affichage du menu, et, tant que la notification reste à montrer, sa mise à jour
  par image : il attend la fin de la vérification, et la fermeture d'un autre message du menu.
- **Un menu remplacé par un mod.** Le `MainMenu` de MPH n'a ni `g_PopupDialog` ni `OnUpdate` : son
  XML inclut le modèle `PopupDialog` sans l'instancier. L'accroche XML ajoute alors
  `<MakeInstance Name="PopupDialog" />` à la fin de `MainMenu.xml` (et le modèle s'il manque), comme
  le XML du jeu, et le bloc crée son popup sur ces contrôles (`include("PopupDialog")`). Sans ces
  contrôles, l'erreur va au journal.
- **Le message**, dans le popup du menu (`g_PopupDialog`, ou le sien), dans la langue du jeu
  ([textes/](../textes/README.md)) : « La version 0.1.2 de BetterGameExperience est disponible (installée : 0.1.1) ».
  - **Mettre à jour** lance l'installeur, `%LOCALAPPDATA%\BetterGameExperience\BetterGameExperience.exe`
    (il s'y copie à chaque installation), avec `--after-game <dossier de l'exe>`, puis ferme le jeu
    (`Events.UserConfirmedClose()`, comme « Quitter vers le bureau » ; `UI.ExitGame()` ne ferme le jeu qu'en partie). L'installeur attend la fin du jeu, installe, et propose de le relancer.
    Sans installeur à cet endroit, la page de la release s'ouvre dans le navigateur, et le jeu reste
    ouvert.
  - **Plus tard** (ou Échap) : plus de notification jusqu'au prochain lancement du jeu.
- **Le journal** : `maj : version … disponible (installée : …)`, `maj : à jour …`,
  `maj : vérification impossible : …`, `maj : notification de la version …`, `maj : installeur
  lancé (…)`, et toute erreur du bloc, qui n'est jamais propagée au menu.

## Le signalement d'un bug

[src/rapport.rs](src/rapport.rs) et [src/rapport.lua](src/rapport.lua). Le relais qui reçoit les
rapports et les publie sur Discord : [rapport_worker](../rapport_worker/README.md).

- **Le bouton.** « Signaler un bug (BGE) », dans la liste principale du menu Échap, juste sous
  « Options ». Après la lecture d'`InGameTopOptionsMenu.xml`, l'accroche XML recopie l'élément du
  bouton `OptionsButton` juste après lui, sous l'ID `BGD_RapportButton` : même style, même taille,
  même pile. La copie perd son texte et son infobulle (le bloc pose les siens), et les ID de ses
  enfants sont préfixés. Sans `OptionsButton` (autre version du menu), rien n'est ajouté, et le
  journal liste les boutons trouvés. À la fin du contexte s'ajoutent le formulaire du rapport
  ([src/rapport_formulaire.xml](src/rapport_formulaire.xml) : explication, consigne, champ de texte
  `EditBox` dans un cadre `EditTextArea`, comme le nom LAN de l'écran d'options, Envoyer et
  Annuler), puis l'instance `PopupDialog` si le menu ne l'a pas.
- **L'accroche Lua.** `rapport.lua` est ajouté à la fin d'`InGameTopOptionsMenu.lua`, que le
  remplaçant de MPH inclut (`ingametopoptionsmenu_MPH.lua`), après avoir posé
  `Civ6Proxy_RapportEnvoyer(lignes)`, `Civ6Proxy_RapportEtat()` et `Civ6Proxy_Journal(texte)`.
- **En multijoueur**, à chaque étape, un message `BGE -` part dans le chat de tous, en anglais, et
  s'affiche dans sa langue chez chaque joueur qui a BetterGameExperience (`BGD_TxtChat`), dans les
  couleurs atténuées du jeu (`COLOR_Civ6Green`, `COLOR_Civ6Red`, propres à ce module) :
  1. Clic : si la partie n'est pas déjà en pause, le joueur demande la pause comme MPH
     (`SetWantsPause(true)`, `BroadcastPlayerInfo`). « X signale un bug, merci de patienter. »
     Le formulaire s'ouvre : ce qui sera envoyé, un champ pour décrire le problème (1 000
     caractères au plus, envoyé dans `meta.json`), **Envoyer le rapport** et **Annuler**. Menu
     fermé puis rouvert, le formulaire revient tel quel, texte compris.
  2. **Annuler** (ou Échap) : « X a annulé le rapport de bug. La pause peut être levée. »
  3. **Envoyer le rapport** : « X : envoi du rapport en cours... ». Le bloc demande une sauvegarde
     `BGE_rapport` (comme la sauvegarde rapide), et le proxy lance l'envoi dans un thread.
  4. Fin de l'envoi : « X : envoi du rapport terminé. La pause peut être levée. », ou, en échec,
     « X : le rapport n'a pas pu être envoyé, il est gardé sur son ordinateur. La pause peut être
     levée. »

  Le bloc ne lève jamais la pause lui-même. En solo, ni pause ni message : le rapport part seul.
- **Le suivi.** Un popup dit que l'envoi est en cours ; on peut le fermer, et fermer le menu,
  l'envoi continue. Le menu interroge `Civ6Proxy_RapportEtat()` à chaque image tant qu'il est
  affiché. Fermé, il n'a plus d'images, et la partie en pause n'a plus d'événements : le suivi passe
  par le contexte `InGame`, toujours affiché. `InGame.xml` reçoit un minuteur
  (`BGD_RapportMinuteur`, une `SlideAnim` d'une demi-seconde, comme le minuteur de la minimap du
  jeu) et `InGame.lua` le bloc [src/rapport_suivi.lua](src/rapport_suivi.lua), lancé par
  `LuaEvents.BGD_RapportSuivre`, aidé hors pause par `GameCoreEventPublishComplete`.
- **La fin** ([src/rapport_fin.lua](src/rapport_fin.lua), `BGD_RapportTerminer`), par le suivi
  d'InGame, ou par le menu s'il n'y a pas de suivi : le message à tous, puis une ligne du chat que
  seul le joueur local voit (`LuaEvents.BGD_DepartTexte`, le canal des lignes locales du proxy de
  GameCore) : le numéro du rapport (`BGE-7F3A2C`), ou le chemin du zip en cas d'échec, et le contact
  (@goaterino en message privé Discord). Pas de popup : celui du menu ne se voyait qu'en le
  rouvrant. Le menu remet son état à zéro et ferme le popup « envoi en cours »
  (`LuaEvents.BGD_RapportFini`).
- **Le zip**, préparé dans le thread : il attend la sauvegarde `BGE_rapport.Civ6Save` (20 s au
  plus, taille stable), puis réunit :
  - `meta.json` : ce que dit le bloc (tour, pseudo, Steam ID, civilisation, dirigeant,
    multijoueur, hôte, pause, humains, langue, description), la version de BGE, l'exe (DX11 ou
    DX12), le compte Steam connecté sur la machine (`steam_id_windows`, lu dans
    `HKCU\Software\Valve\Steam\ActiveProcess`, utile en solo où le jeu ne donne pas toujours le
    Steam ID) et la liste des fichiers ;
  - `bge/RolloverProxy.log` et `bge/rollover_proxy.cfg` ;
  - `logs/` : le dossier `Logs` du jeu (`%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI`),
    `AppOptions.txt` et `Mods.sqlite` ;
  - `saves/` : la sauvegarde du rapport et les 3 derniers autosaves (multijoueur, ou solo et
    hotseat), dans `Documents\My Games\…\Saves` (dossier Documents de Windows, OneDrive compris).

  Les fichiers texte sont coupés à leurs 5 derniers Mo, et le nom d'utilisateur Windows est retiré
  des chemins (`C:\Users\<utilisateur>`). Au-delà de 40 Mo, les autosaves sont retirés, du plus
  ancien au plus récent. Compression deflate ([miniz_oxide](https://crates.io/crates/miniz_oxide)).
  Les 5 derniers zips restent dans `%LOCALAPPDATA%\BetterGameExperience\rapports`.
- **L'envoi** : `curl.exe` de Windows, sans fenêtre, 300 s au plus, champ `archive` d'un
  formulaire multipart, vers l'URL du relais gravée à la compilation (`BGE_RAPPORT_URL`, variable
  du dépôt GitHub lue par le workflow de release). `rapport_url=https://…` dans
  `rollover_proxy.cfg` la remplace, pour essayer un autre relais. L'en-tête `X-BGE-Cle` porte la
  clé que le relais exige (`CLE_CLIENT`, la même dans le Worker, un test le vérifie) : elle écarte
  les envois qui ne viennent pas de la DLL. Le relais rend `{"id": "BGE-…"}`.
  Sans relais (compilation locale), ou s'il échoue, le zip est copié sur le Bureau.
- **Le journal** : `rapport : formulaire ouvert`, `pause demandée`, `message : …`, `envoi lancé
  (…)`, `<zip> écrit (… octets)`, `envoyé, numéro …` ou `envoi impossible : …`, et toute erreur du
  bloc, qui n'est jamais propagée au menu.

## Compiler, tester, installer

```bash
cd startup_proxy && cargo build --release
```

**Tests hors jeu.**
- `cargo test --release` : le patch du vrai `Options.xml`, la fusion du fichier de réglages, la
  concordance des clés avec le proxy de GameCore.
- Le bloc Lua, chargé à la suite du vrai `Options.lua` : onglet, listes, valeurs du fichier,
  enregistrement à la confirmation, message de redémarrage, réouverture.

  ```bash
  lua startup_proxy/tests/test_options_proxy.lua
  ```
- Le bloc du salon, sur plusieurs machines simulées qui ont chacune leur copie des configurations
  de joueur : tous équipés, joueur sans proxy, annonce restée d'une sauvegarde, autre version, IA
  et absents, hôte sans proxy (un client prévient), hotseat, migration d'hôte, arrivée tardive,
  verdict périmé rediffusé par un hôte qui n'a plus le proxy, machine qui ne s'annonce pas,
  reconnexion en cours de partie (relais de l'hôte), arrivée sans proxy en cours de partie,
  migration d'hôte en partie, écran de chargement en solo et en réseau.

  ```bash
  lua startup_proxy/tests/test_salon.lua
  ```
- Le bloc de la notification, dans un menu principal simulé : vérification en cours, nouvelle
  version, à jour, autre popup ouvert, Mettre à jour, Plus tard, installeur absent, anglais, erreur.

  ```bash
  lua startup_proxy/tests/test_maj.lua
  ```
- Le bloc de l'option de partie « No First/Last moves (BGE) » (`option_partie.lua`, à la fin de
  `SetupParameters.lua`) : ligne ajoutée à `Parameters` avant chaque lecture des options, sans
  doublon, cache vidé, libellés français et
  anglais, écriture refusée sans casser l'écran. Voir
  [depart_tour/README.md](../depart_tour/README.md).

  ```bash
  lua startup_proxy/tests/test_option_partie.lua
  ```
- Le signalement d'un bug, dans un menu Échap et un contexte InGame simulés qui partagent leurs
  `LuaEvents` : pause demandée ou déjà en place, messages du chat à chaque étape (couleurs,
  traduction à l'arrivée), formulaire, description et Steam ID envoyés, annulation, sauvegarde,
  fin (message à tous et ligne locale) menu ouvert, menu fermé et partie en pause, sans minuteur,
  sans le bloc du suivi, échec, refus, solo, natives, bouton ou formulaire absents.

  ```bash
  lua startup_proxy/tests/test_rapport.lua
  ```
- La DLL : elle se charge, chaque export aboutit à la même adresse que dans la winmm de System32
  (ordinal 2 compris), `timeGetTime` répond par le relais, les lignes de journal sont écrites, et
  aucun détour n'est posé hors du jeu.

  ```bash
  .venv/Scripts/python.exe -I startup_proxy/tests/check_relais.py startup_proxy/target/release/winmm.dll <dossier temporaire>
  ```

**Installation**, jeu fermé. Le script remet d'abord en place une installation de la v0.3.0 ou
d'avant (l'original sous `DatabaseDLL_FinalRelease_orig.dll`), puis pose `winmm.dll`. Il refuse
d'écraser une `winmm.dll` qui n'est pas la sienne (un autre mod). Une vérification des fichiers ou
une mise à jour du jeu par Steam ne la retire pas.

```bash
powershell -ExecutionPolicy Bypass -File startup_proxy/install.ps1
```

```bash
powershell -ExecutionPolicy Bypass -File startup_proxy/install.ps1 -Uninstall
```

## Résultat (31 octobre 2026)

Lancements en DX12 puis en DX11 jusqu'au menu principal : le journal contient
`démarrage : proxy DatabaseDLL chargé dans …\Win64Steam\CivilizationVI_DX12.exe`, puis la même
ligne avec `CivilizationVI.exe`. Le jeu démarre normalement dans les deux cas.

Onglet, en DX12 : il apparaît au menu principal et en jeu, avec les valeurs du fichier. Trois
réglages changés et confirmés sont écrits (`portee`, `ameliorations`, `rendements`), et le jeu
affiche son message de redémarrage. En partie, les boutons d'attaque restent au-dessus des
drapeaux : les deux détours de la lecture XML s'enchaînent.

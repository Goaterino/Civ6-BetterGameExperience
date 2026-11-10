# Journal des modifications

[English](../CHANGELOG.md) · autres langues : voir le [journal anglais](../CHANGELOG.md).

## BetterGameExperience 0.8.2

### Nouveautés
- **Avancement de la recherche et du dogme** : dans le panneau en haut à gauche, à côté du nom de la
  technologie et du dogme en cours, **x/y** indique la science (la culture) déjà investie et le coût
  total, suivi de **(+z)** quand le jeu garde de la science (de la culture) en excédent d'une
  technologie (d'un dogme) terminée avec plus que son coût. Un réglage permet de le désactiver.

## BetterGameExperience 0.8.1

### Nouveautés
- **Annuler la production depuis le bandeau de ville** : un clic droit sur l'icône de production du
  bandeau d'une de vos villes retire l'élément en cours, comme dans le panneau de production.
- **Quartiers en réduction** : dans la liste de production, un quartier dont le coût est réduit
  (moins de quartiers de ce type que votre moyenne) affiche sa réduction, par exemple **-40%**. Elle
  se met à jour dès qu'un quartier est posé, puisqu'en poser un peut faire perdre la réduction. Les
  quartiers qui ne comptent pas dans la moyenne, comme le Thành du Viêt Nam, sont aussi indiqués.
- **Production en réserve** : le bandeau de la ville, le panneau de la ville et le panneau de
  production affichent la production que la ville garde en réserve (débordement d'un élément
  terminé, ou tour passé sans rien produire ; le jeu en garde au plus un tour), ajoutée à sa
  production du tour suivant. Sur le bandeau d'une ville sans production, elle s'affiche en **+N**.
- **BGE Check** : dans un salon multijoueur, l'hôte a un bouton **BGE CHECK**, à côté du MOD CHECK
  de MPH, qui demande à chaque joueur sa version de BetterGameExperience et écrit le bilan dans le
  chat : même version, autre version, installé mais désactivé, ou sans réponse.
- **Message d'arrivée** : à l'ouverture d'un salon avec BetterGameExperience activé, le chat
  l'annonce, avec votre version exacte.

### Corrections de l'interface
- **Panneau de la ville après le retrait de la production** : le panneau en bas à droite se met à
  jour quand on retire le seul élément de la file, au lieu de l'afficher encore.
- **Clic droit sur la production en cours** : il la retire aussi depuis l'onglet Production, plus
  seulement file ouverte.
- **Salon de MPH** : hors draft, le tableau des joueurs remonte dans l'espace vide que MPH réserve
  à son draft, et le « . » affiché au-dessus dans une partie rechargée disparaît.

### Réglages
- Les trois nouveautés ont chacune un réglage dans l'onglet BetterGameExperience, activé par défaut.

## BetterGameExperience 0.8.0

En multijoueur, tous les humains doivent être en 0.8.0. Le correctif des accords en fin de tour ne
protège que ceux qui l'ont : un seul joueur resté en 0.7.x peut ramener le tour bloqué pour tout le
monde. Au salon, l'hôte voit en rouge qui a une autre version, et un salon qui mélange 0.7.x et
0.8.0 désactive les correctifs **[tous les joueurs]**.

### Correctifs du multijoueur [tous les joueurs]
- **Tour qui tourne en boucle après un accord entre humains** : un accord proposé ou accepté juste
  à la fin de votre tour pouvait arriver chez l'hôte après la fin du tour. Le tour suivant se
  relançait alors sans fin chez certains joueurs, il fallait revenir au salon, et la partie se
  désynchronisait souvent. Un tel accord ne part plus une fois votre minuteur écoulé, ni quand plus
  aucun humain n'a son tour actif : une ligne dans votre chat vous le dit. Les accords avec les IA ne
  changent pas.

### Nouveautés
- Bouton **Signaler un bug (BGE)** dans le menu Échap, sous Options. Décrivez le problème : BGE
  envoie une sauvegarde, les derniers autosaves et les journaux du jeu (sans votre nom
  d'utilisateur Windows), et une ligne de votre chat donne le numéro du rapport. Les autres joueurs
  voient dans le chat qu'un rapport est en cours d'envoi. Si l'envoi échoue, le zip est copié sur
  votre Bureau.

### Corrections de l'interface
- **Effet des cartes de politique** : Rationalisme, Grand opéra, Simultaneum et Libre marché
  comptent aussi les rendements fixes ajoutés aux bâtiments qu'elles renforcent (gouverneurs…), et
  plus seulement leur rendement de base. Les bonus des cités-États sur ces bâtiments restent à part,
  comme dans le jeu, qui ne les multiplie pas.
- **Choix des routes commerciales** : pour une route intérieure, l'or que la capacité de Poundmaker
  donne à la ville de destination (+1 par camp et par pâturage) est maintenant affiché, avec une
  ligne d'infobulle qui nomme la ville.
- **Bandeau du haut** : le bouton de la population affiche aussi la moyenne des aménités nettes par
  ville (en vert à partir de +3, en rouge sous 0).
- **Menu principal** : la version de BetterGameExperience s'affiche en bas à gauche, après celles
  du jeu et de MPH (`… - MPH / … - BGE / 0.8.0`).

### Réglages
- Option de partie **« No First/Last moves (BGE) »** : cochée par défaut. L'hôte peut toujours la
  décocher.

## BetterGameExperience 0.7.0

En multijoueur, les correctifs **[tous les joueurs]** demandent que tous les humains soient en
0.7.0 : un salon qui mélange 0.6.x et 0.7.0 les désactive.

### Correctifs de jeu [tous les joueurs]
- **Unités empilées** : deux unités militaires de même classe peuvent se retrouver sur la même case
  (unité achetée dans une ville occupée, déplacement interrompu, téléportation…). L'unité en trop ne
  compte plus dans les bonus de contournement et de soutien : deux unités empilées à côté de la cible
  donnent +2, pas +4. La défense des villes ne change pas.

### Congrès mondial
- **Retirés** : les correctifs de la 0.6.x qui empêchaient la fin du tour pendant le vote (pas de fin de
  tour automatique, pas de fin de tour forcée sur une liste de résolutions vide, 240 s au minuteur
  de MPH). Le minuteur et la fin de tour du jeu et de MPH s'appliquent de nouveau pendant le vote. Le
  bouton **Revenir au vote**, l'écran d'accueil une fois par session et le recalcul des luxes
  restent.

### Corrections de l'interface
- **Écran des cités-États** : les émissaires placés mais pas encore confirmés ne sont plus perdus quand
  l'écran se rafraîchit (émissaire d'un autre joueur, quête, guerre ou paix). Ils ne sont retirés
  que si la cité-État est détruite ou si votre équipe lui déclare la guerre.
- **Écran d'accord** : le bouton de l'or disponible fonctionne de nouveau. Il pouvait rester impossible
  à cliquer après un accord précédent (bug du jeu).
- **Carte entière dès le début** (activé par défaut) : la minimap et la caméra couvrent toute la
  carte dès le premier tour, plus besoin d'épingles aux bords de la carte. Les zones inexplorées
  restent cachées. Purement local.
- **Zone de déplacement d'une unité sur un quartier ennemi** : plus de pic de lag quand vous la
  sélectionnez (campement ennemi non terminé), les zones de contrôle (cases
  rouges) sont affichées, et les cases d'eau où l'unité ne peut pas encore aller (côte avant
  l'embarquement, océan avant la Cartographie) ne sont plus proposées.

### Réglages
- Bouton **Réinitialiser BGE** dans l'onglet BetterGameExperience : remet tous les réglages à leur
  valeur par défaut.
- Nouveau réglage avancé de diagnostic, désactivé par défaut : **Mesure du coût de la zone de
  déplacement (en temps)**.

## BetterGameExperience 0.6.1

Reprend tout le contenu de la pré-release 0.6.0. En multijoueur, les correctifs **[tous les
joueurs]** demandent que tous les humains soient en 0.6.0 ou 0.6.1 : un salon qui mélange 0.5.0 et
0.6.x les désactive.

### Installation et mises à jour
- L'installeur n'affiche plus l'option de lancement Steam après l'installation : il indique que tout
  est en place, et vous pouvez le fermer. La notification de mise à jour du jeu suffit pour rester à
  jour. Une option de lancement déjà ajoutée continue de fonctionner.
- L'installeur s'appelle désormais `BetterGameExperience_V<version>.exe`. À son lancement, il
  supprime les anciens installeurs du même dossier (en général Téléchargements).

### Multijoueur
- **« No First/Last moves (BGE) »** **[tous les joueurs]** : vous restez sur l'écran de passage de
  tour, bouton de fin de tour compris, jusqu'à ce que l'hôte lance le tour, au lieu de voir votre
  tour ouvert avec vos ordres refusés.

### Congrès mondial
- *(expérimental)* **[tous les joueurs]** Pendant que vous attendez les autres joueurs, un bouton
  **Revenir au vote** rouvre la fenêtre de vote avec vos votes et la bonne quantité de faveur
  diplomatique.
- *(expérimental)* Le tour ne se termine plus contre votre gré pendant le vote : pas de fin de tour
  automatique, pas de fin de tour forcée sur une liste de résolutions vide, et le minuteur de MPH
  donne bien les 240 s prévues pour le vote.
- *(expérimental)* **[tous les joueurs]** Les luxes visés par une interdiction sont recalculés à
  chaque session.
- L'écran d'accueil du congrès mondial et son son ne s'affichent plus qu'une fois par session, au
  lieu de revenir chaque fois qu'un autre joueur revient au vote.

### Corrections de l'interface
- Avec Better Spectator Mod, le bandeau affiche de nouveau les ressources stratégiques de vos
  coéquipiers, et celles qu'un coéquipier débloque.

## BetterGameExperience 0.5.0 : première version publique

BetterGameExperience (BGE) corrige et accélère Civilization VI: Gathering Storm grâce à deux DLL posées à côté
des fichiers du jeu. **Aucun fichier du jeu n'est remplacé** : ni la vérification des fichiers par
Steam ni les mises à jour du jeu ne le retirent. Il fonctionne en DirectX 11 et DirectX 12, et avec
BBG, Better Spectator Mod et MPH.

Toutes les modifications sont listées ci-dessous, par catégorie. Celles marquées **[tous les
joueurs]** changent les règles de la partie : elles ne s'activent que si tous les humains du salon
ont la même version de BetterGameExperience. Tout le reste ne concerne que votre machine. On peut jouer
sans risque avec des joueurs qui n'ont pas le mod.

### Installation et mises à jour
- Installeur `BetterGameExperience.exe` : installation, mise à jour, réparation et désinstallation. Il
  trouve le jeu dans les bibliothèques Steam, et vérifie le manifeste signé et le SHA-256 de chaque
  fichier avant de copier quoi que ce soit.
- Deux fichiers sont ajoutés à côté de ceux du jeu : `winmm.dll` et `BGD_GameCore_XP2.dll`. Les
  installations de la 0.3.0 et d'avant sont nettoyées automatiquement. Une `winmm.dll` d'un autre
  mod n'est jamais écrasée.
- Notification de mise à jour au menu principal, y compris celui de MPH. **Mettre à jour** ferme le
  jeu, installe la nouvelle version et propose de relancer le jeu.
- Case « Pré-release » dans l'installeur pour tester les versions à venir, avec un retour en un clic
  à la version stable.
- Option de lancement Steam facultative, qui cherche une mise à jour à chaque lancement du jeu.
- Installeur en 11 langues, celles de Better Balanced Game. Il suit la langue de Windows, et on peut
  en choisir une autre en bas de la fenêtre.

### Réglages
- Nouvel onglet **BetterGameExperience** dans les Options du jeu. On y trouve un interrupteur général, qui
  coupe tout sans désinstaller, un réglage par fonctionnalité, et des options avancées masquées par
  défaut. Il prévient quand un changement demande un redémarrage.
- Crédits des auteurs des mods d'interface intégrés.
- Tous les textes en jeu existent dans les 11 langues de Better Balanced Game : anglais, français,
  allemand, espagnol, italien, japonais, coréen, polonais, portugais du Brésil, russe et chinois
  simplifié.

### Multijoueur
- **Vérification au salon** : chaque machine annonce sa version de BetterGameExperience, et l'hôte tranche
  pour tous. Des messages préfixés `BGE -` dans le chat disent qui n'a pas le mod ou a une autre
  version. Les reconnexions sont prises en compte. Si quelqu'un rejoint sans le mod alors que les
  correctifs **[tous les joueurs]** sont actifs, l'hôte est prévenu.
- Les messages `BGE -` du chat partent en anglais, lisibles par ceux qui n'ont pas le mod. Ceux qui
  l'ont les voient dans la langue de leur jeu.
- **Option de partie « No First/Last moves (BGE) »** **[tous les joueurs]** :
  - Tous les joueurs commencent leur tour ensemble. Les ordres sont retenus et le minuteur reste
    arrêté jusqu'à ce que tout le monde ait fini de charger (60 s au plus).
  - Le tour se termine pour tous quand le minuteur de l'hôte arrive à zéro.
  - L'ancienne synchro de tour de MPH est coupée.
- Parties chargées dans le salon de MPH : les cités-États ne provoquent plus « Opt. joueur
  invalides » ni le bouton Prêt grisé.

### Correctifs de jeu [tous les joueurs]
- Cases travaillées : une ville ne travaille plus une case 2/1 quand une case 2/2 est libre (focus
  nourriture et production).
- Les spectateurs de Better Spectator Mod ne comptent plus dans le tourisme. Ils ne votent plus au
  Congrès mondial et ne peuvent plus être la cible d'une résolution. Ils peuvent toujours consulter
  l'écran du Congrès.
- Une unité navale peut se téléporter vers une ville depuis n'importe quel district naval (port,
  parc aquatique, canal…), et plus seulement depuis un centre-ville, comme elle pouvait déjà se
  téléporter d'une ville vers n'importe quel district naval (fini l'amiral coincé sur un lac d'une
  case).

### Correctifs d'interface
- Les boutons d'attaque (ville, campement, frappe nucléaire depuis un silo) passent au-dessus des
  drapeaux d'unités, où on peut cliquer dessus.
- L'attaque à distance depuis une ville non sélectionnée se fait en un clic, et non plus deux.
- En visée d'une attaque à distance de ville ou de quartier, un clic sur le drapeau d'une unité
  ennemie tire sur elle.
- La caméra ne se recentre plus au clic sur un bouton d'attaque (on peut le réactiver).
- Le compteur de tours de production compte désormais les bonus de gouvernement : fascisme,
  autocratie et république maritime.
- Une merveille retirée de la file de production après deux échanges y est remise.
- La production en cours se met correctement à jour quand on passe à une autre ville.
- Les rendements d'empire du bandeau se mettent à jour après une action de gouverneur ou un
  personnage illustre.
- *(expérimental)* Les raccourcis clavier ne cessent plus de fonctionner quand une diplomatie ferme le menu Échap, ou
  après un double popup du Congrès.
- Échap ferme maintenant correctement l'écran des émissaires : les émissaires en attente ne
  bloquent plus la fin du tour.
- *(expérimental)* La lentille du colon est redessinée après la fondation d'une ville, et les cases trop proches
  d'une ville apparaissent comme interdites.
- Entrée suit la touche choisie pour « Action suivante » : on peut donc la retirer pour l'attribuer
  à un autre raccourci. Maj+Entrée force toujours la fin du tour.
- La zone de déplacement s'affiche pour une unité sur un quartier défendable ennemi.
- *(expérimental)* Les épingles de carte ne sont plus reconstruites quand rien n'a changé : moins de
  lag en multijoueur.

### Mods d'interface intégrés
Chacun peut être désactivé dans l'onglet BetterGameExperience. Si vous avez aussi le mod du Workshop, la
version intégrée à BGE le remplace.
- **Charges des bâtisseurs** sur les drapeaux d'unités (d'après Better Builder Charges Tracking, par
  wltk).
- **Tours restants des routes commerciales** sur les drapeaux de vos négociants (nouveau).
- Bouton **d'effacement des notifications** (d'après NotificationClear, par 336tarot).
- **Bandeau du haut étendu** : nourriture, production, population et luxes, avec le détail par
  ville (d'après TopPanel Extension Pro, par 号码菌 / Azukii).
- **Effet des cartes de politique** : chaque carte montre son gain réel pour votre empire (d'après
  Extended Policy Cards, par Aristos). Better Report Screen n'est plus nécessaire. Valeurs corrigées
  pour :
  - Raj, Médina, Monachisme et Prestige civil ;
  - Conscription et Levée en masse ;
  - les cartes Prétoire et Taxe coloniale ;
  - les points de personnages illustres, désormais affichés [icône]+x.
- **Écran des rapports amélioré**, avec les onglets Accords, Unités, Politiques, Cités-États et
  Villes (d'après Better Report Screen, par Infixo), avec les mêmes corrections.

### Performances et confort
- Passage de tour plus rapide :
  - Pendant une cascade de déclarations de guerre, les mises à jour des villes et de la diplomatie
    n'ont lieu qu'une fois. Le tour mesuré a été plus rapide de 2,4 s (−82 % sur la cascade) :
    beaucoup moins de lag quand des cités-États changent de camp pendant une guerre.
  - Deux caches accélèrent le tour de l'IA : améliorations constructibles et rendements des cases.
  - Les résultats sont identiques à ceux du jeu de base : sans risque face à des joueurs sans le mod.
- L'animation des frappes nucléaires est 20 fois plus rapide avec le combat rapide : environ 0,6 s
  au lieu de 11 s. Affichage seulement.
- Le limiteur d'images propose 45, 75, 90 et 100 images/s.

### Limites connues
- Le mod vise la version officielle actuelle du jeu. Après une mise à jour du jeu, les
  fonctionnalités concernées se coupent d'elles-mêmes jusqu'à la mise à jour de BetterGameExperience.
- Un joueur qui rejoint en cours de partie sans le mod, alors que les correctifs **[tous les
  joueurs]** sont actifs, se désynchronise. L'hôte est prévenu.
- Les correctifs marqués *(expérimental)* n'ont pas encore été essayés en partie, ou leur effet n'a
  pas encore été entièrement mesuré.
- Les traductions autres que l'anglais et le français n'ont pas encore été relues par des locuteurs
  natifs. Les corrections sont bienvenues.

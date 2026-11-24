# Journal des modifications

[English](../CHANGELOG.md) · autres langues : voir le [journal anglais](../CHANGELOG.md).

## BetterGameExperience 0.15.0

En multijoueur, tous les humains doivent être en 0.15.0 pour les correctifs **[tous les
joueurs]** : un salon qui mélange 0.14.0 et 0.15.0 les coupe.

### Versions du jeu
- **Civilization VI 1.0.12.72 n'est plus prise en charge** : ses joueurs sont passés à la 1.0.12.74.
  Sur 1.0.12.72, BetterGameExperience ne pose aucun correctif et le dit au menu principal, dans le
  salon et sur le bouton de rapport de bug.
- **1.0.12.58 avec les fichiers de jeu de juin 2024** : depuis la 0.14.0, cette combinaison n'avait
  plus aucun correctif. Elle est de nouveau reconnue.

### Correctifs multijoueur [tous les joueurs]
- **Tour d'un joueur absent** : quand un autre joueur terminait le tour d'un absent, le jeu pouvait le
  rouvrir de lui-même (une unité avec des mouvements restants), et plus personne ne le terminait : le
  tour restait bloqué. Un tour terminé par un autre joueur reste désormais terminé.
- **Vision d'une cité-État après une guerre** : un joueur redevenu suzerain d'une cité-État après une
  guerre contre elle (fréquent en Teamer) ne retrouvait jamais sa vision. Elle revient désormais, et
  les parties déjà touchées sont réparées au tour suivant.
- **Villages tribaux invisibles après un rechargement** : depuis la 0.14.0, les villages tribaux dans
  le brouillard disparaissaient de la carte après un rechargement ou une resynchro. Ils s'affichent
  de nouveau.
- **Morceaux réseau plus gros** : les gros messages réseau (sauvegardes, contrôles de synchro)
  partent désormais en morceaux de 16 Ko au lieu de 1,2 Ko, seulement en partie et quand chaque
  joueur connecté a la 0.15.0. Un joueur sans BetterGameExperience, ou d'une autre version, ramène
  aussitôt la taille du jeu.

### Multijoueur
- **Contrôles de synchro plus rapides chez l'hôte** : à chaque fin de tour, l'hôte ne recalcule plus
  les mêmes données de synchro pour chaque joueur. Seul l'hôte a besoin de la 0.15.0 pour cela.
- **Longs passages de tour après un retour en partie** : les deux changements ci-dessus visent les
  longs passages de tour vus après le retour d'un joueur (jusqu'à 45 s par tour). Le gain reste à
  confirmer en vraie partie : dites-nous ce que vous constatez.
- **Échange de places avec un joueur sans BetterGameExperience** : un échange de places dans le salon
  pouvait faire compter par l'hôte un joueur sans BetterGameExperience comme équipé, et lui activer
  les correctifs. Chaque annonce est désormais liée au compte Steam du joueur.
- **Prêt gardé dans le salon** : un changement de l'hôte qui laisse les réglages de la partie
  inchangés ne retire plus le Prêt des joueurs. Quand les réglages d'un joueur diffèrent encore de
  ceux de l'hôte une fois l'hôte Prêt, le chat le dit et demande à ce joueur de revenir.
- **Salon après un redémarrage annulé (MPH)** : après l'annulation d'un redémarrage lancé depuis le
  salon, la partie suivante créée ou chargée depuis le salon était chargée deux fois par chaque
  joueur. Elle ne l'est plus qu'une fois.
- **Salon d'une partie sauvegardée** : le rejoindre pouvait afficher « options de joueur non
  valides » pour les leaders de DLC ou de mods jusqu'à la réouverture de la configuration de la
  partie. La liste des joueurs est désormais reconstruite.

### Congrès mondial
- **Deuxième retour au vote** : un joueur revenu au vote une deuxième fois ne pouvait plus donner
  toutes ses voix. Les voix déjà envoyées sont désormais gardées.

### Chargements plus rapides
- **Processeur pendant l'écran de chargement** : depuis la 0.14.0, l'écran de chargement pouvait
  continuer sans pause entre les images ni VSync une fois la carte prête, processeur à fond. Cette
  phase rapide s'arrête désormais dès que le terrain est prêt (3 s au plus).

### Interface
- **Chance de réussite des espions** : la chance affichée au choix d'une mission d'espion compte
  désormais l'évasion : une réussite qui demande de s'échapper ne rapporte que si l'espion s'évade.
  Elle suppose la meilleure route de fuite et pas de contre-espion, comme le jeu. Activé par défaut
  (réglages de l'interface).
- **Signaux d'équipe** : le volume et les raccourcis des signaux sont gardés d'un chargement et d'un
  lancement du jeu à l'autre.
- **Onglet des réglages** : rouvrir l'onglet BetterGameExperience pouvait afficher d'anciennes
  valeurs.

## BetterGameExperience 0.14.0

En multijoueur, tous les humains doivent être en 0.14.0 pour les correctifs **[tous les
joueurs]** : un salon qui mélange 0.13.0 et 0.14.0 les coupe.

### Installation et mises à jour
- **Fichiers signés** : l'installeur et les deux DLL de BetterGameExperience portent désormais un
  certificat de signature de code (Certum, développeur open source). Windows affiche l'éditeur dans
  les propriétés du fichier et dans l'invite de l'installeur, et les alertes de SmartScreen et des
  antivirus devraient se faire plus rares.

### Versions du jeu
- **Civilization VI 1.0.12.58** : BetterGameExperience reconnaît maintenant cette ancienne version du
  jeu, en DX12 comme en DX11. Chaque version du jeu a ses propres adresses, et un jeu dont les
  fichiers ne correspondent pas à sa version n'installe aucun correctif.
- **Version du jeu non prise en charge** : un joueur dont la version du jeu n'est pas prise en charge
  par BetterGameExperience est signalé dans le chat à son arrivée (le message disait que sa copie
  n'était pas la version Steam actuelle).

### Correctifs multijoueur [tous les joueurs]
- **Moins de désynchros après un rechargement** : une partie chargée sur une machine calculait un
  peu autrement que la partie en cours sur les autres. Les améliorations mémorisées des joueurs
  étaient réécrites au chargement, les écouteurs d'événements tournaient dans un autre ordre, l'IA
  gardait pour de bon les préférences de son dirigeant (Sejong : -75 % de points de prophète) et les
  unités données à la création d'un quartier (bonus de difficulté de l'IA, départs en ère avancée)
  étaient créées dans un autre ordre. Une partie chargée calcule maintenant comme une partie en
  cours. Contribution d'OzrielTheLost.
- **Mêmes calculs sur tous les processeurs** : certains calculs (exponentielles, puissances)
  pouvaient arrondir différemment sur les processeurs avec et sans instructions FMA3. Toutes les
  machines les calculent maintenant de la même façon. Contribution d'OzrielTheLost.
- **Échange d'unités** : échanger deux unités pouvait en laisser une à mi-chemin, empilée sur une
  case avec une autre unité (un épéiste sur un tigre accroupi). Les deux unités vont maintenant au
  bout de leur déplacement.

### Multijoueur
- **Désynchros réparées sans rechargement** : quand un objet reste désynchronisé, l'hôte envoie
  maintenant toutes ses données, et la partie du joueur le répare sur place au lieu de dériver
  jusqu'à un rechargement complet. Seul l'hôte a besoin de la 0.14.0 pour cela. Contribution
  d'OzrielTheLost.

### Congrès mondial
- **Tour du congrès après une fin de tour forcée** : quand la fin de tour simultanée forçait la fin
  du tour d'un joueur, le tour du congrès pouvait se terminer deux secondes après s'être ouvert, sans
  vote. Le tour du congrès démarre maintenant avec un minuteur plein, la fenêtre de vote se rouvre
  quand le tour s'est terminé sans vote, et le bouton du congrès montre la session en cours.

### Chargements plus rapides
- **Rechargements plus rapides** : quand les mods sont les mêmes, un rechargement garde l'étape de
  configuration et laisse les textes des mods tels quels, la dernière phase du chargement n'attend
  plus entre ses étapes, et l'attente de la VSync est levée aussi en DX11. Mesuré sur une petite
  carte : de 13 s à 9 s. Réglages dans les options avancées, activés par défaut. Contribution
  d'OzrielTheLost.

### Interface
- **Raccourcis de l'écran d'accord** : dans la colonne « disponible », Maj+clic ajoute autant que
  l'accord l'accepte et Ctrl+clic ajoute dix fois un clic, pour les ressources, l'or, l'or par tour et
  la faveur. Activé par défaut. Contribution de huchenlei.
- **Badge de réduction des quartiers** : après la pose d'un quartier en réduction dans une ville, une
  autre ville déjà consultée affichait encore la réduction alors qu'il coûtait le prix plein.
- **Production en réserve** : le « +N » du bandeau de ville restait affiché après que le tour l'avait
  dépensé.

### Réglages
- **Nouveautés d'interface des mises à jour** : un nouveau réglage en tête de la section Interface.
  Sur « Désactivé », les ajouts d'interface des mises à jour suivantes arrivent coupés, et ceux que
  vous avez gardent leur état actuel.

## BetterGameExperience 0.13.0

En multijoueur, tous les humains doivent être en 0.13.0 pour les correctifs **[tous les
joueurs]** : un salon qui mélange 0.12.0 et 0.13.0 les coupe.

### Mise à jour du jeu 1.0.12.74
- **Mise à jour de Civilization VI du 19 novembre (1.0.12.74)** : BetterGameExperience reconnaît
  maintenant le jeu mis à jour, en DX12 comme en DX11. Les versions précédentes n'y installent aucun
  de leurs correctifs. Contribution de yodha.

### Correctifs multijoueur [tous les joueurs]
- **Joueur qui se déconnecte** : le jeu confiait un joueur déconnecté à l'IA sur chaque machine
  séparément, sans aucun message entre elles : les machines pouvaient jouer ce tour différemment et
  se désynchroniser. L'emplacement reste désormais humain, et l'hôte termine le tour du joueur absent
  une fois que tous les humains connectés ont terminé le leur. Un joueur expulsé est toujours repris
  par l'IA. Contribution de yodha.
- **Rechargement de tout le monde** : quand la partie d'un joueur ne correspond plus, l'hôte lance
  maintenant lui-même le rechargement et tous les joueurs chargent la même copie de sa partie ; le
  tour ne commence qu'une fois que chaque joueur l'a chargée et que sa partie a été vérifiée. Un
  joueur encore différent est rechargé une fois de plus. Contribution de yodha.

## BetterGameExperience 0.12.0

En multijoueur, tous les humains doivent être en 0.12.0 pour les correctifs **[tous les
joueurs]** : un salon qui mélange 0.11.0 et 0.12.0 les coupe.

### Correctifs multijoueur [tous les joueurs]
- **Désynchronisation après une reconnexion** : un joueur reconnecté, ou rechargé seul, faisait
  tourner une partie fraîchement chargée face aux parties en cours des autres. Les deux diffèrent
  dans des données que la vérification de synchronisation ne couvre pas : la partie dérivait de
  nouveau et rechargeait ce joueur encore et encore. Désormais, quand la partie d'un joueur ne
  correspond plus, l'hôte en donne la raison dans le chat et tout le monde recharge aussitôt la
  partie de l'hôte ; l'hôte attend la vérification de chaque joueur sur la partie rechargée, puis le
  tour commence pour tous. Contribution de yodha.
- **Reconnexion en fin de tour** : un joueur reconnecté pendant une fin de tour était comparé au tour
  suivant, ce qui causait une réparation inutile puis un rechargement tous les deux tours.
  Contribution de yodha.
- **Remap de Multiplayer Helper** : après un remap, l'hôte faisait tourner la partie qu'il venait de
  générer, et tous les autres joueurs la même partie chargée depuis la sauvegarde de l'hôte ; l'IA
  faisait des choix légèrement différents de chaque côté (réparations de plusieurs Mo par tour dans
  une partie à 8 joueurs). L'hôte charge maintenant lui aussi sa propre sauvegarde : toutes les
  machines font tourner la même partie. Coût : un écran de chargement de plus pour l'hôte, une fois
  par remap. Contribution de yodha.
- **Chaque partie repart de zéro** : certains compteurs du jeu passaient d'une partie à la suivante
  dans la même session, et les objets étaient numérotés autrement que chez les autres joueurs.
  Contribution de yodha.

### Multijoueur
- **Double chargement au lancement d'une sauvegarde** : l'hôte n'envoie plus une deuxième copie de la
  partie aux joueurs qui chargent encore, ce qui en faisait charger certains deux fois. Quand une
  copie est encore refusée, le chat affiche « Double chargement évité ». Contribution de yodha.
- **Messages réseau abîmés** : un message qui effacerait les réglages de la partie est aussi refusé.
  Le chat nomme le joueur et ce qui a été évité (« Gel de la partie évité », « Effacement des
  réglages de la partie évité »). Contribution de yodha.
- **Messages coupés en fin de chargement** : un message mis en attente pendant un chargement n'est
  plus collé à un message qui arrive encore en morceaux. Contribution de yodha.
- **Correctifs de BetterGameExperience décidés au chargement** : la décision d'activer les correctifs
  **[tous les joueurs]** est revérifiée au chargement et à la fermeture de l'écran de chargement, un
  nouvel hôte après une migration la reprend (les joueurs arrivés après une migration jouaient sans
  eux), et les joueurs ignorés le restent après un rechargement. Contribution de yodha.
- **Jeu pas à jour** : un joueur dont le jeu n'est pas la version Steam actuelle est annoncé dans le
  chat à son arrivée, car les correctifs réseau de BetterGameExperience ne peuvent pas fonctionner
  sur son PC.
- **Popup « Partie en pause »** : il ne s'empile plus pendant les chargements et ne s'affiche plus
  par-dessus l'écran de chargement.
- **Configuration de la partie** : les signaux d'équipe ne peuvent plus être activés ni désactivés
  une fois la partie lancée (le changement ne faisait rien, ou laissait les signaux actifs),
  sauvegardes comprises. No First/Last moves et les signaux d'équipe ne peuvent plus être activés
  dans un salon où un joueur n'a pas BetterGameExperience ou pas la bonne version, mais peuvent
  toujours être désactivés. Contribution de mariusbw.

### No First/Last moves
- L'option reste active une fois activée. L'hôte n'attend plus un joueur qui n'a pas été prêt deux
  tours de suite, et le dit dans le chat. Un joueur qui ne reçoit pas le signal de départ de l'hôte
  pendant 75 s commence son tour seul. Contribution de yodha.
- **Chat plus discret** : le message d'activation se réduit à « No First/Last moves actif. », et les
  lignes de retenue des ordres puis de départ s'affichent une fois par partie au lieu de chaque tour.
  L'annonce « a BetterGameExperience actif » est envoyée une fois par salon, et non à chaque
  réouverture de l'écran du salon.

### Chargements plus rapides
- Nouveaux réglages dans l'onglet BetterGameExperience, tous activés par défaut, qui raccourcissent
  les chargements sans changer la partie : recherche plus rapide des fichiers graphiques (plusieurs
  secondes depuis le menu principal), base de gameplay et textes des mods gardés d'un chargement à
  l'autre quand les mods sont les mêmes (environ 1,5 s chacun), pas d'attente de la VSync sur les
  écrans de chargement (environ 2 s, seulement si la VSync est active), un seul nettoyage Lua à la
  fermeture de l'interface (environ 2 s avant un remap), un premier chargement allégé pour l'hôte au
  remap (environ 10 s), et une analyse des goulets plus rapide à la génération de la carte (environ
  2 s, même résultat). Le journal des goulets de l'IA (AI_ChokePoint.csv, 12 Mo) n'est plus écrit
  par défaut.
- **Threads de tâches** : les threads inoccupés gaspillent moins de temps processeur en attendant
  (réglages dans le même onglet).

### Interface
- **Formation la plus haute en premier** : dans la liste de production, le bouton principal d'une
  unité forme la plus haute formation que la ville peut former (armée, sinon corps ; armada, sinon
  flotte en mer), et l'unité seule passe dans le menu déroulant en dessous. Les achats en or et en foi
  ne changent pas. Activé par défaut. Contribution de huchenlei.
- **Alerte d'habitations précoce** : le bandeau de vos villes montre une maison orange à une
  habitation de la limite (croissance -50 %) et la maison rouge dès que la ville l'atteint (-75 %) ;
  le jeu ne montrait la maison rouge qu'au-delà de la limite. Activé par défaut. Contribution de
  huchenlei.
- **Cartes de politique** : leurs effets ne disparaissent plus pour le reste de la partie quand
  l'écran du gouvernement est dessiné pour la première fois au mauvais moment (vu après le
  rechargement d'une partie) ; ils ne revenaient qu'après un autre rechargement.
- **Drapeaux des unités** : les unités de combat qui ont des charges (légion romaine, toa, nau…)
  montrent de nouveau leurs promotions au lieu de leurs charges ; les ouvriers, les ingénieurs
  militaires et les autres civils et unités de soutien montrent toujours leurs charges.

### Rapport de bug
- Les journaux qui contiennent des jetons de connexion (FiraxisLive.log, EOS.log) sont exclus, et
  les longs journaux gardent leur début et leur fin. Contribution de yodha.

## BetterGameExperience 0.11.0

En multijoueur, tous les humains doivent être en 0.11.0 pour les correctifs **[tous les
joueurs]** : un salon qui mélange 0.10.0 et 0.11.0 les coupe.

### Correctifs multijoueur [tous les joueurs]
- **Bonus de production d'unités qui s'usent** : retirer un bonus de production d'unités en
  pourcentage (les +30 % aux ouvriers d'Ilkum, ou le bonus d'une ville à un type d'unité) enlevait un
  peu plus que ce qu'il avait ajouté, environ 0,4 % à chaque fois, et la perte grandissait à chaque
  échange de la carte. Retirer un bonus enlève maintenant exactement ce qu'il avait ajouté. Les
  sauvegardes gardent la perte déjà accumulée. Contribution de Serianol.

### Multijoueur
- **Partie figée juste après le chargement** : un joueur à la connexion lente pouvait recevoir la
  partie une seconde fois de l'hôte juste après la fin de son chargement, la recharger depuis la
  carte et rester figé. Cette seconde copie tardive est maintenant refusée ; l'arrivée, la
  reconnexion et les resynchronisations fonctionnent comme avant.
- **Plantage au chargement d'une sauvegarde après une autre partie** : la reconnexion des joueurs
  relayés par l'hôte (0.10.0) pouvait lire la connexion de la partie précédente, déjà fermée, et
  faire planter le jeu pendant le chargement de la suivante. Contribution de yodha.
- **Configuration de la partie** : les options de partie ajoutées par BetterGameExperience sont
  grisées quand l'hôte a désactivé BetterGameExperience.

### Signaux d'équipe
- Nouvelles images et nouveaux sons pour la roue, les marqueurs et chaque appel, installés avec
  BetterGameExperience. Ils ne sont ajoutés à la partie que si l'hôte active les signaux d'équipe et
  que tous les joueurs ont BetterGameExperience : un joueur sans BGE peut toujours rejoindre le
  salon. Contribution de mariusbw.
- Le **volume des signaux d'équipe** se règle juste sous les effets sonores dans les options audio,
  au menu principal comme en partie, et les raccourcis des signaux restent listés dans les options
  des touches. Contribution de mariusbw.

### Réglages
- **Onglet BetterGameExperience** : activer ou désactiver BetterGameExperience demande toujours de
  redémarrer le jeu. Revenir à la valeur précédente d'un réglage sans rouvrir l'écran est de nouveau
  enregistré ; ce changement était ignoré.

## BetterGameExperience 0.10.0

En multijoueur, tous les humains doivent être en 0.10.0 pour les correctifs **[tous les
joueurs]** et les signaux d'équipe : un salon qui mélange 0.9.x et 0.10.0 les coupe.

### Correctifs multijoueur [tous les joueurs]
- **Joueurs bloqués sur « pas prêt »** : une action diplomatique (ambassade, accord) reçue juste à
  la fin du tour pouvait faire commencer le tour suivant trop tôt par l'hôte. Certains joueurs le
  commençaient alors déjà marqués comme ayant fini, sans pouvoir redevenir prêts. Le tour suivant
  attend maintenant la vérification du jeu, et l'action est toujours appliquée.
- **Spectateurs (Better Spectator Mod)** : la refonte des spectateurs de la 0.9.0 est suspendue en
  attendant des tests plus approfondis, car les parties avec un spectateur étaient moins stables.
  BetterGameExperience ne change plus le niveau de civilisation des spectateurs et ne remplace plus
  les scripts de Better Spectator Mod, et le réglage « Better Spectator Mod pour les spectateurs »
  disparaît pour l'instant. Les spectateurs comptent de nouveau comme civilisations majeures pour
  les scripts, les victoires et le score, comme avec Better Spectator Mod seul. Le tourisme et les
  votes du Congrès mondial les ignorent toujours.
- **Gardé pour les spectateurs** : changer le joueur observé ne désynchronise toujours pas la
  partie, et le tour du spectateur se termine toujours seul après une pause de MPH. Ces deux
  correctifs s'appliquent désormais aux scripts d'origine de Better Spectator Mod.

### Nouveautés
- **Signaux d'équipe (BGE)** : marquez une case pour votre équipe avec **X**, ou maintenez **X** pour
  une roue d'appels (danger, j'arrive, aide, ennemi disparu) ; **Alt+1–4** les envoient directement.
  Les coéquipiers voient et entendent le signal avec le nom de l'envoyeur, les spectateurs voient les
  deux équipes. Désactivé par défaut : l'hôte coche **Signaux d'équipe (BGE)** dans la configuration
  de la partie. Raccourcis dans les options des touches du jeu, volume dans les options audio ;
  `/mute nom` bloque le chat et les signaux d'un joueur. Contribution de mariusbw.

### Multijoueur
- **Joueurs relayés par l'hôte** : deux joueurs dont la connexion directe a échoué à l'arrivée
  passent par l'hôte jusqu'à la fin de la partie, et sont tous deux déconnectés si l'hôte part.
  Pendant le départ de tour simultané, BetterGameExperience tente maintenant de les reconnecter
  directement (trois essais au plus) et l'annonce dans le chat. Contribution de yodha.
- **Pings bloqués à 0 ms** : le ping d'un joueur dans la liste des joueurs ne reste plus figé à 0 ms
  après l'arrivée ; il est de nouveau mesuré toutes les 5 secondes. Plus besoin de quitter et
  revenir pour le « réparer », ce qui provoquait souvent une désynchronisation. Contribution de yodha.
- **Messages réseau endommagés** : la protection ajoutée en 0.9.0 fonctionne aussi avec la version
  DirectX 11 du jeu. Contribution de yodha.
- **Joueurs sans BetterGameExperience** : l'avertissement en partie pour un joueur arrivé sans
  BetterGameExperience ne s'arrête plus sur une erreur de script. Contribution de yodha.

### Installation et mises à jour
- L'installeur télécharge et relance le jeu sans démarrer de programmes auxiliaires cachés, et porte
  des informations de version : les antivirus devraient moins souvent le signaler.

## BetterGameExperience 0.9.1

Compatible avec la 0.9.0 en multijoueur : des joueurs en 0.9.0 et en 0.9.1 jouent ensemble avec tous
les correctifs.

### Corrections de l'interface
- **Écran des rapports** : les onglets Rendements et Villes fonctionnent de nouveau. Le calcul de
  l'effet des cartes de politique, chargé dans le même écran, remplaçait certaines de ses données et
  les cassait. Les onglets Politiques et Cités-États n'échouent plus sur une ligne avec des points de
  grands personnages, et trier une colonne de l'onglet Villes n'échoue plus quand l'onglet
  Rendements n'a pas été ouvert avant.
- **Quartiers en réduction** : la réduction affichée est maintenant celle que le jeu applique. Le jeu
  ne met à jour le coût d'un quartier qu'à certains moments (nouvelle technologie ou nouveau dogme,
  nouvelle ville, quartier du même type posé ou terminé) : la liste pouvait annoncer une réduction
  que le jeu n'accordait que quelques tours plus tard.

## BetterGameExperience 0.9.0

Comprend tout le contenu de la pré-release 0.8.1. En multijoueur, tous les humains doivent être en
0.9.0 pour les correctifs **[tous les joueurs]** : un salon qui mélange 0.8.x et 0.9.0 les coupe.

### Correctifs du multijoueur [tous les joueurs]
- **Spectateurs (Better Spectator Mod)** : ils ne comptent plus comme des civilisations majeures et
  sortent des listes de joueurs des scripts, des victoires et du score. BetterGameExperience
  embarque ses propres versions des scripts de Better Spectator Mod, faites pour cela. Contribution
  de banipouz.
- **Désynchronisation due à un spectateur** : changer le joueur qu'observe un spectateur ne
  désynchronise plus la partie (Better Spectator Mod l'écrivait dans les données de la partie, sur
  la seule machine du spectateur).
- **Spectateurs dans le bandeau des dirigeants** : les joueurs ne voient plus les spectateurs dans
  le bandeau en haut à droite, où ils apparaissaient comme une IA, avec un portrait « ? », leur nom
  et leurs rendements.
- **Tour du spectateur après une pause** : après une pause de MPH, le tour du spectateur se termine
  de nouveau tout seul, au lieu d'attendre qu'il le passe à la main.

### Multijoueur
- **Messages réseau abîmés** : un message abîmé, vu à la déconnexion d'un joueur, pouvait geler le
  jeu jusqu'au manque de mémoire, quelques minutes plus tard. Les valeurs impossibles d'un tel
  message sont maintenant écartées et la partie continue ; une ligne dans le chat le dit à tous. Une
  configuration de partie ou de joueur abîmée n'est plus appliquée, au lieu d'effacer la
  configuration. Protège les joueurs qui ont BetterGameExperience. Contribution de yodha.
- **No First/Last moves avec un hôte spectateur** : le tour démarre maintenant dès que les joueurs
  sont prêts, au lieu de les retenir jusqu'au délai de sécurité.

### Nouveautés
- **Avancement de la recherche et du dogme** : dans le panneau en haut à gauche, à côté du nom de la
  technologie et du dogme en cours, **x/y** indique la science (la culture) déjà investie et le coût
  total, suivi de **(+z)** quand le jeu garde de la science (de la culture) en excédent d'une
  technologie (d'un dogme) terminée avec plus que son coût. Un réglage permet de le désactiver.
- **Disable Advanced UI indicators (BGE)** : nouvelle option de partie, juste sous No First/Last
  moves, décochée par défaut. Quand l'hôte la coche, aucun joueur ne voit l'avancement de la
  recherche et du dogme (**x/y (+z)**) ni la production en réserve des villes, quels que soient ses
  réglages. Elle est relue à chaque partie : passer à une partie sans l'option, sans relancer
  Civ VI, les fait réapparaître.
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
- **Écran diplomatique** : après la rencontre d'un joueur dont l'équipe manquait à la liste des
  équipes du jeu, les icônes des dirigeants échouaient à chaque tour, et ouvrir l'écran diplomatique
  pouvait masquer toute l'interface pour de bon. Les icônes fonctionnent de nouveau, et si l'écran
  diplomatique ne s'ouvre pas, l'interface revient.
- **Imhotep** : son effet affiche maintenant le montant à la vitesse de la partie (87, ou 175 pour
  une merveille antique ou classique, en vitesse En ligne), dans l'écran des personnages illustres
  et la Civilopédie, au lieu de 175 dans tous les cas.
- **Pause de MPH** : le menu MULTIPLAYER HELPER et le vote de remap s'affichent maintenant au-dessus
  de l'écran de pause de MPH, avec des boutons cliquables.
- **Bandeau du haut** : les activités moyennes par ville sont arrondies à l'entier le plus proche,
  et non plus à l'entier inférieur.
- **Signaler un bug (BGE)** : la sauvegarde est maintenant jointe au rapport ; seules les
  sauvegardes automatiques partaient.

### Réglages
- Les nouveautés d'interface (annuler depuis le bandeau, quartiers en réduction, production en
  réserve, avancement de la recherche et du dogme) et les scripts de Better Spectator Mod ont chacun
  un réglage dans l'onglet BetterGameExperience, activé par défaut.

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

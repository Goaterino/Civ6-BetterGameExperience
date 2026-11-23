# Passages de tour longs : retour d'un joueur, débit réseau et contrôles de synchro

Analyse seule (2026-11-23), sans correctif. Point de départ : deux rapports d'une même partie,
BGE-2890AC (hôte) et BGE-AEA3D7 (un client), puis une lecture de l'exe dans Ghidra :
`CivilizationVI_DX12.exe` **1.0.12.58**. Les adresses ci-dessous sont celles de ce build. Le code
du jeu est le même en 1.0.12.74 : seules les adresses sont décalées.

## Les pistes, en bref

| Piste | Gain attendu | POL | État |
|---|---|---|---|
| [Cache des hachages de l'hôte](#piste-1--cache-des-hachages-de-lhôte) | environ 25 s par tour en fin de partie, pour tous | non (hôte seul) | à implémenter |
| [Resynchro générale au-delà d'un seuil](#piste-2--resynchro-générale-au-delà-dun-seuil) | supprime les environ 45 s par tour d'un joueur qui ne converge pas | non | à concevoir |
| [Morceaux plus gros](#piste-3--morceaux-plus-gros) | liens lents bien plus rapides, **si** Steam limite en messages | oui | essai à deux machines d'abord |
| [Nouvelle pile Steam](#piste-4--nouvelle-pile-steam) | débit réglable, mesures fiables | oui (par joueur) | gros chantier, plus tard |

## Le cas d'origine : BGE-2890AC et BGE-AEA3D7

**La partie.**
- 11 humains, T87, BGE 0.14.0, correctifs POL actifs.
- Hôte : acsess (joueur 0). Client : Scribus (joueur 8).
- Journaux réseau à UTC+1.

**Deux retours en cours de partie.**
- **dowon3737 (joueur 6), en plein T55** : départ à 10:16:38 UTC, partie en pause, sauvegarde de
  3,77 Mo, reprise à 10:19:14.
- **Scribus, pendant la barrière du T68** : la coupure vient du lien 1↔8, et il a fallu trois
  entrées. Il a chargé une sauvegarde de fin de tour et s'est resynchronisé en un tour.

**Le passage de tour saute après le premier retour.** De 14–17 s (T50–54), il passe à 68 s au T55,
puis 59–97 s jusqu'au T86. Tout le surcoût (environ +45 s par tour) est dans la barrière de synchro :

| Fin du T | Passage | Barrière | Delta envoyé au joueur 6 |
|---|---|---|---|
| 1–39 | 6–11 s | 5–8 s | < 9 Ko |
| 50–54 | 14–17 s | 11–12 s | 6–12 Ko |
| **55 (retour)** | **68 s** | **63 s** | **7,24 Mo, 107 archives** |
| 56–67 | 59–79 s | 54–72 s | 5,5–5,8 Mo, 74–83 archives |
| 69–86 | 76–97 s | 70–88 s | 5,8–5,9 Mo, 82–83 archives |

**Le joueur 6 ne converge jamais.**
- À chaque contrôle, l'hôte lui renvoie environ 5,8 Mo, plus que la sauvegarde entière. Ce sont
  presque toujours les mêmes archives :
  - `AI::CityBuild` (62 à 72 villes) des joueurs {0, 1, 4, 5, 6, 7, 10} ;
  - `AI::GrandStrategic` des mêmes joueurs.
- La partie attend son `NetGameSyncProcessed` 40 à 50 s, contre environ 2 s avant son départ.
- Aucune `NetSnapshotRequest` : la réparation par deltas ne bascule jamais sur une resynchro
  complète.
- Deux hypothèses, non tranchées :
  - **H1** : le correctif POL de chargement (`gs_read_hook`, `proxy/src/partie_chargee.rs`) ne
    reproduit pas l'état de l'hôte ;
  - **H2** : un état d'IA non archivé, calculé en début de tour, manque à un joueur qui charge en
    plein tour. Scribus, qui a chargé en fin de tour, a convergé.

**Une seconde lenteur, sans lien avec les retours.** L'hôte traite les contrôles l'un après l'autre,
et chaque contrôle lui coûte de plus en plus cher (voir piste 1) :
- environ 1 s à 11 000 archives, environ 3 s à 28 000 ;
- au T86, le dernier contrôle arrive 31 s après la fin du tour.

**Trou dans la trace.** `sync repair: log limit reached` dès le T25 (`MAX_LOGS = 400`,
`startup_proxy/src/reparation_synchro.rs`), saturé par `AI::Religious`.

Les commits « joueur absent » de `main` (9c7752c, 42197c9) ne changent rien à ce cas : le joueur 6
était connecté et envoyait lui-même sa fin de tour.

## Débit entre joueurs : pas de limiteur dans le jeu

### Le chemin d'envoi en partie Steam

1. **`FNetSession` envoie le message** (`140832a40`, la ligne `Net SEND (…) chunks=…`). Pour chaque
   destinataire, il appelle `14083e930`.
2. **`14083e930` découpe le message** et envoie tous les morceaux dans la même boucle, sans budget
   par image ni attente.
3. **`FNetAccessENetSteam::Send` (`14084ec20`)** range chaque morceau dans la file du thread de
   transport (`14085f6d0`, préfixe `F`).
4. **`FSteamTransportThread::Update` (`14085db90`)** vide toute la file à chaque passage :
   - `SendP2PPacket(steamID, data, taille, 2 = k_EP2PSendReliable, canal 0)` sur
     `SteamNetworking005`, l'ancienne API P2P ;
   - **le code de retour n'est pas lu** ;
   - côté réception, un paquet par passage (`ReadP2PPacket`, tampon fixe de 0x800 octets), mais la
     boucle (`1409946e0`) ne dort pas : la période, le champ `+0x74` du transport, vaut 0.

C'est donc la couche P2P de Steam qui fixe le débit. La doc de Steam annonce pour le mode fiable un
découpage et une fenêtre glissante, sans chiffre
([ISteamNetworking](https://partner.steamgames.com/doc/api/ISteamNetworking)).

**ENet** (version proche de la 1.2) ne sert pas en partie Steam : c'est le chemin du LAN et de
l'IP directe. Là, sa fenêtre de 32 Ko de données fiables en vol est un vrai plafond (32 Ko ÷ ping) :
- un seul `enet_host_create` (`140c6f100`), avec une bande passante de 0 ;
- bornes de la fenêtre dans `140c6ee70`, `140c6ff80`, `140c70030` ;
- test en `140c71945`.

### Ce que donnent les journaux : un débit propre à chaque lien

**Hôte → client** (envoi → accusé) :
- vers Scribus, la sauvegarde de 4,18 Mo arrive en 4 à 6 s, soit environ 0,7 à 1,4 Mo/s ;
- vers dowon, environ 135 Ko/s, presque identique sur 30 tours (environ 118 morceaux/s) ;
- le 2026-11-13 (`analyses/double_load`), un autre lien passait 265 Ko/s.

**Client → hôte** (contrôles de synchro de 300 à 600 Ko, du premier morceau au message complet) :

| Joueur | Morceaux/s, au mieux | Ko/s, au mieux |
|---|---|---|
| Finarfin, nutella67 | ≥ 480 | ≥ 540 |
| Antosak, Belizarius | ≥ 270 | ≥ 310 |
| Scribus | 103 | 117 |
| iDar | 86 | 98 |
| co6a4ken, apap, Baboonik | 53–55 | 61–62 |
| **dowon3737** | **33** | **38** |

**Limites de mesure.**
- Les deux heures dépendent du fil principal de l'hôte, souvent occupé à traiter un autre contrôle :
  ces débits sont des minima.
- Les heures sont à la seconde : les liens rapides sont seulement bornés.

**Ce qu'on en tire.**
- Le débit change d'un lien à l'autre, mais reste stable sur un même lien.
- Trois liens plafonnent exactement au même niveau, 53–55 morceaux/s. C'est un indice d'une limite
  comptée en **messages** chez Steam, pas une preuve.
- Aucun journal ne dit si un lien passe par les relais de Valve ni quel est son ping.

## Piste 1 : cache des hachages de l'hôte

**Ce que fait l'hôte.**
- La fonction qui traite un contrôle est `1407f0b70`, la même que `sync_notes` et
  `reparation_synchro` redirigent.
- Pour chaque archive annoncée par le client, elle recalcule le hachage de l'archive de l'hôte avec
  `140dc5390` :
  - elle appelle le hachage de chaque variable (méthode virtuelle `+0x38`) ;
  - range les résultats dans un tableau qu'elle agrandit au fur et à mesure ;
  - puis calcule leur FNV-1a.
- Rien n'est gardé entre deux contrôles. Avec 10 clients, l'hôte refait 10 fois ce calcul sur environ
  30 000 archives, alors que le client a fait le sien une seule fois.

**Le cache.**
- Calculer chaque hachage une fois par barrière et le réutiliser pour les contrôles suivants.
- Gain attendu au T86 : environ 3 s pour le premier contrôle, puis presque rien, soit environ 25 s de
  moins par tour.
- **Hôte seul** : les clients exécutent le code d'origine.
- **Pas de parallélisation.** Elle appellerait GameCore depuis plusieurs threads, et le cache retire
  presque tout le calcul.

**L'état de l'hôte change-t-il pendant la barrière ?**
- **Le traitement.** Les contrôles sont traités à leur arrivée (`1407ed820`) et à chaque mise à jour du
  gestionnaire (`1407ece80`), par `1407f4260`, qui traite d'une traite les contrôles déjà arrivés.
  Entre deux arrivées, le fil principal fait tourner ses images.
- **Le tour suivant.** Il ne démarre pas pendant le contrôle : le contrôleur ne sort de la barrière
  (`1400f3930`) qu'une fois la synchro finie (`DAT_142309628+0x40`).
- **Les messages, eux, sont appliqués.** GameCore applique les messages qui arrivent pendant la
  barrière :
  - dans BGE-2890AC, 3 `UnitOperation`, 4 `PlayerOperation` et 18 `PlayerConnectionChange` en
    81 barrières ;
  - exemple au T11 : le `MOVE_TO` de dowon arrive à la seconde où commence le contrôle. L'hôte l'a
    appliqué, et `Unit 6-1-262146` est en écart pour les 10 clients, avec le même hachage côté hôte.
- **Test sur les journaux.** Quand plusieurs clients signalent la même archive dans une même barrière,
  l'hôte note à chaque fois son propre hachage :

  | Rapports (côté hôte) | Archives vues par au moins 2 clients | Hachage de l'hôte modifié |
  |---|---|---|
  | BGE-2890AC, -387676, -2A9A3B, -4EB5B0, -FB5126, -3D6183 | 796, sur des barrières jusqu'à 265 s | 0 |
  | BGE-93D8D4 | 115 | 8, tous dus à un rechargement général entre les deux contrôles |

  Ce test ne voit que les archives en écart chez au moins deux clients, surtout `AI::Religious`.

**Conception retenue.**
- Le cache est marqué **périmé** dès qu'un message de GameCore est appliqué pendant la barrière. Il
  est aussi vidé à chaque nouveau contrôle, chargement, départ ou retour de joueur, et migration
  d'hôte. Les résultats restent alors identiques au jeu d'origine.
- Le gain disparaît seulement dans les barrières qui reçoivent un ordre tardif (7 sur 81 dans
  BGE-2890AC).
- **Seul l'appel de `140dc5390` fait par le traitement du contrôle est redirigé.** La même fonction
  sert au client pour construire son contrôle, et sans doute ailleurs : aucun autre appel n'est
  touché.

**À vérifier.**
- Le verrou d'accès de l'interface à GameCore (`DAT_1412fe640`, `1400f2c00`) pendant le contrôle.
- Le moment exact où GameCore applique un message remis par l'exe.

## Piste 2 : resynchro générale au-delà d'un seuil

**L'écart est net.**
- Un client sain reçoit 3 à 16 Ko par tour.
- Le joueur 6 en recevait 5,8 Mo sur environ 80 archives.
- Un seuil vers 500 Ko ou 30 archives, **2 contrôles de suite sur les mêmes archives** (plus de 80 %
  en commun), ne se déclencherait pas par erreur.

**Pourquoi un rechargement général plutôt qu'une sauvegarde au seul joueur.** Le joueur 6 avait déjà
reçu une sauvegarde complète et a divergé aussitôt. Si H2 est juste, seul le chargement par tous de
la même sauvegarde de fin de tour remet tout le monde au même état. Le chemin `reload:` existe déjà.

**Règles.**
- À la barrière de fin de tour seulement.
- Un rechargement automatique au plus par période : si l'écart revient, on s'arrête et on l'annonce
  dans le chat.
- Un réglage dans `rollover_proxy.cfg`.

**À vérifier.** `reload:` avec des clients sans BGE, et le risque de double chargement
(`analyses/double_load`), plus grand sur un lien lent.

## Piste 3 : morceaux plus gros

**La taille des morceaux est une variable.**
- Variable globale `14128f328` (0x48e = 1 166), réglée à l'ouverture de session (`1408499e0`) depuis
  la table `14128f478` : 1 166 / 1 166 / 1 200 / 1 200 / 1 170 selon le transport.
- Mais `1407f7ab0` ramène toute valeur au-dessus de 1 165 à 1 166.
- En mode fiable, Steam accepte des messages jusqu'à 1 Mo.

**Ce qu'il faut changer (correctif POL).**
1. La variable ou le plafond de `1407f7ab0`.
2. Le tampon de réception de 0x800 octets (`DAT_14238e040`, dans `14085db90`). On peut demander la
   taille avec `IsP2PPacketAvailable`.
3. La même valeur chez tous les joueurs : le receveur recalcule le nombre de morceaux avec sa propre
   taille (`1407f7ae0`, à confirmer).
4. Vérifier que `startup_proxy/src/flush.rs` ne suppose pas 1 166 octets.

**Gain incertain.**
- Si Steam limite en octets en vol (débit ≈ fenêtre ÷ ping), presque rien.
- S'il limite en messages, ×15 à ×50 sur les liens lents.

**Essai à deux machines.** Morceaux de 16 Ko, tampon de 64 Ko, chronométrer l'envoi d'une
sauvegarde et d'un contrôle, avec une trace temporaire en ms au thread de transport.

## Piste 4 : nouvelle pile Steam

**Quelle interface.** `ISteamNetworkingMessages`, plutôt que `ISteamNetworkingSockets` : elle a le
même modèle « Steam ID + octets + canal » que l'ancienne API.

**Ce qu'elle apporte.**
- Débit réglable : `SendRateMin/Max`, 256 Ko/s par défaut
  ([steamnetworkingtypes.h](https://github.com/ValveSoftware/GameNetworkingSockets/blob/master/include/steam/steamnetworkingtypes.h)).
- Tampon d'envoi réglable (`SendBufferSize`).
- Messages fiables jusqu'à 512 Ko.
- Mesures par lien : ping, qualité, octets en attente.
- Routage par les relais de Valve (SDR).

**Où la brancher.** Remplacer `FSteamTransportThread::Update` (`14085db90`) :
- vider la file `+0x50` avec `SendMessageToUser` ;
- recevoir avec `ReceiveMessagesOnChannel`, puis `1408526b0`.

**Points durs.**
- Choix de la pile **par joueur**, selon que le joueur annonce BGE.
- Interfaces à demander par `SteamInternal_FindOrCreateUserInterface` (`SteamNetworkingMessages002`,
  `SteamNetworkingUtils004`), car le `steam_api64.dll` du jeu est ancien.
- Acceptation des sessions.
- Délais de coupure à faire concorder avec ceux du jeu (20 s).

## Mesures à ajouter dans le proxy

- **Par joueur, à chaque fin de tour** : `GetP2PSessionState` (relais ou direct, octets en attente)
  et le ping.
- **Par contrôle et par joueur, hors limite de journal** : archives en écart, octets envoyés, attente
  du `NetGameSyncProcessed`, temps passé dans le traitement du contrôle.
- **Le cap du journal `sync repair`** : par joueur ou par tour.

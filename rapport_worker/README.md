# Relais des rapports de bug

Un Cloudflare Worker entre le bouton « Signaler un bug (BGE) » du jeu
([startup_proxy](../startup_proxy/README.md#le-signalement-dun-bug)) et Discord :

```
proxy (curl.exe) ──zip──► Worker ──► R2 (les zips, 50 Mo au plus)
                                  └─► webhook ──► forum Discord : un fil par rapport
```

- **`POST /rapport`** : le zip arrive dans le champ `archive` d'un formulaire multipart. Le Worker
  vérifie la limite par IP (3 rapports par minute), la taille (50 Mo au plus) et la signature du zip,
  le range dans R2 sous `rapports/BGE-XXXXXX.zip`, lit son `meta.json` et ouvre un fil dans le salon
  forum : titre `BGE-XXXXXX · joueur · tour N`, résumé de la partie, lien de téléchargement. Il rend
  `{"id": "BGE-XXXXXX"}`, que le jeu montre au joueur.
- **`GET /rapport/BGE-XXXXXX?cle=…`** : le zip. La clé, tirée au hasard pour chaque rapport, n'est
  écrite que dans le fil Discord. Sans elle, le Worker répond 404.
- **Pourquoi un relais** plutôt que le webhook dans la DLL : une URL de webhook gravée dans la DLL
  se lit avec `strings` et servirait à spammer le salon, et un webhook n'accepte que 10 Mo de pièces
  jointes, moins qu'une sauvegarde tardive avec ses journaux. Le Worker garde le webhook secret.
- Si Discord ne répond pas, le zip reste dans R2 et le joueur a son numéro : on le retrouve dans le
  bucket. L'erreur va aux journaux du Worker (`npx wrangler tail`).

Le texte écrit par le joueur (pseudo, civilisation…) passe sans mise en forme ni mention
(`allowed_mentions` vide) : un pseudo `@everyone` ne notifie personne.

## Mise en place

Une fois, avec un compte Cloudflare (l'offre gratuite suffit : R2 gratuit jusqu'à 10 Go, Workers
jusqu'à 100 000 requêtes par jour).

1. **Discord.** Créer un salon **forum** `#rapports-de-bug`, réservé à l'équipe. Dans ses paramètres :
   Intégrations > Webhooks > Nouveau webhook, puis copier son URL. Facultatif : créer un tag
   (« À trier ») et noter son ID (mode développeur, clic droit sur le tag) pour le poser sur
   chaque fil.
2. **Cloudflare.** Dans le tableau de bord : R2 > activer R2 (une carte est demandée, rien n'est
   facturé sous les seuils gratuits).
3. **Le Worker**, depuis ce dossier :

   ```bash
   npx wrangler login
   ```

   ```bash
   npx wrangler r2 bucket create bge-rapports
   ```

   ```bash
   npx wrangler secret put DISCORD_WEBHOOK
   ```

   (coller l'URL du webhook quand il la demande ; elle n'est écrite nulle part dans le dépôt)

   ```bash
   npx wrangler deploy
   ```

   Pour le tag du forum, décommenter `DISCORD_TAG` dans [wrangler.toml](wrangler.toml) avant de
   déployer.
4. **Essai**, avec un zip quelconque (le Worker exige seulement un zip) :

   ```bash
   curl -sS -F "archive=@essai.zip;type=application/zip" https://bge-rapports.<compte>.workers.dev/rapport
   ```

   Il doit répondre `{"id":"BGE-…"}`, et un fil doit apparaître dans le forum.
5. **Le jeu.** Dans le dépôt GitHub : Settings > Secrets and variables > Actions > Variables, créer
   `BGE_RAPPORT_URL` = `https://bge-rapports.<compte>.workers.dev/rapport`. Le workflow de release
   la grave dans `winmm.dll` à la prochaine release. Ce n'est pas un secret : n'importe qui peut la
   lire dans la DLL, la limite par IP et la taille maximale sont là pour ça.

   Pour essayer avant une release, avec une DLL déjà installée : ajouter
   `rapport_url=https://bge-rapports.<compte>.workers.dev/rapport` à `rollover_proxy.cfg`.

## Rétention

Les zips ne sont pas supprimés par le Worker. Pour les effacer après 90 jours, ajouter une règle
de cycle de vie au bucket : tableau de bord, R2 > bge-rapports > Settings > Object lifecycle rules.
Avec une version récente de wrangler, la même règle en ligne de commande (vérifier la syntaxe avec
`npx wrangler r2 bucket lifecycle add --help`) :

```bash
npx wrangler r2 bucket lifecycle add bge-rapports expiration-90j rapports/ --expire-days 90
```

## Tests

Hors Cloudflare, R2, la limite par IP et Discord simulés (Node 20 ou plus) :

```bash
node --test rapport_worker/test/index.test.js
```

Ils couvrent la lecture de `meta.json` dans le zip, le rangement, le fil Discord (titre, résumé,
tag, mentions coupées), le téléchargement avec et sans la clé, Discord en panne, et les refus
(limite, pas un zip, trop gros, mauvaise route). Le workflow de release les lance.

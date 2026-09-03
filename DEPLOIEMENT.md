# Mettre le jeu en ligne

Trois hébergeurs, chacun pour ce qu'il sait faire :

| Quoi | Où | Pourquoi celui-là |
|---|---|---|
| Interface Next.js | **Vercel** | fait pour Next.js, gratuit à cette échelle |
| Serveur de jeu Rust | **Render** | un processus qui tourne en continu et tient des WebSockets |
| Base Postgres | **Neon** | déjà en place |

## Pourquoi deux hébergeurs et pas un

Vercel exécute des fonctions : elles démarrent à la requête, répondent, et s'arrêtent. C'est parfait pour des pages, inutilisable pour une table de belote, qui doit **garder quatre connexions ouvertes** et posséder l'état de la partie en mémoire. Le serveur Rust a besoin d'un processus qui vit — c'est ce que Render fournit.

## Ce qui parle à quoi

```
Navigateur ──── https ────> Vercel ──── https ────> Render ──── > Neon
     │                    (relais /api)                (jeu)      (base)
     └──────────── wss (WebSocket, direct) ──────────────┘
```

Deux choses à retenir :

**Le HTTP passe par Vercel.** Le front n'appelle jamais Render directement : il appelle `/api/...` chez lui, et le serveur Next relaie ([next.config.ts](apps/web/next.config.ts)). Pour le navigateur, tout vient donc du même domaine.

**Le WebSocket va directement à Render.** Un relais ne sait pas transporter une connexion persistante. C'est la seule adresse à configurer en dur côté navigateur.

### La bonne surprise

Comme le navigateur ne parle qu'à Vercel en HTTP, **le cookie de session reste en première partie**. Pas de `SameSite=None`, pas de cookie tiers, donc rien que Safari ou un bloqueur puisse refuser — et **aucun nom de domaine à acheter**. C'était le principal piège annoncé au départ ; le relais mis en place pour faire marcher les téléphones l'a supprimé au passage.

---

## 1. Le serveur de jeu, sur Render

Les *Blueprints* (création automatique depuis [render.yaml](render.yaml)) sont
réservés aux comptes payants. On crée donc le service à la main : c'est le même
résultat, le fichier ne fait que pré-remplir ce formulaire.

**New** → **Web Service** → connecter le dépôt `belote`.

| Champ | Valeur |
|---|---|
| Name | `belote-api` |
| Language | **Rust** |
| Branch | `main` |
| Root Directory | *laisser vide* — le serveur est à la racine du workspace Cargo |
| Build Command | `cargo build --release -p belote-server` |
| Start Command | `./target/release/belote-server` |
| Region | Frankfurt — là où se trouve déjà la base |
| Health Check Path | `/health` *(section Advanced)* |

Puis quatre variables d'environnement :

| Nom | Valeur |
|---|---|
| `DATABASE_URL` | la chaîne Neon, **sans** `&channel_binding=require` — la bibliothèque Rust ne connaît pas ce paramètre. Garder `?sslmode=require`. |
| `JWT_SECRET` | cliquer sur **Generate** : Render fabrique une valeur aléatoire, meilleure que tout ce qu'on choisirait à la main |
| `SECURE_COOKIES` | `true` |
| `ALLOWED_ORIGIN` | l'adresse Vercel, connue seulement après l'étape 2 — mettre `https://exemple.vercel.app` et corriger ensuite |

Ne pas définir `PORT` : Render l'impose lui-même, et le serveur le lit.

Déployer. La première compilation Rust prend 5 à 10 minutes.
Vérifier ensuite que `https://belote-api.onrender.com/health` répond `ok`.

Les migrations s'appliquent toutes seules au démarrage — rien à lancer à la main.

## 2. L'interface, sur Vercel

1. Sur [vercel.com](https://vercel.com) → **Add New** → **Project**, choisir le dépôt.
2. **Root Directory : `apps/web`.** C'est le réglage à ne pas rater : sans lui,
   Vercel cherche un projet Next.js à la racine et ne trouve que du Rust.
3. Deux variables d'environnement :

   | Nom | Valeur | Qui la lit |
   |---|---|---|
   | `BACKEND_URL` | `https://belote-api.onrender.com` | le serveur Next, jamais le navigateur |
   | `NEXT_PUBLIC_WS_URL` | `wss://belote-api.onrender.com` | le navigateur, pour le WebSocket |

   `wss://` et non `ws://` : en https, un navigateur refuse une connexion non chiffrée.

4. Déployer, puis retourner sur Render corriger `ALLOWED_ORIGIN` avec la vraie
   adresse Vercel.

## 3. Vérifier

```bash
# Le serveur répond
curl https://belote-api.onrender.com/health

# Le parcours complet, vu depuis le front en production
API_URL=https://belote.vercel.app node scripts/api-check.mjs
```

Puis, à la main : créer un compte, lancer une partie solo, jouer une donne.
Idéalement à deux appareils sur deux réseaux différents, dont un en 4G.

---

## Ce qui coûte

| | Plan | Prix |
|---|---|---|
| Vercel | Hobby | gratuit |
| Neon | Free | gratuit |
| Render | Free, ou Starter | gratuit, ou ~7 $/mois |

Le plan gratuit de Render endort le service après 15 minutes sans trafic. Concrètement :

- la première personne à ouvrir le jeu attend une **cinquantaine de secondes**, le temps du réveil ;
- une partie en cours au moment de l'endormissement est **perdue** — l'état des tables vit en mémoire. Le journal, lui, est en base : la partie apparaît simplement comme interrompue dans l'historique ;
- une fois réveillé, il reste éveillé tant qu'on joue.

C'est acceptable pour essayer et montrer le projet. Le passage au plan Starter s'impose le jour où vous jouez régulièrement à plusieurs.

## Points de vigilance

**Une seule instance de serveur.** L'état des tables et les tickets WebSocket vivent en mémoire. Passer à deux instances casserait le jeu : deux joueurs pourraient atterrir sur des serveurs différents. Ne pas activer l'autoscaling sans avoir d'abord déplacé cet état.

**Les parties survivent aux redéploiements, pas les tables.** Le journal est en base, mais une table en cours vit en mémoire : un redéploiement coupe les parties. À faire quand personne ne joue.

**Le `JWT_SECRET` change si on le régénère.** Toutes les sessions ouvertes deviennent invalides et chacun doit se reconnecter.

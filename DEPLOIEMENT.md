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

1. Sur [render.com](https://render.com) → **New** → **Blueprint**, choisir le dépôt.
   Render lit [render.yaml](render.yaml) et propose le service `belote-api`.
2. Renseigner les deux variables laissées vides :
   - `DATABASE_URL` — la chaîne Neon, **sans** `&channel_binding=require` (la
     bibliothèque Rust ne connaît pas ce paramètre) :
     ```
     postgresql://…@….neon.tech/belote?sslmode=require
     ```
   - `ALLOWED_ORIGIN` — l'adresse Vercel, connue seulement après l'étape 2.
     Mettre `https://exemple.vercel.app` pour l'instant, on corrigera.
3. Déployer. La première compilation Rust prend 5 à 10 minutes.
4. Vérifier : `https://belote-api.onrender.com/health` doit répondre `ok`.

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
| Render | **Starter** | ~7 $/mois |

Le plan gratuit de Render endort le service après 15 minutes sans trafic : la partie en cours est coupée et le réveil prend une cinquantaine de secondes. Acceptable pour montrer le projet, pas pour y jouer.

## Points de vigilance

**Une seule instance de serveur.** L'état des tables et les tickets WebSocket vivent en mémoire. Passer à deux instances casserait le jeu : deux joueurs pourraient atterrir sur des serveurs différents. Ne pas activer l'autoscaling sans avoir d'abord déplacé cet état.

**Les parties survivent aux redéploiements, pas les tables.** Le journal est en base, mais une table en cours vit en mémoire : un redéploiement coupe les parties. À faire quand personne ne joue.

**Le `JWT_SECRET` change si on le régénère.** Toutes les sessions ouvertes deviennent invalides et chacun doit se reconnecter.

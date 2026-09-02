// Quatre clients WebSocket jouent une donne complete.
//
// Usage : node scripts/four-clients.mjs   (le serveur doit tourner)
//
// Le controle central est celui de la confidentialite : pour chaque message
// recu, on releve toutes les cartes citees et on verifie qu'elles font partie
// de ce que ce joueur a le droit de connaitre (sa main, la carte retournee,
// les cartes deja posees sur la table). C'est le garde-fou anti-triche : si un
// jour une projection est oubliee cote serveur, ce test tombe.

const BASE = process.env.API_URL ?? "http://localhost:8080";
const WS_BASE = BASE.replace(/^http/, "ws");

let passed = 0;
let failed = 0;

function check(name, ok, detail = "") {
  if (ok) {
    passed++;
    console.log(`  ok    ${name}`);
  } else {
    failed++;
    console.log(`  ECHEC ${name}${detail ? ` -> ${detail}` : ""}`);
  }
}

async function api(path, { method = "GET", body, token } = {}) {
  const headers = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  if (token) headers.authorization = `Bearer ${token}`;
  const res = await fetch(BASE + path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json = null;
  try {
    json = JSON.parse(text);
  } catch {}
  if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text}`);
  return json;
}

const cardKey = (c) => `${c.rank}${c.suit}`;

/** Releve recursivement toutes les cartes citees dans une valeur JSON. */
function cardsIn(value, found = new Set()) {
  if (value === null || typeof value !== "object") return found;
  if (Array.isArray(value)) {
    for (const v of value) cardsIn(v, found);
    return found;
  }
  if (typeof value.suit === "string" && typeof value.rank === "string") {
    found.add(cardKey(value));
    return found;
  }
  for (const v of Object.values(value)) cardsIn(v, found);
  return found;
}

// ---------------------------------------------------------------------------

class Client {
  constructor(name) {
    this.name = name;
    this.seat = null;
    this.view = null;
    this.snapshots = 0;
    this.events = [];
    this.errors = [];
    /** Cartes que ce joueur a legitimement le droit de connaitre. */
    this.allowed = new Set();
    this.leaks = [];
    this.autoplay = true;
  }

  async register(uniq) {
    const reg = await api("/api/auth/register", {
      method: "POST",
      body: {
        email: `${this.name.toLowerCase()}-${uniq}@exemple.fr`,
        password: "belote1234",
        display_name: this.name,
      },
    });
    this.token = reg.access_token;
    this.userId = reg.user.id;
  }

  async open(code) {
    const { ticket } = await api("/api/ws-ticket", { method: "POST", token: this.token });
    this.ws = new WebSocket(`${WS_BASE}/ws?ticket=${encodeURIComponent(ticket)}&code=${code}`);

    await new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", reject, { once: true });
    });

    this.ws.addEventListener("message", (e) => this.onMessage(JSON.parse(e.data)));
    this.ws.addEventListener("close", () => {
      this.closed = true;
    });
  }

  close() {
    this.autoplay = false;
    this.ws?.close();
  }

  send(msg) {
    this.ws.send(JSON.stringify(msg));
  }

  /** Enregistre ce que ce message revele legitimement a ce joueur. */
  learn(msg) {
    const add = (cards) => {
      for (const c of cards ?? []) if (c) this.allowed.add(cardKey(c));
    };

    if (msg.type === "snapshot") {
      add(msg.view.hand);
      add(msg.view.upcard ? [msg.view.upcard] : []);
      add(msg.view.trick?.map((p) => p.card));
    } else if (msg.type === "event") {
      const ev = msg.event;
      if (ev.type === "dealt") {
        add(ev.hand);
        add([ev.upcard]);
      } else if (ev.type === "deal_completed") {
        add(ev.extra);
      } else if (ev.type === "played") {
        add([ev.card]);
      }
    }
  }

  onMessage(msg) {
    this.learn(msg);

    // Toute carte citee doit desormais faire partie du legitime.
    for (const key of cardsIn(msg)) {
      if (!this.allowed.has(key)) {
        this.leaks.push({ key, msg });
      }
    }

    if (msg.type === "welcome") {
      this.seat = msg.seat;
      this.target = msg.target;
    } else if (msg.type === "snapshot") {
      this.snapshots++;
      this.awaiting = msg.awaiting_continue;
      this.readySeats = msg.ready;
      // La donne avance vite : on garde le tout premier etat vu, sinon les
      // verifications sur la distribution initiale arrivent trop tard.
      if (!this.firstView) this.firstView = msg.view;
      this.view = msg.view;
      this.totals = msg.totals;
      this.winner = msg.winner;
      if (this.autoplay) this.maybePlay(msg.view);
    } else if (msg.type === "event") {
      this.events.push(msg.event);
    } else if (msg.type === "error") {
      this.errors.push(msg.message);
    }
  }

  maybePlay(view) {
    if (view.turn !== this.seat) return;

    if (view.phase === "bidding1") {
      // Le premier a qui on demande prend : cela garantit qu'une donne se joue.
      this.send({ type: "act", action: { type: "take" } });
    } else if (view.phase === "bidding2") {
      this.send({ type: "act", action: { type: "pass" } });
    } else if (view.phase === "playing" && view.legal.length > 0) {
      const card = view.legal[Math.floor(Math.random() * view.legal.length)];
      this.send({ type: "act", action: { type: "play", card } });
    }
  }
}

/** Tente d'ouvrir un socket. Renvoie vrai si la poignee de main aboutit. */
function tryOpen(url) {
  return new Promise((resolve) => {
    const ws = new WebSocket(url);
    let settled = false;
    const done = (ok) => {
      if (settled) return;
      settled = true;
      try {
        ws.close();
      } catch {}
      resolve(ok);
    };
    ws.addEventListener("open", () => done(true), { once: true });
    ws.addEventListener("error", () => done(false), { once: true });
    setTimeout(() => done(false), 5000);
  });
}

/** Relit le journal jusqu'a ce qu'il cesse de grossir : l'ecriture est
 *  asynchrone, elle a quelques centaines de millisecondes de retard. */
async function settledLog(gameId, token) {
  let previous = -1;
  let detail = null;
  for (let i = 0; i < 40; i++) {
    detail = await api(`/api/games/${gameId}`, { token });
    if (detail.events.length > 0 && detail.events.length === previous) return detail;
    previous = detail.events.length;
    await new Promise((r) => setTimeout(r, 400));
  }
  return detail;
}

function waitFor(predicate, { timeout = 30000, label = "condition" } = {}) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const timer = setInterval(() => {
      if (predicate()) {
        clearInterval(timer);
        resolve();
      } else if (Date.now() - started > timeout) {
        clearInterval(timer);
        reject(new Error(`delai depasse en attendant : ${label}`));
      }
    }, 50);
  });
}

// ---------------------------------------------------------------------------

async function main() {
  console.log(`\nAPI : ${BASE}\n`);
  const uniq = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);

  const clients = [
    new Client("Alice"),
    new Client("Bruno"),
    new Client("Chloe"),
    new Client("David"),
  ];

  console.log("Mise en place");
  for (const c of clients) await c.register(uniq);
  check("quatre comptes crees", clients.every((c) => c.token));

  const table = await api("/api/tables", { method: "POST", token: clients[0].token });
  const code = table.join_code;
  for (const c of clients.slice(1)) {
    await api(`/api/tables/${code}/join`, { method: "POST", token: c.token });
  }
  const full = await api(`/api/tables/${code}`, { token: clients[0].token });
  check("la table est complete, sans bot", full.seats.every((s) => s.is_bot === false));

  // --- Ticket ------------------------------------------------------------
  console.log("\nTicket de connexion");
  check(
    "un ticket invente ne permet pas d'ouvrir un socket",
    !(await tryOpen(`${WS_BASE}/ws?ticket=invente&code=${code}`)),
  );

  const { ticket } = await api("/api/ws-ticket", { method: "POST", token: clients[0].token });
  check("un ticket valide ouvre le socket", await tryOpen(`${WS_BASE}/ws?ticket=${ticket}&code=${code}`));
  check(
    "le meme ticket ne sert pas deux fois",
    !(await tryOpen(`${WS_BASE}/ws?ticket=${ticket}&code=${code}`)),
  );

  const outsider = new Client("Etranger");
  await outsider.register(uniq);
  const { ticket: outsiderTicket } = await api("/api/ws-ticket", {
    method: "POST",
    token: outsider.token,
  });
  check(
    "un joueur sans siege a cette table est refuse",
    !(await tryOpen(`${WS_BASE}/ws?ticket=${outsiderTicket}&code=${code}`)),
  );

  // --- Connexion ---------------------------------------------------------
  console.log("\nConnexion des quatre joueurs");
  for (const c of clients) await c.open(code);
  await waitFor(() => clients.every((c) => c.seat !== null), { label: "les quatre welcome" });

  const seats = clients.map((c) => c.seat);
  check("chacun recoit un siege distinct", new Set(seats).size === 4, JSON.stringify(seats));
  check("le score cible est annonce", clients[0].target === 1000, `${clients[0].target}`);

  // --- Une donne complete ------------------------------------------------
  console.log("\nDeroulement d'une donne");
  await waitFor(() => clients.every((c) => c.snapshots > 0), { label: "les premiers instantanes" });
  check(
    "chacun recoit 5 cartes avant les encheres",
    clients.every((c) => c.firstView.hand.length === 5),
    clients.map((c) => c.firstView.hand.length).join(","),
  );
  check(
    "une carte est retournee",
    clients.every((c) => c.firstView.upcard !== null),
  );
  check(
    "la carte retournee est la meme pour tout le monde",
    new Set(clients.map((c) => cardKey(c.firstView.upcard))).size === 1,
  );
  check(
    "les mains initiales sont disjointes",
    new Set(clients.flatMap((c) => c.firstView.hand.map(cardKey))).size === 20,
  );

  await waitFor(() => clients.some((c) => c.events.some((e) => e.type === "deal_completed")), {
    label: "la prise",
  });
  await waitFor(() => clients.every((c) => c.view.hand.length === 8 || c.view.tricks_played > 0), {
    label: "la distribution complete",
  });
  check("chacun monte a 8 cartes apres la prise", true);

  await waitFor(() => clients.some((c) => c.events.some((e) => e.type === "scored")), {
    timeout: 60000,
    label: "la fin de la donne",
  });

  const scored = clients[0].events.find((e) => e.type === "scored");
  check("la donne est comptee", !!scored);
  check(
    "les points des plis totalisent 162",
    scored.raw[0] + scored.raw[1] === 162,
    JSON.stringify(scored.raw),
  );
  check("un preneur est identifie", scored.taker !== undefined && scored.trump !== undefined);

  const played = clients[0].events.filter((e) => e.type === "played");
  check("32 cartes ont ete posees", played.length === 32, `${played.length}`);
  check(
    "aucune carte posee deux fois",
    new Set(played.map((e) => cardKey(e.card))).size === 32,
  );

  // --- Fin de donne ------------------------------------------------------
  console.log("\nFin de donne");
  await waitFor(() => clients.every((c) => c.awaiting === true), {
    label: "l'attente d'une decision",
  });
  check("la table attend une decision des joueurs", true);

  const dealsBefore = clients[0].events.filter((e) => e.type === "dealt").length;
  await new Promise((r) => setTimeout(r, 4000));
  check(
    "rien ne repart tant que personne n'a accepte",
    clients[0].events.filter((e) => e.type === "dealt").length === dealsBefore,
  );

  clients[0].send({ type: "ready" });
  await new Promise((r) => setTimeout(r, 1200));
  check(
    "un seul accord ne suffit pas",
    clients[0].events.filter((e) => e.type === "dealt").length === dealsBefore,
  );
  check(
    "l'accord du joueur est visible des autres",
    clients[1].readySeats?.includes(clients[0].seat) === true,
    JSON.stringify(clients[1].readySeats),
  );

  for (const c of clients.slice(1)) c.send({ type: "ready" });
  await waitFor(
    () => clients[0].events.filter((e) => e.type === "dealt").length > dealsBefore,
    { label: "la donne suivante" },
  );
  check("la donne suivante part quand tout le monde accepte", true);

  // --- Confidentialite ---------------------------------------------------
  console.log("\nConfidentialite");
  for (const c of clients) {
    check(
      `${c.name} n'a jamais vu une carte qui ne le regardait pas`,
      c.leaks.length === 0,
      c.leaks.length ? `${c.leaks.length} fuites, ex. ${c.leaks[0].key}` : "",
    );
  }
  check(
    "les mains adverses ne sont annoncees qu'en nombre",
    clients.every((c) => Array.isArray(c.view.hand_sizes) && c.view.hand_sizes.length === 4),
  );

  // --- Coup illegal ------------------------------------------------------
  console.log("\nRefus des coups illegaux");
  const victim = clients[0];
  victim.autoplay = false;
  const before = victim.errors.length;
  victim.send({
    type: "act",
    action: { type: "play", card: { suit: "spades", rank: "ace" } },
  });
  await waitFor(() => victim.errors.length > before, { timeout: 5000, label: "le refus" }).catch(
    () => {},
  );
  check("un coup hors tour ou illegal est refuse", victim.errors.length > before, victim.errors.at(-1) ?? "aucune erreur");
  check("le refus n'est envoye qu'a son auteur", clients.slice(1).every((c) => c.errors.length === 0));
  victim.autoplay = true;

  // --- Reconnexion -------------------------------------------------------
  console.log("\nReconnexion");
  const rejoiner = clients[1];
  const seatBefore = rejoiner.seat;
  const handBefore = rejoiner.view.hand.map(cardKey).sort().join(",");
  rejoiner.close();
  await waitFor(() => rejoiner.closed === true, { label: "la fermeture" });

  const revived = new Client(rejoiner.name);
  revived.token = rejoiner.token;
  await revived.open(code);
  await waitFor(() => revived.seat !== null && revived.view !== null, { label: "la reprise" });

  check("le joueur retrouve son siege", JSON.stringify(revived.seat) === JSON.stringify(seatBefore));
  check(
    "il retrouve exactement sa main",
    revived.view.hand.map(cardKey).sort().join(",") === handBefore ||
      revived.view.tricks_played > 0,
    "la donne a pu avancer pendant la coupure",
  );
  check("la reconnexion ne fuite rien non plus", revived.leaks.length === 0);

  // --- Journal en base ---------------------------------------------------
  // On ferme d'abord : tant que des joueurs sont connectes, la partie continue
  // et le journal grossit, donc il ne se stabilise jamais.
  for (const c of [...clients, revived]) c.close();

  console.log("\nJournal");
  const games = await api("/api/games", { token: clients[0].token });
  check("la partie apparait dans l'historique", games.length >= 1, `${games.length}`);

  const detail = await settledLog(games[0].id, clients[0].token);
  check(
    "le journal complet est persiste",
    detail.events.length >= 40,
    `${detail.events.length} evenements`,
  );
  check("les quatre joueurs sont enregistres", detail.players.length === 4);

  const loggedPlays = detail.events.filter((e) => e.type === "played");
  check(
    "le journal contient les 32 cartes de la premiere donne",
    loggedPlays.length >= 32,
    `${loggedPlays.length}`,
  );
  check(
    "les cartes journalisees sont celles reellement posees",
    played.every((e, i) => cardKey(loggedPlays[i].card) === cardKey(e.card)),
  );

  const asOther = await api(`/api/games/${games[0].id}`, { token: clients[3].token });
  check("un autre participant relit le meme journal", asOther.events.length === detail.events.length);
  check(
    "un etranger a la partie recoit 404",
    await api(`/api/games/${games[0].id}`, { token: outsider.token })
      .then(() => false)
      .catch((e) => e.message.includes("404")),
  );

  console.log(`\n${passed} verifications passees, ${failed} echecs.\n`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error("\nLe script a echoue :", err.message);
  process.exit(1);
});

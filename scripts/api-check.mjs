// Verification bout en bout de l'API HTTP : comptes, session, salons.
//
// Usage : node scripts/api-check.mjs   (le serveur doit tourner)
//
// Ce script n'est pas un test unitaire : il exerce le serveur reel avec sa
// vraie base, et verifie surtout ce qu'un test unitaire ne voit pas, a savoir
// la rotation des jetons et l'attribution des sieges.

const BASE = process.env.API_URL ?? "http://localhost:8080";

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

async function req(path, { method = "GET", body, token, cookie } = {}) {
  const headers = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  if (token) headers.authorization = `Bearer ${token}`;
  if (cookie) headers.cookie = `belote_refresh=${cookie}`;

  const res = await fetch(BASE + path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  const text = await res.text();
  let json = null;
  try {
    json = JSON.parse(text);
  } catch {
    /* reponse non-JSON, par exemple /health */
  }
  return { status: res.status, json, text, setCookie: res.headers.getSetCookie() };
}

function refreshCookie(setCookie) {
  for (const raw of setCookie) {
    const m = /^belote_refresh=([^;]*)/.exec(raw);
    if (m) return m[1];
  }
  return null;
}

function cookieAttrs(setCookie) {
  return setCookie.find((c) => c.startsWith("belote_refresh=")) ?? "";
}

const uniq = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);

async function main() {
  console.log(`\nAPI : ${BASE}\n`);

  // --- Sonde -------------------------------------------------------------
  console.log("Sante");
  const health = await req("/health");
  check("GET /health repond 200 ok", health.status === 200 && health.text === "ok", health.text);

  // --- Inscription -------------------------------------------------------
  console.log("\nInscription");
  const emailA = `alice-${uniq}@exemple.fr`;
  const reg = await req("/api/auth/register", {
    method: "POST",
    body: { email: emailA, password: "belote1234", display_name: "Alice" },
  });
  check("inscription acceptee", reg.status === 200, `${reg.status} ${reg.text}`);
  check("un jeton d'acces est renvoye", !!reg.json?.access_token);
  check("le pseudo est conserve", reg.json?.user?.display_name === "Alice");

  const attrs = cookieAttrs(reg.setCookie);
  check("le cookie de refresh est pose", attrs.length > 0);
  check("le cookie est HttpOnly", /HttpOnly/i.test(attrs), attrs);
  check("le cookie est limite au chemin /", /Path=\//i.test(attrs), attrs);
  check(
    "le jeton de refresh n'apparait pas dans le corps JSON",
    !reg.text.includes(refreshCookie(reg.setCookie) ?? "###"),
  );

  const dup = await req("/api/auth/register", {
    method: "POST",
    body: { email: emailA.toUpperCase(), password: "belote1234", display_name: "Sosie" },
  });
  check("une adresse deja prise est refusee, casse comprise", dup.status === 409, `${dup.status}`);

  const weak = await req("/api/auth/register", {
    method: "POST",
    body: { email: `faible-${uniq}@exemple.fr`, password: "court", display_name: "Bob" },
  });
  check("un mot de passe trop court est refuse", weak.status === 400, `${weak.status}`);

  // --- Jeton d'acces -----------------------------------------------------
  console.log("\nJeton d'acces");
  const accessA = reg.json.access_token;
  const me = await req("/api/me", { token: accessA });
  check("GET /api/me renvoie le bon compte", me.json?.email === emailA, me.text);

  const anon = await req("/api/me");
  check("GET /api/me sans jeton renvoie 401", anon.status === 401, `${anon.status}`);

  const tampered = accessA.slice(0, -3) + "aaa";
  const forged = await req("/api/me", { token: tampered });
  check("un jeton falsifie est rejete", forged.status === 401, `${forged.status}`);

  // --- Connexion ---------------------------------------------------------
  console.log("\nConnexion");
  const badPass = await req("/api/auth/login", {
    method: "POST",
    body: { email: emailA, password: "mauvais-mot-de-passe" },
  });
  check("mauvais mot de passe : 401", badPass.status === 401, `${badPass.status}`);

  const unknown = await req("/api/auth/login", {
    method: "POST",
    body: { email: `fantome-${uniq}@exemple.fr`, password: "belote1234" },
  });
  check(
    "compte inexistant : reponse identique a un mauvais mot de passe",
    unknown.status === badPass.status && unknown.text === badPass.text,
    `${unknown.status} ${unknown.text}`,
  );

  const login = await req("/api/auth/login", {
    method: "POST",
    body: { email: emailA, password: "belote1234" },
  });
  check("connexion valide : 200", login.status === 200, `${login.status}`);

  // --- Rotation du refresh ----------------------------------------------
  console.log("\nRotation du jeton de rafraichissement");
  const refresh1 = refreshCookie(login.setCookie);
  const rot = await req("/api/auth/refresh", { method: "POST", cookie: refresh1 });
  check(
    "le refresh renvoie un nouveau jeton d'acces",
    rot.status === 200 && !!rot.json?.access_token,
    `${rot.status}`,
  );

  const refresh2 = refreshCookie(rot.setCookie);
  check("le jeton de refresh a bien tourne", !!refresh2 && refresh2 !== refresh1);

  const replay = await req("/api/auth/refresh", { method: "POST", cookie: refresh1 });
  check("rejouer l'ancien jeton de refresh est refuse", replay.status === 401, `${replay.status}`);

  const bogus = await req("/api/auth/refresh", { method: "POST", cookie: "jeton-invente" });
  check("un jeton de refresh invente est refuse", bogus.status === 401, `${bogus.status}`);

  // --- Salons ------------------------------------------------------------
  console.log("\nSalons");
  const table = await req("/api/tables", { method: "POST", token: accessA });
  check("creation d'une table : 200", table.status === 200, `${table.status} ${table.text}`);

  const code = table.json?.join_code;
  check("un code de salon a 6 caracteres est renvoye", code?.length === 6, code);
  check(
    "le code evite les caracteres ambigus",
    !!code && ![..."O0I1L"].some((ch) => code.includes(ch)),
    code,
  );
  check("la table a exactement 4 sieges", table.json?.seats?.length === 4);
  check("le createur occupe le siege 0", table.json?.seats?.[0]?.is_bot === false);
  check(
    "les trois autres sieges sont des bots",
    table.json?.seats?.slice(1).every((s) => s.is_bot === true),
  );

  const anonTable = await req("/api/tables", { method: "POST" });
  check("creer une table sans etre connecte : 401", anonTable.status === 401, `${anonTable.status}`);

  // Un deuxieme joueur rejoint.
  const regB = await req("/api/auth/register", {
    method: "POST",
    body: { email: `bruno-${uniq}@exemple.fr`, password: "belote1234", display_name: "Bruno" },
  });
  const accessB = regB.json.access_token;

  const join = await req(`/api/tables/${code}/join`, { method: "POST", token: accessB });
  check("un ami rejoint la table : 200", join.status === 200, `${join.status} ${join.text}`);

  const seatB = join.json?.seats?.find((s) => s.display_name === "Bruno");
  check("il prend le premier siege libre", seatB?.seat === 1, JSON.stringify(seatB));
  check("son siege n'est plus tenu par un bot", seatB?.is_bot === false);

  const rejoin = await req(`/api/tables/${code}/join`, { method: "POST", token: accessB });
  const seatB2 = rejoin.json?.seats?.find((s) => s.display_name === "Bruno");
  check("rejoindre deux fois est idempotent", rejoin.status === 200 && seatB2?.seat === 1);
  check(
    "il n'occupe qu'un seul siege",
    rejoin.json?.seats?.filter((s) => s.display_name === "Bruno").length === 1,
  );

  const missing = await req("/api/tables/ZZZZZZ/join", { method: "POST", token: accessB });
  check("rejoindre un code inconnu : 404", missing.status === 404, `${missing.status}`);

  const lower = await req(`/api/tables/${code.toLowerCase()}`, { token: accessB });
  check("le code est insensible a la casse", lower.status === 200, `${lower.status}`);

  // --- Historique --------------------------------------------------------
  console.log("\nHistorique");
  const games = await req("/api/games", { token: accessA });
  check("l'historique repond 200", games.status === 200, `${games.status}`);
  check("aucune partie jouee pour l'instant", Array.isArray(games.json) && games.json.length === 0);

  const foreign = await req("/api/games/00000000-0000-0000-0000-000000000000", { token: accessA });
  check("une partie d'autrui renvoie 404 et non 403", foreign.status === 404, `${foreign.status}`);

  // --- Deconnexion -------------------------------------------------------
  console.log("\nDeconnexion");
  const logout = await req("/api/auth/logout", { method: "POST", cookie: refresh2 });
  check("deconnexion : 200", logout.status === 200, `${logout.status}`);

  const afterLogout = await req("/api/auth/refresh", { method: "POST", cookie: refresh2 });
  check(
    "le jeton de refresh est invalide apres deconnexion",
    afterLogout.status === 401,
    `${afterLogout.status}`,
  );

  // --- Bilan -------------------------------------------------------------
  console.log(`\n${passed} verifications passees, ${failed} echecs.\n`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error("\nLe script a echoue :", err);
  process.exit(1);
});

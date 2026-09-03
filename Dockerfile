# Image du serveur de jeu.
#
# Le depot contient un package.json a la racine, pour l'espace de travail npm :
# les hebergeurs qui devinent le langage y voient un projet Node et ne trouvent
# pas cargo. Cette image leve l'ambiguite — et rend le deploiement portable,
# vers Fly.io ou Railway aussi bien que Render.

# --- Compilation -----------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# L'image « slim » fournit gcc, mais pas pkg-config, dont certaines
# dependances natives ont besoin pour se localiser.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Seul le code Rust est necessaire : le front est bati ailleurs, par Vercel.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Les migrations sont embarquees dans le binaire a la compilation
# (sqlx::migrate!), il n'y a donc rien a copier dans l'image finale.
RUN cargo build --release -p belote-server

# --- Execution -------------------------------------------------------------
FROM debian:bookworm-slim

# Les certificats racines : sans eux, la connexion TLS a Postgres echoue.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/belote-server /usr/local/bin/belote-server

# L'hebergeur impose son propre PORT ; celui-ci n'est qu'un defaut lisible.
ENV PORT=8080
EXPOSE 8080

# Pas de shell entre l'hebergeur et le serveur : les signaux d'arret lui
# parviennent directement, et il se termine proprement.
CMD ["belote-server"]

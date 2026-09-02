-- Schema initial : comptes, tables de jeu, et journal d'evenements.
--
-- `game_events` est append-only : c'est lui l'historique complet. On ne stocke
-- jamais un etat de partie mutable, on rejoue les evenements.

create table users (
    id            uuid primary key,
    email         text        not null,
    password_hash text        not null,
    display_name  text        not null,
    created_at    timestamptz not null default now()
);

-- Unicite insensible a la casse sans dependre de l'extension citext, qui n'est
-- pas disponible partout.
create unique index users_email_key on users (lower(email));

create table refresh_tokens (
    id         uuid primary key,
    user_id    uuid        not null references users (id) on delete cascade,
    token_hash bytea       not null unique,
    expires_at timestamptz not null,
    revoked_at timestamptz,
    created_at timestamptz not null default now()
);

create index refresh_tokens_user_idx on refresh_tokens (user_id);

create table game_tables (
    id         uuid primary key,
    join_code  text        not null unique,
    owner_id   uuid        not null references users (id) on delete cascade,
    status     text        not null default 'lobby',
    created_at timestamptz not null default now()
);

create table table_seats (
    table_id uuid     not null references game_tables (id) on delete cascade,
    seat     smallint not null check (seat between 0 and 3),
    user_id  uuid references users (id) on delete set null,
    is_bot   boolean  not null default true,
    primary key (table_id, seat)
);

-- Un joueur ne peut occuper qu'un seul siege a une table donnee.
create unique index table_seats_one_seat_per_user
    on table_seats (table_id, user_id)
    where user_id is not null;

create table games (
    id           uuid primary key,
    table_id     uuid        not null references game_tables (id) on delete cascade,
    started_at   timestamptz not null default now(),
    ended_at     timestamptz,
    final_scores jsonb
);

create index games_table_idx on games (table_id, started_at desc);

-- Qui jouait, et a quelle place. Permet de retrouver l'historique d'un joueur.
create table game_players (
    game_id uuid     not null references games (id) on delete cascade,
    seat    smallint not null check (seat between 0 and 3),
    user_id uuid references users (id) on delete set null,
    is_bot  boolean  not null,
    primary key (game_id, seat)
);

create index game_players_user_idx on game_players (user_id);

-- Le journal. `seq` garantit l'ordre et rend les reemissions idempotentes.
create table game_events (
    id         bigserial primary key,
    game_id    uuid        not null references games (id) on delete cascade,
    seq        integer     not null,
    payload    jsonb       not null,
    created_at timestamptz not null default now(),
    unique (game_id, seq)
);

create index game_events_game_idx on game_events (game_id, seq);

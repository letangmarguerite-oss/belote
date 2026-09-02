-- Distingue les deux intentions a la creation d'une table.
--
-- Vrai  : partie solo contre des bots, on commence des l'arrivee du joueur.
-- Faux  : table entre amis, elle attend que le proprietaire la lance, ce qui
--         laisse le temps de partager le code.
alter table game_tables
    add column autostart boolean not null default true;

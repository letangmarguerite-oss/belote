-- Objectif de points du match, choisi a la creation de la table.
-- 1000 est la valeur usuelle ; certains preferent des parties courtes.
alter table game_tables
    add column target_points integer not null default 1000;

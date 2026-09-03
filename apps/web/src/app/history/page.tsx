"use client";

import { useEffect, useState } from "react";

import { Shell } from "@/components/Shell";
import { api } from "@/lib/api";
import type { GameSummary, Stats } from "@/lib/types";

export default function HistoryPage() {
  return (
    <Shell requireAuth>
      <main className="mx-auto w-full max-w-2xl flex-1 px-5 pb-10">
        <h1 className="font-display text-2xl text-bone">Parties jouées</h1>
        <StatsPanel />
        <GameList />
      </main>
    </Shell>
  );
}

/** Chiffres deduits du journal des parties, jamais de compteurs stockes. */
function StatsPanel() {
  const [stats, setStats] = useState<Stats | null>(null);

  useEffect(() => {
    void api<Stats>("/api/stats")
      .then(setStats)
      .catch(() => setStats(null));
  }, []);

  if (!stats || stats.deals_played === 0) return null;

  const takeRate =
    stats.deals_taken > 0
      ? Math.round((100 * stats.deals_taken_made) / stats.deals_taken)
      : null;

  return (
    <div className="panel mt-5 grid grid-cols-2 gap-x-4 gap-y-3 p-4 sm:grid-cols-4">
      <Figure label="Parties gagnées" value={`${stats.games_won}`} hint={`sur ${stats.games_finished}`} />
      <Figure label="Donnes jouées" value={`${stats.deals_played}`} />
      <Figure
        label="Contrats tenus"
        value={takeRate === null ? "—" : `${takeRate} %`}
        hint={stats.deals_taken > 0 ? `${stats.deals_taken} prises` : "jamais pris"}
      />
      <Figure
        label="Meilleure donne"
        value={`${stats.best_deal}`}
        hint={
          stats.belotes + stats.capots > 0
            ? [
                stats.belotes > 0 ? `${stats.belotes} belote${stats.belotes > 1 ? "s" : ""}` : null,
                stats.capots > 0 ? `${stats.capots} capot${stats.capots > 1 ? "s" : ""}` : null,
              ]
                .filter(Boolean)
                .join(" · ")
            : undefined
        }
      />
    </div>
  );
}

function Figure({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div>
      <p className="font-display text-2xl text-gold">{value}</p>
      <p className="text-xs text-bone">{label}</p>
      {hint && <p className="text-[0.7rem] text-bone-dim">{hint}</p>}
    </div>
  );
}

function GameList() {
  const [games, setGames] = useState<GameSummary[] | null>(null);

  useEffect(() => {
    void api<GameSummary[]>("/api/games?limit=30")
      .then(setGames)
      .catch(() => setGames([]));
  }, []);

  if (games === null) {
    return <p className="mt-6 text-bone-dim">Chargement…</p>;
  }

  if (games.length === 0) {
    return (
      <p className="mt-6 text-bone-dim">
        Aucune partie pour l&apos;instant. Créez une table pour commencer.
      </p>
    );
  }

  return (
    <ul className="mt-6 flex flex-col gap-2">
      {games.map((game) => {
        const myTeam = game.seat % 2;
        const totals = game.final_scores?.totals;
        const closed = !!game.ended_at;
        // Une partie close sans vainqueur a ete abandonnee en cours de route :
        // le score reste affiche, mais ce n'est ni une victoire ni une defaite.
        const completed = closed && game.final_scores?.completed === true;
        const won = !!totals && totals[myTeam] > totals[1 - myTeam];

        let label: string;
        if (!closed) label = "En cours";
        else if (completed) label = won ? "Victoire" : "Défaite";
        else if (totals && (totals[0] > 0 || totals[1] > 0))
          label = "Interrompue";
        else label = "Sans donne jouée";

        return (
          <li
            key={game.id}
            className="panel flex items-center justify-between gap-3 px-4 py-3"
          >
            <div className="min-w-0">
              <p className="text-sm text-bone">
                {new Date(game.started_at).toLocaleString("fr-FR", {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </p>
              <p className="text-xs text-bone-dim">{label}</p>
            </div>
            {totals && (
              <p className="shrink-0 font-display text-lg">
                <span className={completed && won ? "text-gold" : "text-bone"}>
                  {totals[myTeam]}
                </span>
                <span className="text-bone-dim"> — {totals[1 - myTeam]}</span>
              </p>
            )}
          </li>
        );
      })}
    </ul>
  );
}

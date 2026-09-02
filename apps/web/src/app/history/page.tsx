"use client";

import { useEffect, useState } from "react";

import { Shell } from "@/components/Shell";
import { api } from "@/lib/api";
import type { GameSummary } from "@/lib/types";

export default function HistoryPage() {
  return (
    <Shell requireAuth>
      <main className="mx-auto w-full max-w-2xl flex-1 px-5 pb-10">
        <h1 className="font-display text-2xl text-bone">Parties jouées</h1>
        <GameList />
      </main>
    </Shell>
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
        const finished = !!game.ended_at && !!totals;
        const won = finished && totals[myTeam] > totals[1 - myTeam];

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
              <p className="text-xs text-bone-dim">
                {finished ? (won ? "Victoire" : "Défaite") : "Partie en cours"}
              </p>
            </div>
            {totals && (
              <p className="shrink-0 font-display text-lg">
                <span className={won ? "text-gold" : "text-bone"}>
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

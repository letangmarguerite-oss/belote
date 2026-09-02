"use client";

import { useState } from "react";

import type { SeatInfo } from "@/lib/types";

/**
 * Salon d'attente.
 *
 * Le code reste affiche en grand tant que la partie n'est pas lancee : c'est
 * le seul moment ou il sert, et il n'a aucune raison de disparaitre avant que
 * tout le monde soit entre.
 */
export function Lobby({
  code,
  seats,
  canStart,
  onStart,
  onLeave,
}: {
  code: string;
  seats: SeatInfo[];
  canStart: boolean;
  onStart: () => void;
  onLeave: () => void;
}) {
  const [copied, setCopied] = useState<"link" | "code" | null>(null);

  const copy = async (what: "link" | "code") => {
    const text =
      what === "link" ? `${window.location.origin}/table/${code}` : code;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      // Le presse-papiers peut etre refuse : le code reste lisible a l'ecran.
    }
  };

  const humans = seats.filter((s) => !s.is_bot);
  const bots = seats.length - humans.length;

  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center gap-6 px-5 pb-10">
      <div className="text-center">
        <p className="text-sm text-bone-dim">Code de la table</p>
        <button
          type="button"
          onClick={() => copy("code")}
          className="panel mt-2 w-full px-4 py-5 font-display text-4xl tracking-[0.35em] text-gold sm:text-5xl"
          aria-label={`Copier le code ${code}`}
        >
          {copied === "code" ? "Copié !" : code}
        </button>
        <button
          type="button"
          onClick={() => copy("link")}
          className="mt-3 text-sm text-bone-dim underline-offset-2 hover:text-bone hover:underline"
        >
          {copied === "link" ? "Lien copié !" : "Copier le lien d'invitation"}
        </button>
      </div>

      <ul className="panel flex flex-col divide-y divide-bone/10 p-1">
        {seats.map((seat) => (
          <li
            key={seat.seat}
            className="flex items-center justify-between px-3 py-2.5 text-sm"
          >
            <span className="flex items-center gap-2">
              <span
                className={`inline-block size-1.5 rounded-full ${
                  seat.is_bot
                    ? "bg-bone-dim/50"
                    : seat.connected
                      ? "bg-emerald-400"
                      : "bg-gold/50"
                }`}
              />
              <span className={seat.is_bot ? "text-bone-dim" : "text-bone"}>
                {seat.display_name}
              </span>
            </span>
            <span className="text-xs text-bone-dim">
              {seat.is_bot
                ? "bot"
                : seat.connected
                  ? "prêt"
                  : "en attente"}
            </span>
          </li>
        ))}
      </ul>

      <div className="flex flex-col gap-2">
        {canStart ? (
          <button type="button" className="btn btn-gold w-full" onClick={onStart}>
            Commencer la partie
          </button>
        ) : (
          <p className="text-center text-sm text-bone-dim">
            En attente que l&apos;hôte lance la partie…
          </p>
        )}
        <button type="button" className="btn btn-ghost w-full" onClick={onLeave}>
          Quitter la table
        </button>
      </div>

      {canStart && bots > 0 && (
        <p className="text-center text-xs text-bone-dim">
          {bots === 1
            ? "Un siège sera tenu par un bot."
            : `${bots} sièges seront tenus par des bots.`}{" "}
          Attendez vos amis avant de lancer, ou commencez tout de suite.
        </p>
      )}
    </main>
  );
}

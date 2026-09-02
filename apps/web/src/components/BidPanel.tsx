"use client";

import { PlayingCard } from "./PlayingCard";
import { SUIT_LABEL, SUIT_SYMBOL, isRed } from "@/lib/cards";
import type { Action, PlayerView, Suit } from "@/lib/types";

interface Props {
  view: PlayerView;
  onAct: (action: Action) => void;
}

/** Les encheres : prendre la retournee au premier tour, nommer une couleur au second. */
export function BidPanel({ view, onAct }: Props) {
  const myTurn = view.turn === view.seat;

  if (view.phase !== "bidding1" && view.phase !== "bidding2") return null;

  if (!myTurn) {
    return (
      <p className="text-center text-sm text-bone-dim">
        {view.phase === "bidding1"
          ? "Premier tour d'encheres…"
          : "Second tour d'encheres…"}
      </p>
    );
  }

  if (view.phase === "bidding1") {
    return (
      <div className="panel flex flex-col items-center gap-3 p-4">
        <div className="flex items-center gap-3">
          {view.upcard && (
            <div className="w-12" style={{ aspectRatio: "100 / 150" }}>
              <PlayingCard card={view.upcard} />
            </div>
          )}
          <p className="text-sm">
            Prendre a{" "}
            <span className="font-semibold text-gold">
              {view.upcard ? SUIT_LABEL[view.upcard.suit].toLowerCase() : ""}
            </span>{" "}
            ?
          </p>
        </div>
        <div className="flex gap-2">
          <button type="button" className="btn btn-gold" onClick={() => onAct({ type: "take" })}>
            Je prends
          </button>
          <button type="button" className="btn btn-ghost" onClick={() => onAct({ type: "pass" })}>
            Je passe
          </button>
        </div>
      </div>
    );
  }

  // Second tour : toute couleur sauf celle de la retournee.
  const choices = (["spades", "hearts", "diamonds", "clubs"] as Suit[]).filter(
    (suit) => suit !== view.upcard?.suit,
  );

  return (
    <div className="panel flex flex-col items-center gap-3 p-4">
      <p className="text-sm text-bone-dim">Choisir l&apos;atout, ou passer</p>
      <div className="flex flex-wrap justify-center gap-2">
        {choices.map((suit) => (
          <button
            key={suit}
            type="button"
            className="btn btn-ghost"
            onClick={() => onAct({ type: "choose_trump", suit })}
          >
            <span
              className="text-lg"
              style={{ color: isRed(suit) ? "var(--color-ruby)" : "var(--color-bone)" }}
            >
              {SUIT_SYMBOL[suit]}
            </span>
            {SUIT_LABEL[suit]}
          </button>
        ))}
        <button type="button" className="btn btn-ghost" onClick={() => onAct({ type: "pass" })}>
          Passer
        </button>
      </div>
    </div>
  );
}

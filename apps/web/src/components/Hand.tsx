"use client";

import { useState } from "react";

import { PlayingCard } from "./PlayingCard";
import { cardKey, hasCard, sameCard, sortHand } from "@/lib/cards";
import type { Card, Suit } from "@/lib/types";

interface Props {
  hand: Card[];
  legal: Card[];
  myTurn: boolean;
  trump: Suit | null;
  onPlay: (card: Card) => void;
}

/**
 * La main, en eventail.
 *
 * Une carte se joue en deux temps : un premier appui la souleve, un second la
 * pose. Sur telephone, ou les cartes se chevauchent, un seul appui ferait
 * jouer la mauvaise carte trop souvent pour un jeu ou l'on ne peut pas revenir
 * en arriere.
 */
export function Hand({ hand, legal, myTurn, trump, onPlay }: Props) {
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const cards = sortHand(hand, trump);

  // La selection se deduit du rendu plutot que de se synchroniser dans un
  // effet : des que la carte quitte la main ou que le tour passe, elle
  // s'annule d'elle-meme, sans rendu supplementaire.
  const selected =
    (myTurn && cards.find((card) => cardKey(card) === selectedKey)) || null;

  const tap = (card: Card, playable: boolean) => {
    if (!playable) return;
    if (selected && sameCard(selected, card)) {
      onPlay(card);
      setSelectedKey(null);
    } else {
      setSelectedKey(cardKey(card));
    }
  };

  return (
    <div className="w-full">
      <div className="flex items-end justify-center px-2">
        {cards.map((card, index) => {
          const playable = myTurn && hasCard(legal, card);
          const isSelected = !!selected && sameCard(selected, card);

          return (
            <button
              key={cardKey(card)}
              type="button"
              onClick={() => tap(card, playable)}
              disabled={!playable}
              aria-pressed={isSelected}
              className="animate-deal relative shrink-0 rounded-lg transition-transform duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-gold"
              style={{
                width: "clamp(2.9rem, 13vw, 5rem)",
                aspectRatio: "100 / 150",
                marginLeft: index === 0 ? 0 : "clamp(-1.5rem, -5vw, -0.6rem)",
                zIndex: isSelected ? 50 : index,
                transform: isSelected ? "translateY(-1.35rem)" : undefined,
                filter: isSelected
                  ? "drop-shadow(0 10px 14px rgba(0,0,0,0.55))"
                  : "drop-shadow(0 3px 5px rgba(0,0,0,0.4))",
                cursor: playable ? "pointer" : "default",
              }}
            >
              <PlayingCard card={card} dimmed={myTurn && !playable} />
            </button>
          );
        })}
      </div>

      <div className="mt-3 flex h-11 items-center justify-center">
        {selected ? (
          <button
            type="button"
            className="btn btn-gold"
            onClick={() => {
              onPlay(selected);
              setSelectedKey(null);
            }}
          >
            Jouer cette carte
          </button>
        ) : myTurn ? (
          <p className="text-sm text-gold">A vous de jouer</p>
        ) : null}
      </div>
    </div>
  );
}

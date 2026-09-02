"use client";

import { useState } from "react";

import { PlayingCard } from "./PlayingCard";
import {
  RANK_LABEL,
  SUIT_SYMBOL,
  cardKey,
  hasCard,
  isRed,
  sameCard,
  sortHand,
} from "@/lib/cards";
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
              className="animate-deal relative shrink-0 transition-all duration-150 focus:outline-none"
              style={{
                width: "clamp(2.9rem, 13vw, 5rem)",
                aspectRatio: "100 / 150",
                marginLeft: index === 0 ? 0 : "clamp(-1.5rem, -5vw, -0.6rem)",
                zIndex: isSelected ? 50 : index,
                // La carte choisie sort franchement du rang : elle monte, elle
                // grandit, et un liseré doré l'entoure. Sur telephone les
                // cartes se chevauchent, un simple decalage ne suffit pas.
                transform: isSelected
                  ? "translateY(-2rem) scale(1.1)"
                  : undefined,
                borderRadius: "0.55rem",
                boxShadow: isSelected
                  ? "0 0 0 3px var(--color-gold), 0 14px 20px rgba(0,0,0,0.6)"
                  : "0 3px 5px rgba(0,0,0,0.4)",
                cursor: playable ? "pointer" : "default",
              }}
            >
              <PlayingCard card={card} dimmed={myTurn && !playable} />
            </button>
          );
        })}
      </div>

      <div className="mt-4 flex h-11 items-center justify-center gap-2">
        {selected ? (
          <>
            <button
              type="button"
              className="btn btn-gold"
              onClick={() => {
                onPlay(selected);
                setSelectedKey(null);
              }}
            >
              Jouer {RANK_LABEL[selected.rank]}
              <span
                style={{
                  color: isRed(selected.suit)
                    ? "var(--color-ruby)"
                    : "var(--color-ink-950)",
                }}
              >
                {SUIT_SYMBOL[selected.suit]}
              </span>
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setSelectedKey(null)}
            >
              Annuler
            </button>
          </>
        ) : myTurn ? (
          <p className="text-sm text-gold">
            À vous — touchez une carte pour la choisir
          </p>
        ) : null}
      </div>
    </div>
  );
}

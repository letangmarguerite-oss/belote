// Une carte, en SVG inline.
//
// SVG plutot qu'une image : net a toute densite d'ecran, aucune requete
// reseau, et la couleur se pilote en CSS.

import { RANK_LABEL, SUIT_SYMBOL, isRed } from "@/lib/cards";
import type { Card } from "@/lib/types";

interface Props {
  card: Card;
  /** Grisee : carte non jouable dans la situation actuelle. */
  dimmed?: boolean;
  className?: string;
}

export function PlayingCard({ card, dimmed = false, className = "" }: Props) {
  const red = isRed(card.suit);
  const ink = red ? "var(--color-ruby)" : "#16211d";
  const rank = RANK_LABEL[card.rank];
  const symbol = SUIT_SYMBOL[card.suit];

  return (
    <svg
      viewBox="0 0 100 150"
      className={`h-full w-full ${className}`}
      role="img"
      aria-label={`${rank} de ${card.suit}`}
      style={{ opacity: dimmed ? 0.42 : 1 }}
    >
      <rect
        x="1.5"
        y="1.5"
        width="97"
        height="147"
        rx="10"
        fill="var(--color-bone)"
        stroke="rgba(0,0,0,0.28)"
        strokeWidth="1.5"
      />
      <g fill={ink} fontFamily="Georgia, serif" fontWeight="700">
        <text x="10" y="30" fontSize="26" textAnchor="start">
          {rank}
        </text>
        <text x="10" y="50" fontSize="20" textAnchor="start">
          {symbol}
        </text>
        <text x="50" y="97" fontSize="52" textAnchor="middle">
          {symbol}
        </text>
        <g transform="rotate(180 50 75)">
          <text x="10" y="30" fontSize="26" textAnchor="start">
            {rank}
          </text>
          <text x="10" y="50" fontSize="20" textAnchor="start">
            {symbol}
          </text>
        </g>
      </g>
    </svg>
  );
}

/** Dos de carte, pour les mains adverses. */
export function CardBack({ className = "" }: { className?: string }) {
  return (
    <svg viewBox="0 0 100 150" className={`h-full w-full ${className}`} aria-hidden="true">
      <rect
        x="1.5"
        y="1.5"
        width="97"
        height="147"
        rx="10"
        fill="var(--color-felt-700)"
        stroke="var(--color-gold-dim)"
        strokeWidth="2"
      />
      <rect
        x="10"
        y="10"
        width="80"
        height="130"
        rx="6"
        fill="none"
        stroke="var(--color-gold-dim)"
        strokeWidth="1"
        opacity="0.55"
      />
      <circle cx="50" cy="75" r="17" fill="none" stroke="var(--color-gold-dim)" strokeWidth="1.5" opacity="0.7" />
    </svg>
  );
}

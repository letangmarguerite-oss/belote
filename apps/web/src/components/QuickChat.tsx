"use client";

import { useEffect, useRef, useState } from "react";

/**
 * Annonces toutes faites.
 *
 * Seul le rang de la phrase circule sur le reseau, jamais son texte : rien
 * qu'un joueur ait redige ne traverse la table. Il n'y a donc rien a moderer,
 * rien a echapper a l'affichage, et la liste peut etre reformulee sans toucher
 * au serveur.
 */
export const PHRASES = [
  "Bien joué !",
  "À toi",
  "Oups",
  "Merci !",
  "Désolé",
  "On y va ?",
] as const;

export function QuickChat({ onSay }: { onSay: (phrase: number) => void }) {
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);

  // Un appui a cote referme le panneau, comme n'importe quel menu.
  useEffect(() => {
    if (!open) return;
    const close = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);

  return (
    <div ref={container} className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex size-9 items-center justify-center rounded-full bg-ink-900/70 text-base text-bone-dim transition-colors hover:text-bone"
        aria-label="Envoyer une annonce"
        aria-expanded={open}
      >
        💬
      </button>

      {open && (
        <div className="panel absolute bottom-11 right-0 z-50 flex w-44 flex-col p-1">
          {PHRASES.map((phrase, index) => (
            <button
              key={phrase}
              type="button"
              className="rounded-lg px-3 py-2 text-left text-sm text-bone transition-colors hover:bg-bone/10"
              onClick={() => {
                onSay(index);
                setOpen(false);
              }}
            >
              {phrase}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

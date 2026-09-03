"use client";

import { useEffect, useRef, useState } from "react";

import { primeAudio, sounds } from "@/lib/sound";
import { useSettings } from "@/store/settings";

/** Reglages locaux : son et vibration, propres a cet appareil. */
export function Settings() {
  const { sound, vibrate, toggleSound, toggleVibrate } = useSettings();
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);

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
        onClick={() => {
          // Le navigateur n'autorise le son qu'apres un geste : on en profite.
          primeAudio();
          setOpen((v) => !v);
        }}
        className="flex size-9 items-center justify-center rounded-full bg-ink-900/70 text-base text-bone-dim transition-colors hover:text-bone"
        aria-label="Réglages"
        aria-expanded={open}
      >
        {sound ? "🔊" : "🔇"}
      </button>

      {open && (
        <div className="panel absolute right-0 top-11 z-50 flex w-48 flex-col p-1">
          <Toggle
            label="Sons"
            on={sound}
            onChange={() => {
              // On fait entendre le reglage qu'on vient d'activer.
              if (!sound) {
                primeAudio();
                sounds.card();
              }
              toggleSound();
            }}
          />
          <Toggle
            label="Vibration"
            on={vibrate}
            onChange={() => {
              if (!vibrate && "vibrate" in navigator) navigator.vibrate(18);
              toggleVibrate();
            }}
          />
          <p className="px-3 pb-1 pt-2 text-[0.7rem] text-bone-dim">
            La vibration ne fonctionne que sur téléphone.
          </p>
        </div>
      )}
    </div>
  );
}

function Toggle({
  label,
  on,
  onChange,
}: {
  label: string;
  on: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      onClick={onChange}
      className="flex items-center justify-between rounded-lg px-3 py-2 text-sm text-bone transition-colors hover:bg-bone/10"
    >
      {label}
      <span
        className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
          on ? "bg-gold" : "bg-bone/20"
        }`}
      >
        <span
          className={`absolute top-0.5 size-4 rounded-full bg-ink-950 transition-all ${
            on ? "left-[1.15rem]" : "left-0.5"
          }`}
        />
      </span>
    </button>
  );
}

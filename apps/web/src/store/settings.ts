"use client";

import { create } from "zustand";

const KEY = "belote:settings";

interface Settings {
  sound: boolean;
  vibrate: boolean;
  toggleSound: () => void;
  toggleVibrate: () => void;
}

/**
 * Preferences locales, conservees dans le navigateur.
 *
 * Elles ne concernent que cet appareil : on veut le son sur son ordinateur et
 * la vibration sur son telephone, sans que l'un impose l'autre. Rien de tout
 * cela n'a de raison de remonter au serveur.
 */
function load(): { sound: boolean; vibrate: boolean } {
  const defaults = { sound: true, vibrate: true };
  if (typeof window === "undefined") return defaults;
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return defaults;
    const parsed = JSON.parse(raw) as Partial<typeof defaults>;
    return {
      sound: parsed.sound ?? defaults.sound,
      vibrate: parsed.vibrate ?? defaults.vibrate,
    };
  } catch {
    // Navigation privee, stockage refuse, valeur corrompue : on repart des
    // reglages par defaut plutot que de casser la page.
    return defaults;
  }
}

function save(value: { sound: boolean; vibrate: boolean }): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(value));
  } catch {
    /* stockage indisponible : le reglage vaudra pour cette session */
  }
}

export const useSettings = create<Settings>((set, get) => ({
  ...load(),

  toggleSound: () => {
    const next = { sound: !get().sound, vibrate: get().vibrate };
    save(next);
    set(next);
  },

  toggleVibrate: () => {
    const next = { sound: get().sound, vibrate: !get().vibrate };
    save(next);
    set(next);
  },
}));

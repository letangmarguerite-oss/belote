"use client";

import { create } from "zustand";

import { api, refresh, setAccessToken } from "@/lib/api";
import type { AuthResponse, User } from "@/lib/types";

interface AuthState {
  user: User | null;
  /** Faux tant qu'on n'a pas tente de restaurer la session au chargement. */
  ready: boolean;
  bootstrap: () => Promise<void>;
  login: (email: string, password: string) => Promise<void>;
  register: (
    email: string,
    password: string,
    displayName: string,
  ) => Promise<void>;
  logout: () => Promise<void>;
}

export const useAuth = create<AuthState>((set) => ({
  user: null,
  ready: false,

  // Au chargement de la page, le jeton d'acces est perdu (il est en memoire).
  // Le cookie de rafraichissement, lui, a survecu : il rouvre la session.
  bootstrap: async () => {
    const auth = await refresh();
    set({ user: auth?.user ?? null, ready: true });
  },

  login: async (email, password) => {
    const auth = await api<AuthResponse>("/api/auth/login", {
      method: "POST",
      body: { email, password },
    });
    setAccessToken(auth.access_token);
    set({ user: auth.user, ready: true });
  },

  register: async (email, password, displayName) => {
    const auth = await api<AuthResponse>("/api/auth/register", {
      method: "POST",
      body: { email, password, display_name: displayName },
    });
    setAccessToken(auth.access_token);
    set({ user: auth.user, ready: true });
  },

  logout: async () => {
    try {
      await api("/api/auth/logout", { method: "POST" });
    } finally {
      setAccessToken(null);
      set({ user: null });
    }
  },
}));

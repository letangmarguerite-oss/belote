// Client HTTP.
//
// Le jeton d'acces vit en memoire, jamais dans localStorage : un script tiers
// injecte dans la page ne peut pas le lire. La session, elle, tient au cookie
// HttpOnly de rafraichissement, que le JavaScript ne voit pas.

import type { AuthResponse } from "./types";

/**
 * Vide par defaut : les appels partent en relatif vers le serveur Next, qui
 * relaie vers le back (voir next.config.ts). C'est ce qui rend l'application
 * utilisable depuis un telephone sans configurer d'adresse IP.
 *
 * En production, on peut pointer directement une API sur un autre domaine.
 */
export const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "";

/** Au-dela, on considere le serveur injoignable plutot que d'attendre sans fin. */
const REQUEST_TIMEOUT = 10_000;

/**
 * Adresse du WebSocket. Il ne passe pas par le relais Next (qui ne sait pas
 * relayer une connexion persistante) : on le construit depuis l'hote de la
 * page, donc il suit automatiquement localhost, l'IP locale ou le domaine.
 */
export function wsUrl(): string {
  const configured = process.env.NEXT_PUBLIC_WS_URL;
  if (configured) return configured;

  const port = process.env.NEXT_PUBLIC_WS_PORT ?? "8080";
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.hostname}:${port}`;
}

let accessToken: string | null = null;

export const setAccessToken = (token: string | null) => {
  accessToken = token;
};

export const getAccessToken = () => accessToken;

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

interface Options {
  method?: string;
  body?: unknown;
  /** Interdit la tentative de rafraichissement, pour ne pas boucler. */
  noRetry?: boolean;
}

export async function api<T>(path: string, options: Options = {}): Promise<T> {
  const { method = "GET", body, noRetry = false } = options;

  const headers: Record<string, string> = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  if (accessToken) headers.authorization = `Bearer ${accessToken}`;

  const res = await fetch(API_URL + path, {
    method,
    headers,
    // Indispensable : c'est ce qui transporte le cookie de rafraichissement.
    credentials: "include",
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT),
  });

  // Jeton expire : on le renouvelle une fois, puis on rejoue la requete.
  if (res.status === 401 && !noRetry && path !== "/api/auth/refresh") {
    const renewed = await refresh();
    if (renewed) return api<T>(path, { ...options, noRetry: true });
  }

  const text = await res.text();
  if (!res.ok) {
    let message = text;
    try {
      message = (JSON.parse(text) as { error?: string }).error ?? text;
    } catch {
      /* reponse non-JSON */
    }
    throw new ApiError(res.status, message || `Erreur ${res.status}`);
  }

  return text ? (JSON.parse(text) as T) : (undefined as T);
}

/** Echange le cookie contre un nouveau jeton d'acces. */
export async function refresh(): Promise<AuthResponse | null> {
  try {
    const auth = await api<AuthResponse>("/api/auth/refresh", {
      method: "POST",
      noRetry: true,
    });
    setAccessToken(auth.access_token);
    return auth;
  } catch {
    // Session absente, expiree, ou serveur injoignable : dans tous les cas on
    // repart sur un ecran de connexion plutot que de rester bloque.
    setAccessToken(null);
    return null;
  }
}

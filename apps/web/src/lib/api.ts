// Client HTTP.
//
// Le jeton d'acces vit en memoire, jamais dans localStorage : un script tiers
// injecte dans la page ne peut pas le lire. La session, elle, tient au cookie
// HttpOnly de rafraichissement, que le JavaScript ne voit pas.

import type { AuthResponse } from "./types";

export const API_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:8080";

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
    setAccessToken(null);
    return null;
  }
}

import type { NextConfig } from "next";

/**
 * L'API est servie par le meme hote que les pages.
 *
 * Sans cela, le navigateur d'un telephone appellerait `localhost:8080`, qui
 * designe le telephone lui-meme : la requete n'atteint jamais le PC. En
 * relayant depuis le serveur Next, l'application marche a l'identique sur
 * localhost, sur le reseau local et derriere un nom de domaine — et les
 * cookies restent en premiere partie, donc plus de question de CORS.
 */
const BACKEND_URL = process.env.BACKEND_URL ?? "http://localhost:8080";

const nextConfig: NextConfig = {
  async rewrites() {
    return [
      { source: "/api/:path*", destination: `${BACKEND_URL}/api/:path*` },
      { source: "/health", destination: `${BACKEND_URL}/health` },
    ];
  },
};

export default nextConfig;

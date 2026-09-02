import type { MetadataRoute } from "next";

/** Rend l'application installable sur telephone comme sur ordinateur. */
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Belote",
    short_name: "Belote",
    description: "Jouer a la belote en ligne avec ses amis.",
    start_url: "/",
    display: "standalone",
    background_color: "#062516",
    theme_color: "#062516",
    // L'orientation reste libre : le tapis s'adapte au portrait comme au paysage.
    icons: [
      {
        src: "/icon.svg",
        sizes: "any",
        type: "image/svg+xml",
        purpose: "any",
      },
    ],
  };
}

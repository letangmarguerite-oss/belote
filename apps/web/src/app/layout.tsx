import type { Metadata, Viewport } from "next";

import "./globals.css";

export const metadata: Metadata = {
  title: "Belote",
  description: "Jouer a la belote en ligne avec ses amis.",
  applicationName: "Belote",
  appleWebApp: { capable: true, statusBarStyle: "black-translucent", title: "Belote" },
};

export const viewport: Viewport = {
  themeColor: "#062516",
  // Le tapis se dimensionne lui-meme : le zoom ferait deborder les cartes.
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="fr">
      <body className="antialiased">
        <div id="app-root">{children}</div>
      </body>
    </html>
  );
}

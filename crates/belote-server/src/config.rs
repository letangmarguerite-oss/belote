//! Configuration lue dans l'environnement.

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    /// Origine autorisee pour CORS. Jamais `*` : on envoie des cookies.
    pub allowed_origin: String,
    pub port: u16,
    /// Domaine pose sur le cookie de refresh. Vide = cookie propre a l'hote.
    pub cookie_domain: Option<String>,
    /// En local (http) le cookie ne peut pas etre `Secure`.
    pub secure_cookies: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL manquante (voir .env.example)")?;
        let jwt_secret =
            std::env::var("JWT_SECRET").context("JWT_SECRET manquante (voir .env.example)")?;

        anyhow::ensure!(
            jwt_secret.len() >= 32,
            "JWT_SECRET doit faire au moins 32 caracteres"
        );

        let allowed_origin =
            std::env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());
        // Render impose le port par l'environnement.
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let cookie_domain = std::env::var("COOKIE_DOMAIN").ok().filter(|d| !d.is_empty());
        let secure_cookies = std::env::var("SECURE_COOKIES")
            .map(|v| v != "false" && v != "0")
            .unwrap_or_else(|_| allowed_origin.starts_with("https://"));

        Ok(Config {
            database_url,
            jwt_secret,
            allowed_origin,
            port,
            cookie_domain,
            secure_cookies,
        })
    }
}

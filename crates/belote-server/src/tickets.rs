//! Tickets d'ouverture de WebSocket.
//!
//! Le JWT ne doit pas transiter dans l'URL : une query string se retrouve dans
//! les journaux du serveur, dans l'en-tete Referer et dans l'historique du
//! navigateur. Le client demande donc un ticket a usage unique, valable 30
//! secondes, et c'est lui qu'il presente a la connexion.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use uuid::Uuid;

pub const TICKET_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Default)]
pub struct TicketStore {
    inner: Arc<Mutex<HashMap<String, (Uuid, Instant)>>>,
}

impl TicketStore {
    pub fn issue(&self, user_id: Uuid) -> String {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let ticket = URL_SAFE_NO_PAD.encode(bytes);

        let mut guard = self.inner.lock().expect("tickets empoisonnes");
        // On profite du passage pour jeter les tickets perimes.
        guard.retain(|_, (_, issued)| issued.elapsed() < TICKET_TTL);
        guard.insert(ticket.clone(), (user_id, Instant::now()));
        ticket
    }

    /// Consomme le ticket. Un meme ticket ne peut servir qu'une fois.
    pub fn redeem(&self, ticket: &str) -> Option<Uuid> {
        let mut guard = self.inner.lock().expect("tickets empoisonnes");
        let (user_id, issued) = guard.remove(ticket)?;
        if issued.elapsed() >= TICKET_TTL {
            return None;
        }
        Some(user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_works_once_and_only_once() {
        let store = TicketStore::default();
        let user = Uuid::new_v4();
        let ticket = store.issue(user);

        assert_eq!(store.redeem(&ticket), Some(user));
        assert_eq!(store.redeem(&ticket), None, "un ticket ne se rejoue pas");
    }

    #[test]
    fn an_unknown_ticket_is_refused() {
        let store = TicketStore::default();
        assert_eq!(store.redeem("jamais-emis"), None);
    }

    #[test]
    fn two_tickets_never_collide() {
        let store = TicketStore::default();
        let a = store.issue(Uuid::new_v4());
        let b = store.issue(Uuid::new_v4());
        assert_ne!(a, b);
    }
}

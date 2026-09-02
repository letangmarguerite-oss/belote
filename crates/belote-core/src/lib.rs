//! Moteur de regles de la belote classique.
//!
//! Ce crate est volontairement pur : pas d'async, pas de reseau, pas de base de
//! donnees. Il expose une machine a etats deterministe qu'on pilote avec des
//! `Action` et qui produit des `Event`. L'etat se reconstruit integralement en
//! rejouant ces evenements, ce qui donne l'historique, la reconnexion et la
//! reprise apres redemarrage sans code dedie.
//!
//! ```
//! use belote_core::{rules, state::{GameState, Seat}};
//! use rand::SeedableRng;
//!
//! let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
//! let mut state = GameState::new(Seat(0), 0);
//! rules::reduce(&mut state, &rules::start_deal(Seat(0), 0, &mut rng));
//! assert_eq!(state.hand(Seat(1)).len(), 5);
//! ```

pub mod action;
pub mod bot;
pub mod card;
pub mod event;
pub mod rules;
pub mod scoring;
pub mod sim;
pub mod state;
pub mod view;

pub use action::{Action, RuleError};
pub use card::{Card, Rank, Suit};
pub use event::{Event, PublicEvent};
pub use state::{DealScore, GameState, Phase, PlayedCard, Seat, Team};
pub use view::{project, PlayerView};

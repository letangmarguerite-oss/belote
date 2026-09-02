//! Deroulement complet d'une donne jouee par quatre bots.
//!
//! Sert de harnais de test : cela permet de valider le moteur sur des dizaines
//! de milliers de parties sans reseau, sans base et sans interface.

use rand::Rng;

use crate::bot;
use crate::event::Event;
use crate::rules::{apply, reduce, start_deal};
use crate::state::{GameState, Phase, Seat};
use crate::view::project;

/// Nombre maximal de redistributions avant d'abandonner (garde-fou : quatre
/// passes aux deux tours peuvent theoriquement se repeter).
const MAX_REDEALS: usize = 100;

#[derive(Debug)]
pub struct DealOutcome {
    pub state: GameState,
    pub events: Vec<Event>,
    pub redeals: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SimError {
    #[error("le bot n'a propose aucune action en phase {0:?}")]
    BotStuck(Phase),
    #[error("action refusee par le moteur: {0}")]
    Rejected(#[from] crate::action::RuleError),
    #[error("trop de redistributions consecutives : personne ne prend jamais")]
    TooManyRedeals,
}

/// Joue une donne entiere. Redistribue tant que personne ne prend.
pub fn play_deal<R: Rng>(dealer: Seat, carry_in: u16, rng: &mut R) -> Result<DealOutcome, SimError> {
    let mut events = Vec::new();
    let mut redeals = 0;

    loop {
        let mut state = GameState::new(dealer, carry_in);
        let dealt = start_deal(dealer, carry_in, rng);
        reduce(&mut state, &dealt);
        events.push(dealt);

        while !state.is_over() {
            let seat = state.turn;
            let view = project(&state, seat);
            let action = bot::choose_action(&view).ok_or(SimError::BotStuck(state.phase))?;
            for ev in apply(&state, seat, action)? {
                reduce(&mut state, &ev);
                events.push(ev);
            }
        }

        if state.phase == Phase::Finished {
            return Ok(DealOutcome {
                state,
                events,
                redeals,
            });
        }

        redeals += 1;
        if redeals > MAX_REDEALS {
            return Err(SimError::TooManyRedeals);
        }
    }
}

/// Verifie les invariants qui doivent tenir sur toute donne achevee.
/// Renvoie la liste des violations constatees (vide = tout va bien).
pub fn check_invariants(outcome: &DealOutcome) -> Vec<String> {
    use crate::scoring::DEAL_TOTAL;
    use std::collections::HashSet;

    let mut problems = Vec::new();
    let state = &outcome.state;

    if state.card_points[0] + state.card_points[1] != DEAL_TOTAL {
        problems.push(format!(
            "total des plis = {} au lieu de {DEAL_TOTAL}",
            state.card_points[0] + state.card_points[1]
        ));
    }
    if state.tricks_won[0] + state.tricks_won[1] != 8 {
        problems.push(format!(
            "{} plis joues au lieu de 8",
            state.tricks_won[0] + state.tricks_won[1]
        ));
    }
    if state.hands.iter().any(|h| !h.is_empty()) {
        problems.push("des cartes restent en main a la fin de la donne".into());
    }

    // Aucune carte ne doit avoir ete jouee deux fois, et il doit y en avoir 32.
    let played: Vec<_> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Played { card, .. } => Some(*card),
            _ => None,
        })
        .collect();
    // On ne compte que la derniere donne effectivement jouee : les
    // redistributions precedentes n'ont produit aucun `Played`.
    if played.len() != 32 {
        problems.push(format!("{} cartes posees au lieu de 32", played.len()));
    }
    if played.iter().collect::<HashSet<_>>().len() != played.len() {
        problems.push("une carte a ete posee deux fois".into());
    }

    if state.score.is_none() {
        problems.push("la donne s'est terminee sans decompte".into());
    }

    problems
}

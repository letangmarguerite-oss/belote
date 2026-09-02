//! Harnais de simulation : fait jouer quatre bots et verifie les invariants.
//!
//! Usage : cargo run --bin simulate -- --seed 42 --games 1000 [--verbose]

use std::process::ExitCode;

use belote_core::rules::reduce;
use belote_core::sim::{check_invariants, play_deal};
use belote_core::state::{GameState, Seat};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() -> ExitCode {
    let mut seed = 42u64;
    let mut games = 1000usize;
    let mut verbose = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                seed = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(seed);
                i += 2;
            }
            "--games" => {
                games = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(games);
                i += 2;
            }
            "--verbose" => {
                verbose = true;
                i += 1;
            }
            other => {
                eprintln!("argument inconnu: {other}");
                return ExitCode::FAILURE;
            }
        }
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut failures = 0usize;
    let mut totals = [0u32; 2];
    let mut takes = [0u32; 4];
    let mut contracts_made = 0usize;
    let mut capots = 0usize;
    let mut belotes = 0usize;
    let mut litiges = 0usize;
    let mut redeals = 0usize;
    let mut carry = 0u16;

    for game in 0..games {
        let dealer = Seat((game % 4) as u8);
        let outcome = match play_deal(dealer, carry, &mut rng) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("donne {game} : {e}");
                failures += 1;
                carry = 0;
                continue;
            }
        };

        let problems = check_invariants(&outcome);
        if !problems.is_empty() {
            failures += 1;
            eprintln!("donne {game} : {}", problems.join(" | "));
        }

        // Le journal doit reconstruire exactement le meme etat.
        let mut replay = GameState::new(dealer, carry);
        for ev in &outcome.events {
            reduce(&mut replay, ev);
        }
        if replay != outcome.state {
            failures += 1;
            eprintln!("donne {game} : le rejeu du journal diverge de l'etat courant");
        }

        let score = outcome.state.score.as_ref().expect("donne comptee");
        totals[0] += score.points[0] as u32;
        totals[1] += score.points[1] as u32;
        takes[score.taker.index()] += 1;
        contracts_made += score.contract_made as usize;
        capots += score.capot.is_some() as usize;
        belotes += score.belote.is_some() as usize;
        litiges += score.litige as usize;
        redeals += outcome.redeals;
        carry = score.carry_out;

        if verbose {
            println!(
                "donne {game:>4} | preneur {:?} atout {:<8} | {:>3} - {:<3} | {}",
                score.taker,
                score.trump.name_fr(),
                score.points[0],
                score.points[1],
                if score.contract_made {
                    "contrat"
                } else if score.litige {
                    "litige"
                } else {
                    "dedans"
                }
            );
        }
    }

    println!("\n--- {games} donnes, graine {seed} ---");
    println!("echecs d'invariants : {failures}");
    println!("points cumules      : equipe A {} / equipe B {}", totals[0], totals[1]);
    println!(
        "contrats remplis    : {contracts_made} ({:.1} %)",
        100.0 * contracts_made as f64 / games as f64
    );
    println!("prises par siege    : {takes:?}");
    println!("capots {capots} | belotes {belotes} | litiges {litiges} | redistributions {redeals}");

    if failures == 0 {
        println!("\nOK : aucun invariant viole.");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

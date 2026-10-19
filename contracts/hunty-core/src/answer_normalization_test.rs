//! #1255: answers were only trimmed and ASCII-lowercased, so `"New   York"`
//! and `"new york"` hashed differently and a player with the right answer lost
//! an attempt. Internal whitespace runs now collapse to one space. Non-ASCII
//! text is deliberately left as-is (the contract cannot case-fold Unicode);
//! that behaviour is documented on `normalize_and_hash_answer`.

use crate::errors::HuntError;
use crate::storage::Storage;
use crate::HuntyCore;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, BytesN, Env, String};

const T0: u64 = 1_700_000_000;

fn step<T>(env: &Env, contract_id: &Address, f: impl FnOnce(&Env) -> T) -> T {
    env.as_contract(contract_id, || f(env))
}

fn hash(env: &Env, contract_id: &Address, answer: &str) -> BytesN<32> {
    let answer = String::from_str(env, answer);
    step(env, contract_id, |env| {
        HuntyCore::normalize_and_hash_answer(env, 1, 1, &answer).unwrap()
    })
}

fn new_env() -> (Env, Address) {
    let env = Env::default();
    env.ledger().set_timestamp(T0);
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    (env, contract_id)
}

/// Builds an Active, one-clue hunt with the supplied stored answer.
fn active_hunt(env: &Env, contract_id: &Address, creator: &Address, answer: &str) -> (u64, u32) {
    let hunt_id = step(env, contract_id, |env| {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Normalization Hunt"),
            String::from_str(env, "Answer normalization"),
            None,
            None,
            0,
            None,
            None,
        )
        .unwrap()
    });
    let stored_answer = String::from_str(env, answer);
    let clue_id = step(env, contract_id, |env| {
        HuntyCore::add_clue(
            env.clone(),
            hunt_id,
            String::from_str(env, "Question?"),
            stored_answer,
            10,
            true,
            None,
            None,
        )
        .unwrap()
    });
    step(env, contract_id, |env| {
        HuntyCore::activate_hunt(env.clone(), hunt_id, creator.clone()).unwrap()
    });
    (hunt_id, clue_id)
}

fn register(env: &Env, contract_id: &Address, hunt_id: u64, player: &Address) {
    step(env, contract_id, |env| {
        HuntyCore::register_player(env.clone(), hunt_id, player.clone()).unwrap()
    });
}

// ---------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------

#[test]
fn internal_whitespace_runs_collapse_to_one_space() {
    let (env, contract_id) = new_env();

    let expected = hash(&env, &contract_id, "new york");
    for variant in ["New   York", "NEW YORK", "new\tyork", "new \n York"] {
        assert_eq!(
            hash(&env, &contract_id, variant),
            expected,
            "{variant:?} must normalize to \"new york\""
        );
    }
}

#[test]
fn outer_whitespace_is_trimmed() {
    let (env, contract_id) = new_env();
    assert_eq!(
        hash(&env, &contract_id, "   answer \t\n"),
        hash(&env, &contract_id, "answer")
    );
}

#[test]
fn only_ascii_letters_are_lowercased() {
    let (env, contract_id) = new_env();

    // Documented limitation: no Unicode case folding, so the initial É stays
    // uppercase. This is what makes "ÉCOLE" and "école" different answers.
    assert_eq!(
        hash(&env, &contract_id, "ÉCOLE"),
        hash(&env, &contract_id, "École")
    );
    assert_ne!(
        hash(&env, &contract_id, "ÉCOLE"),
        hash(&env, &contract_id, "école")
    );

    // Accents are preserved byte-for-byte while ASCII letters still fold.
    assert_eq!(
        hash(&env, &contract_id, "Café"),
        hash(&env, &contract_id, "café")
    );
    assert_ne!(
        hash(&env, &contract_id, "Café"),
        hash(&env, &contract_id, "Cafe")
    );
}

#[test]
fn whitespace_only_answers_are_still_rejected() {
    let (env, contract_id) = new_env();
    let answer = String::from_str(&env, " \t \n ");
    let result = step(&env, &contract_id, |env| {
        HuntyCore::normalize_and_hash_answer(env, 1, 1, &answer)
    });
    assert!(matches!(result, Err(HuntError::InvalidAnswer)));
}

// ---------------------------------------------------------------------------
// End-to-end: a player with the right answer is no longer punished
// ---------------------------------------------------------------------------

#[test]
fn double_spaced_stored_answer_accepts_the_single_spaced_submission() {
    let (env, contract_id) = new_env();
    let creator = Address::generate(&env);
    let player = Address::generate(&env);
    let (hunt_id, clue_id) = active_hunt(&env, &contract_id, &creator, "New  York");
    register(&env, &contract_id, hunt_id, &player);

    let result = step(&env, &contract_id, |env| {
        HuntyCore::submit_answer(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            String::from_str(env, "new york"),
            1,
            T0,
        )
    });
    assert_eq!(result, Ok(true));

    let progress = step(&env, &contract_id, |env| {
        Storage::get_player_progress(env, hunt_id, &player).unwrap()
    });
    assert!(progress.completed_clues.contains(clue_id));
}

#[test]
fn extra_whitespace_in_the_submission_still_scores() {
    let (env, contract_id) = new_env();
    let creator = Address::generate(&env);
    let player = Address::generate(&env);
    let (hunt_id, clue_id) = active_hunt(&env, &contract_id, &creator, "new york");
    register(&env, &contract_id, hunt_id, &player);

    let result = step(&env, &contract_id, |env| {
        HuntyCore::submit_answer(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            String::from_str(env, "  NEW   YORK  "),
            1,
            T0,
        )
    });
    assert_eq!(result, Ok(true));
}

#[test]
fn precomputed_hash_of_the_normalized_answer_is_accepted() {
    let (env, contract_id) = new_env();
    let creator = Address::generate(&env);
    let player = Address::generate(&env);
    let (hunt_id, clue_id) = active_hunt(&env, &contract_id, &creator, "New   York");
    register(&env, &contract_id, hunt_id, &player);

    // Off-chain callers hash the normalized form; the contract must agree.
    let normalized = String::from_str(&env, "new york");
    let answer_hash = step(&env, &contract_id, |env| {
        HuntyCore::normalize_and_hash_answer(env, hunt_id, clue_id, &normalized).unwrap()
    });

    let result = step(&env, &contract_id, |env| {
        HuntyCore::submit_answer_with_hash(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            answer_hash.clone(),
            1,
            T0,
        )
    });
    assert_eq!(result, Ok(true));
}

#[test]
fn alias_answers_use_the_same_normalization() {
    let (env, contract_id) = new_env();
    let creator = Address::generate(&env);
    let player = Address::generate(&env);

    // Aliases are Draft-only, so configure the clue before activation.
    let hunt_id = step(&env, &contract_id, |env| {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Alias Hunt"),
            String::from_str(env, "Alias normalization"),
            None,
            None,
            0,
            None,
            None,
        )
        .unwrap()
    });
    let clue_id = step(&env, &contract_id, |env| {
        HuntyCore::add_clue(
            env.clone(),
            hunt_id,
            String::from_str(env, "Question?"),
            String::from_str(env, "statue"),
            10,
            true,
            None,
            None,
        )
        .unwrap()
    });
    step(&env, &contract_id, |env| {
        HuntyCore::add_clue_aliases(
            env.clone(),
            hunt_id,
            clue_id,
            soroban_sdk::vec![&env, String::from_str(env, "Empire  State")],
        )
        .unwrap()
    });
    step(&env, &contract_id, |env| {
        HuntyCore::activate_hunt(env.clone(), hunt_id, creator.clone()).unwrap()
    });
    register(&env, &contract_id, hunt_id, &player);

    let result = step(&env, &contract_id, |env| {
        HuntyCore::submit_answer(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            String::from_str(env, "empire state"),
            1,
            T0,
        )
    });
    assert_eq!(result, Ok(true));
}

#[test]
fn wrong_answers_are_still_reported_as_incorrect() {
    let (env, contract_id) = new_env();
    let creator = Address::generate(&env);
    let player = Address::generate(&env);
    let (hunt_id, clue_id) = active_hunt(&env, &contract_id, &creator, "New York");
    register(&env, &contract_id, hunt_id, &player);

    let result = step(&env, &contract_id, |env| {
        HuntyCore::submit_answer(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            String::from_str(env, "new jersey"),
            1,
            T0,
        )
    });
    // Collapsing whitespace must not turn near-misses into matches.
    assert_eq!(result, Ok(false));
}

//! Tests for the O(1) `get_hunt_statistics` implementation (issue: statistics
//! scan).
//!
//! # Background
//!
//! The old implementation called `Storage::get_hunt_players`, which loaded
//! every `PlayerProgress` record in order to compute `completed_count` and
//! `total_score_sum`.  On large hunts that exceeds Soroban's per-invocation
//! read-entry budget and the call fails.
//!
//! The fix maintains two running counters:
//!   - `HCMPL` — incremented once when a player's `is_completed` flips to true.
//!   - `HSCRS` — incremented by the clue score on every successful submission.
//!
//! `get_hunt_statistics` now reads three O(1) keys (`PLCT`, `HCMPL`, `HSCRS`)
//! regardless of player count.  These tests pin the correctness of the counters
//! and demonstrate that the call works with a large number of players.

#![cfg(test)]

use crate::storage::Storage;
use crate::HuntyCore;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String};

const T0: u64 = 1_700_000_000;

// ── helpers ──────────────────────────────────────────────────────────────────

fn step<T>(env: &Env, cid: &Address, f: impl FnOnce(&Env) -> T) -> T {
    env.as_contract(cid, || f(env))
}

/// Creates an active hunt with a single required clue worth `points` points.
/// Returns `(hunt_id, clue_id)`.
fn setup_hunt(env: &Env, cid: &Address, creator: &Address, points: u32) -> (u64, u32) {
    let hunt_id = step(env, cid, |env| {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Stats hunt"),
            String::from_str(env, "desc"),
            None,
            None,
            0,   // no submission rate limit
            None,
            None,
        )
        .unwrap()
    });

    // Allow many attempts so the budget test isn't blocked by attempt limits.
    step(env, cid, |env| {
        HuntyCore::set_max_attempts_per_clue(env.clone(), hunt_id, creator.clone(), 999, 0)
            .unwrap()
    });

    let clue_id = step(env, cid, |env| {
        HuntyCore::add_clue(
            env.clone(),
            hunt_id,
            String::from_str(env, "Q?"),
            String::from_str(env, "right"),
            points,
            true,  // is_required
            None,
            None,
        )
        .unwrap()
    });

    step(env, cid, |env| {
        HuntyCore::activate_hunt(env.clone(), hunt_id, creator.clone()).unwrap()
    });

    (hunt_id, clue_id)
}

fn register(env: &Env, cid: &Address, hunt_id: u64, player: &Address) {
    step(env, cid, |env| {
        HuntyCore::register_player(env.clone(), hunt_id, player.clone()).unwrap()
    });
}

fn complete(env: &Env, cid: &Address, hunt_id: u64, clue_id: u32, player: &Address) {
    step(env, cid, |env| {
        HuntyCore::submit_answer(
            env.clone(),
            hunt_id,
            clue_id,
            player.clone(),
            String::from_str(env, "right"),
            env.ledger().timestamp(),
        )
        .unwrap()
    });
}

// ── unit-style correctness tests ─────────────────────────────────────────────

/// Empty hunt: all stats are zero and the call succeeds.
#[test]
fn test_statistics_empty_hunt() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);

    let (hunt_id, _clue_id) = setup_hunt(&env, &cid, &creator, 50);

    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    assert_eq!(stats.total_players, 0);
    assert_eq!(stats.completed_count, 0);
    assert_eq!(stats.total_score_sum, 0);
    assert_eq!(stats.completion_rate_percent, 0);
    assert_eq!(stats.average_score, 0);
}

/// One player registers but does not complete: total_players = 1, completed = 0.
#[test]
fn test_statistics_registered_not_completed() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let player = Address::generate(&env);

    let (hunt_id, _clue_id) = setup_hunt(&env, &cid, &creator, 50);
    register(&env, &cid, hunt_id, &player);

    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    assert_eq!(stats.total_players, 1);
    assert_eq!(stats.completed_count, 0);
    assert_eq!(stats.total_score_sum, 0);
    assert_eq!(stats.completion_rate_percent, 0);
    assert_eq!(stats.average_score, 0);
}

/// One player completes the hunt: all counters are updated correctly.
#[test]
fn test_statistics_one_completer() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let player = Address::generate(&env);

    let (hunt_id, clue_id) = setup_hunt(&env, &cid, &creator, 100);
    register(&env, &cid, hunt_id, &player);
    complete(&env, &cid, hunt_id, clue_id, &player);

    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    assert_eq!(stats.total_players, 1);
    assert_eq!(stats.completed_count, 1);
    assert_eq!(stats.completion_rate_percent, 100);
    // score_sum >= clue points (time-bonus can only increase it)
    assert!(stats.total_score_sum >= 100);
    assert_eq!(stats.average_score, stats.total_score_sum as u32);
}

/// Two players register, only one completes: completion rate = 50 %.
#[test]
fn test_statistics_partial_completion() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let (hunt_id, clue_id) = setup_hunt(&env, &cid, &creator, 100);
    register(&env, &cid, hunt_id, &p1);
    register(&env, &cid, hunt_id, &p2);
    complete(&env, &cid, hunt_id, clue_id, &p1);

    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    assert_eq!(stats.total_players, 2);
    assert_eq!(stats.completed_count, 1);
    assert_eq!(stats.completion_rate_percent, 50);
}

/// Checks that the running counters in storage are consistent with the
/// values returned by `get_hunt_statistics`.
#[test]
fn test_statistics_counters_match_storage() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let (hunt_id, clue_id) = setup_hunt(&env, &cid, &creator, 80);
    register(&env, &cid, hunt_id, &p1);
    register(&env, &cid, hunt_id, &p2);
    complete(&env, &cid, hunt_id, clue_id, &p1);
    complete(&env, &cid, hunt_id, clue_id, &p2);

    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    // Direct counter reads must agree with the public API.
    let raw_completed = step(&env, &cid, |env| {
        Storage::get_hunt_completed_count(env, hunt_id)
    });
    let raw_score_sum = step(&env, &cid, |env| {
        Storage::get_hunt_score_sum(env, hunt_id)
    });
    let raw_player_count = step(&env, &cid, |env| {
        Storage::get_player_count(env, hunt_id)
    });

    assert_eq!(stats.total_players, raw_player_count);
    assert_eq!(stats.completed_count, raw_completed);
    assert_eq!(stats.total_score_sum, raw_score_sum);
}

// ── budget test ───────────────────────────────────────────────────────────────

/// Registers and completes MANY_PLAYERS players, then calls `get_hunt_statistics`
/// exactly once.  Because the implementation reads only three O(1) counters the
/// call must not fail regardless of the player count.  (If the old scan-all-
/// players code were still in place it would exceed Soroban's read budget long
/// before reaching this player count.)
///
/// We keep the count modest (50) so the test itself completes quickly without
/// exceeding the *write* budget incurred while populating state.
#[test]
fn test_statistics_budget_many_players() {
    const MANY_PLAYERS: u32 = 50;
    const CLUE_POINTS: u32 = 10;

    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(T0);

    let cid = env.register(HuntyCore, ());
    let creator = Address::generate(&env);

    let (hunt_id, clue_id) = setup_hunt(&env, &cid, &creator, CLUE_POINTS);

    // Register all players.
    let players: soroban_sdk::Vec<Address> = {
        let mut v = soroban_sdk::Vec::new(&env);
        for _ in 0..MANY_PLAYERS {
            v.push_back(Address::generate(&env));
        }
        v
    };

    for i in 0..MANY_PLAYERS as usize {
        let p = players.get(i as u32).unwrap();
        register(&env, &cid, hunt_id, &p);
    }

    // Half the players complete the hunt.
    let completers = MANY_PLAYERS / 2;
    for i in 0..completers as usize {
        let p = players.get(i as u32).unwrap();
        complete(&env, &cid, hunt_id, clue_id, &p);
    }

    // This single call must succeed with O(1) reads.
    let stats = step(&env, &cid, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    assert_eq!(stats.total_players, MANY_PLAYERS);
    assert_eq!(stats.completed_count, completers);
    assert_eq!(stats.completion_rate_percent, 50);
    // Each completer earned at least CLUE_POINTS.
    assert!(stats.total_score_sum >= (completers as u64) * (CLUE_POINTS as u64));
    // Non-completers have score 0, so average ≤ total / total_players.
    let expected_avg = stats.total_score_sum / (MANY_PLAYERS as u64);
    assert_eq!(stats.average_score as u64, expected_avg);
}

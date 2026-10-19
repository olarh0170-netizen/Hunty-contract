//! #1251: `difficulty_rating` is the mean clue difficulty and must round to
//! the nearest tier. Integer division used to truncate it, so a hunt mixing
//! easy and hard clues (e.g. 2, 2, 2, 5) was rated 2 instead of 3.

use crate::types::BatchClueInput;
use crate::HuntyCore;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String};

fn create_hunt(env: &Env, contract_id: &Address, creator: &Address) -> u64 {
    env.as_contract(contract_id, || {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Rounding Hunt"),
            String::from_str(env, "Difficulty rating rounds to the nearest tier"),
            None,
            None,
            0,
            None,
            None,
        )
        .unwrap()
    })
}

fn add_clue(env: &Env, contract_id: &Address, hunt_id: u64, difficulty: Option<u32>) {
    env.as_contract(contract_id, || {
        HuntyCore::add_clue(
            env.clone(),
            hunt_id,
            String::from_str(env, "Question"),
            String::from_str(env, "answer"),
            10,
            true,
            difficulty,
            None,
        )
        .unwrap()
    });
}

fn rating(env: &Env, contract_id: &Address, hunt_id: u64) -> u32 {
    env.as_contract(contract_id, || {
        HuntyCore::get_hunt_info(env.clone(), hunt_id)
            .unwrap()
            .difficulty_rating
    })
}

#[test]
fn mixed_clue_difficulty_rounds_instead_of_truncating() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    // Mean of 2, 2, 2, 5 is 2.75; truncation reported 2.
    for difficulty in [2u32, 2, 2, 5] {
        add_clue(&env, &contract_id, hunt_id, Some(difficulty));
    }

    assert_eq!(rating(&env, &contract_id, hunt_id), 3);
}

#[test]
fn average_below_half_rounds_down() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    // Mean of 1, 1, 2 is 1.33.
    for difficulty in [1u32, 1, 2] {
        add_clue(&env, &contract_id, hunt_id, Some(difficulty));
    }

    assert_eq!(rating(&env, &contract_id, hunt_id), 1);
}

#[test]
fn exact_half_rounds_up() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    // Mean of 2, 3 is 2.5.
    add_clue(&env, &contract_id, hunt_id, Some(2));
    add_clue(&env, &contract_id, hunt_id, Some(3));

    assert_eq!(rating(&env, &contract_id, hunt_id), 3);
}

#[test]
fn exact_integer_average_is_unchanged() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    for difficulty in [2u32, 4] {
        add_clue(&env, &contract_id, hunt_id, Some(difficulty));
    }

    assert_eq!(rating(&env, &contract_id, hunt_id), 3);
}

#[test]
fn batch_added_clues_use_the_same_rounding() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    let clues = vec![
        &env,
        BatchClueInput {
            question: String::from_str(&env, "Q1"),
            answer: String::from_str(&env, "A1"),
            points: 10,
            is_required: true,
            difficulty: 2,
        },
        BatchClueInput {
            question: String::from_str(&env, "Q2"),
            answer: String::from_str(&env, "A2"),
            points: 10,
            is_required: true,
            difficulty: 2,
        },
        BatchClueInput {
            question: String::from_str(&env, "Q3"),
            answer: String::from_str(&env, "A3"),
            points: 10,
            is_required: true,
            difficulty: 2,
        },
        BatchClueInput {
            question: String::from_str(&env, "Q4"),
            answer: String::from_str(&env, "A4"),
            points: 10,
            is_required: true,
            difficulty: 5,
        },
    ];
    env.as_contract(&contract_id, || {
        HuntyCore::add_clues(env.clone(), hunt_id, clues.clone()).unwrap();
    });

    assert_eq!(rating(&env, &contract_id, hunt_id), 3);
}

#[test]
fn hunt_without_clues_has_zero_rating() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    assert_eq!(rating(&env, &contract_id, hunt_id), 0);
}

//! #1252: `add_clues` used to pass `None` for the clue weight, so every
//! batch-added clue had weight 1 and a batch hunt could not be configured the
//! same way as a hunt built with `add_clue`. `BatchClueInput` now carries the
//! same optional weight as `add_clue`.

use crate::errors::HuntErrorCode;
use crate::storage::Storage;
use crate::types::{BatchClueInput, HuntStatus};
use crate::HuntyCore;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String};

fn create_hunt(env: &Env, contract_id: &Address, creator: &Address, title: &str) -> u64 {
    env.as_contract(contract_id, || {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, title),
            String::from_str(env, "Batch clue weight"),
            None,
            None,
            0,
            None,
            None,
        )
        .unwrap()
    })
}

fn batch_clue(env: &Env, question: &str, difficulty: u32, weight: Option<u32>) -> BatchClueInput {
    BatchClueInput {
        question: String::from_str(env, question),
        answer: String::from_str(env, "answer"),
        points: 10,
        is_required: true,
        difficulty,
        weight,
    }
}

#[test]
fn batched_clues_keep_their_configured_weight() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator, "Weighted Batch");

    let clues = vec![
        &env,
        batch_clue(&env, "Q1", 2, Some(3)),
        batch_clue(&env, "Q2", 2, None),
        batch_clue(&env, "Q3", 4, Some(1)),
    ];
    env.as_contract(&contract_id, || {
        HuntyCore::add_clues(env.clone(), hunt_id, clues.clone()).unwrap();
    });

    env.as_contract(&contract_id, || {
        assert_eq!(
            Storage::get_clue_or_error(&env, hunt_id, 1).unwrap().weight,
            3
        );
        // No weight keeps the documented default of 1.
        assert_eq!(
            Storage::get_clue_or_error(&env, hunt_id, 2).unwrap().weight,
            1
        );
        assert_eq!(
            Storage::get_clue_or_error(&env, hunt_id, 3).unwrap().weight,
            1
        );
    });
}

#[test]
fn batch_and_single_clue_creation_produce_the_same_clue() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);

    let single_hunt = create_hunt(&env, &contract_id, &creator, "Single");
    let batch_hunt = create_hunt(&env, &contract_id, &creator, "Batch");

    env.as_contract(&contract_id, || {
        HuntyCore::add_clue(
            env.clone(),
            single_hunt,
            String::from_str(&env, "Same question"),
            String::from_str(&env, "answer"),
            10,
            true,
            Some(2),
            Some(3),
        )
        .unwrap();
        HuntyCore::add_clues(
            env.clone(),
            batch_hunt,
            vec![&env, batch_clue(&env, "Same question", 2, Some(3))],
        )
        .unwrap();
    });

    env.as_contract(&contract_id, || {
        let single = Storage::get_clue_or_error(&env, single_hunt, 1).unwrap();
        let batch = Storage::get_clue_or_error(&env, batch_hunt, 1).unwrap();
        assert_eq!(batch.question, single.question);
        assert_eq!(batch.points, single.points);
        assert_eq!(batch.is_required, single.is_required);
        assert_eq!(batch.difficulty, single.difficulty);
        assert_eq!(batch.weight, single.weight);
    });
}

#[test]
fn zero_weight_is_rejected_for_batched_clues() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator, "Zero Weight");

    env.as_contract(&contract_id, || {
        assert_eq!(
            HuntyCore::add_clues(
                env.clone(),
                hunt_id,
                vec![&env, batch_clue(&env, "Q1", 2, Some(0))],
            ),
            Err(HuntErrorCode::InvalidWeight)
        );
        // The rejected batch must not leave a partially added clue behind.
        assert_eq!(Storage::get_clue_counter(&env, hunt_id), 0);
        let hunt = Storage::get_hunt_or_error(&env, hunt_id).unwrap();
        assert_eq!(hunt.total_clues, 0);
        assert_eq!(hunt.status, HuntStatus::Draft);
    });
}

#[test]
fn weighted_batched_clue_changes_the_clue_but_not_the_hunt_difficulty() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator, "Weight vs Difficulty");

    env.as_contract(&contract_id, || {
        HuntyCore::add_clues(
            env.clone(),
            hunt_id,
            vec![&env, batch_clue(&env, "Q1", 2, Some(5))],
        )
        .unwrap();

        let clue = Storage::get_clue_or_error(&env, hunt_id, 1).unwrap();
        assert_eq!(clue.weight, 5);
        // Weight is a scoring multiplier, not a difficulty tier.
        assert_eq!(clue.difficulty, 2);
        assert_eq!(
            Storage::get_hunt_or_error(&env, hunt_id)
                .unwrap()
                .difficulty_rating,
            2
        );
    });
}

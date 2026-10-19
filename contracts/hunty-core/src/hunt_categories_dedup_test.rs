//! #1253: `set_hunt_categories` accepted duplicates, so `["art", "art", "art"]`
//! used three of the five category slots for one category. Duplicates are now
//! collapsed after sanitization and only distinct categories count towards the
//! cap.

use crate::errors::HuntErrorCode;
use crate::types::HuntStatus;
use crate::HuntyCore;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String};

fn create_hunt(env: &Env, contract_id: &Address, creator: &Address) -> u64 {
    env.as_contract(contract_id, || {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Category Hunt"),
            String::from_str(env, "Category de-duplication"),
            None,
            None,
            0,
            None,
            None,
        )
        .unwrap()
    })
}

fn set_categories(
    env: &Env,
    contract_id: &Address,
    hunt_id: u64,
    creator: &Address,
    categories: soroban_sdk::Vec<String>,
) -> Result<(), HuntErrorCode> {
    env.as_contract(contract_id, || {
        HuntyCore::set_hunt_categories(env.clone(), hunt_id, creator.clone(), categories)
    })
}

fn categories(env: &Env, contract_id: &Address, hunt_id: u64) -> soroban_sdk::Vec<String> {
    env.as_contract(contract_id, || {
        HuntyCore::get_hunt_info(env.clone(), hunt_id)
            .unwrap()
            .categories
    })
}

#[test]
fn duplicate_categories_are_collapsed() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, "art"),
            String::from_str(&env, "art"),
            String::from_str(&env, "art"),
        ],
    )
    .unwrap();

    let stored = categories(&env, &contract_id, hunt_id);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored.get(0).unwrap(), String::from_str(&env, "art"));
}

#[test]
fn duplicates_created_by_sanitization_are_collapsed() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    // Trimming turns these into the same category. Comparison stays
    // case-sensitive, so "Art" is a distinct category.
    set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, " art "),
            String::from_str(&env, "art"),
            String::from_str(&env, "Art"),
        ],
    )
    .unwrap();

    let stored = categories(&env, &contract_id, hunt_id);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored.get(0).unwrap(), String::from_str(&env, "art"));
    assert_eq!(stored.get(1).unwrap(), String::from_str(&env, "Art"));
}

#[test]
fn duplicates_do_not_consume_category_slots() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, "art"),
            String::from_str(&env, "art"),
            String::from_str(&env, "art"),
            String::from_str(&env, "music"),
            String::from_str(&env, "sport"),
            String::from_str(&env, "code"),
            String::from_str(&env, "game"),
        ],
    )
    .unwrap();

    let stored = categories(&env, &contract_id, hunt_id);
    assert_eq!(stored.len(), 5);
    assert_eq!(stored.get(0).unwrap(), String::from_str(&env, "art"));
    assert_eq!(stored.get(4).unwrap(), String::from_str(&env, "game"));
}

#[test]
fn first_seen_order_is_preserved() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, "art"),
            String::from_str(&env, "music"),
            String::from_str(&env, "art"),
        ],
    )
    .unwrap();

    let stored = categories(&env, &contract_id, hunt_id);
    assert_eq!(stored.len(), 2);
    assert_eq!(stored.get(0).unwrap(), String::from_str(&env, "art"));
    assert_eq!(stored.get(1).unwrap(), String::from_str(&env, "music"));
}

#[test]
fn more_than_five_distinct_categories_are_still_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    let result = set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, "a"),
            String::from_str(&env, "b"),
            String::from_str(&env, "c"),
            String::from_str(&env, "d"),
            String::from_str(&env, "e"),
            String::from_str(&env, "f"),
        ],
    );
    assert_eq!(result, Err(HuntErrorCode::TooManyCategories));

    // The rejected call must not partially apply its categories.
    let hunt = env.as_contract(&contract_id, || {
        HuntyCore::get_hunt_info(env.clone(), hunt_id).unwrap()
    });
    assert_eq!(hunt.categories.len(), 0);
    assert_eq!(hunt.status, HuntStatus::Draft);
}

#[test]
fn exactly_five_distinct_categories_are_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    set_categories(
        &env,
        &contract_id,
        hunt_id,
        &creator,
        vec![
            &env,
            String::from_str(&env, "a"),
            String::from_str(&env, "b"),
            String::from_str(&env, "c"),
            String::from_str(&env, "d"),
            String::from_str(&env, "e"),
        ],
    )
    .unwrap();

    assert_eq!(categories(&env, &contract_id, hunt_id).len(), 5);
}

#[test]
fn blank_categories_are_still_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(HuntyCore, ());
    let creator = Address::generate(&env);
    let hunt_id = create_hunt(&env, &contract_id, &creator);

    assert_eq!(
        set_categories(
            &env,
            &contract_id,
            hunt_id,
            &creator,
            vec![&env, String::from_str(&env, "   ")],
        ),
        Err(HuntErrorCode::InvalidCategory)
    );
}

use crate::{RewardErrorCode, RewardManager};
use reward_interface::RewardConfig;
use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, String};

/// Registers a mock token contract and returns (token_address, token_admin).
fn create_mock_token(env: &Env) -> (Address, Address) {
    let admin = Address::generate(env);
    let address = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (address, admin)
}

/// Returns a fresh address authorized to call `distribute_rewards`, which now
/// requires an explicitly authorized caller (#1061).
fn authorized_distributor(env: &Env) -> Address {
    let caller = Address::generate(env);
    crate::storage::Storage::add_authorized_contract(env, &caller);
    caller
}

/// Mints `amount` units of the token at `token_address` to `to`.
fn mint_tokens(env: &Env, token_address: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token_address).mint(to, &amount);
}

fn init_contract(env: &Env, admin: &Address, xlm_token: &Address) -> Address {
    let hunty_core = Address::generate(env);
    RewardManager::initialize(
        env.clone(),
        admin.clone(),
        xlm_token.clone(),
        hunty_core.clone(),
    )
    .unwrap();
    hunty_core
}

#[test]
fn test_create_pool_with_xlm_token() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        let result = RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            xlm_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        );

        assert!(result.is_ok());

        let config = RewardManager::get_pool_config(env.clone(), 1).unwrap();
        assert_eq!(config.token_address, xlm_token);
        assert_eq!(config.creator, creator);
    });
}

#[test]
fn test_create_pool_with_usdc_token() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let (usdc_token, _usdc_admin) = create_mock_token(&env);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        let result = RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            usdc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        );

        assert!(result.is_ok());

        let config = RewardManager::get_pool_config(env.clone(), 1).unwrap();
        assert_eq!(config.token_address, usdc_token);
    });
}

#[test]
fn test_create_multiple_pools_with_different_tokens() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let (usdc_token, _usdc_admin) = create_mock_token(&env);
    let (eurc_token, _eurc_admin) = create_mock_token(&env);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            xlm_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            2,
            usdc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            3,
            eurc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        let config1 = RewardManager::get_pool_config(env.clone(), 1).unwrap();
        assert_eq!(config1.token_address, xlm_token);

        let config2 = RewardManager::get_pool_config(env.clone(), 2).unwrap();
        assert_eq!(config2.token_address, usdc_token);

        let config3 = RewardManager::get_pool_config(env.clone(), 3).unwrap();
        assert_eq!(config3.token_address, eurc_token);
    });
}

#[test]
fn test_invalid_token_contract_rejected() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let invalid_token = Address::generate(&env);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        let result = RewardManager::create_reward_pool(
            env.clone(),
            creator.clone(),
            1,
            invalid_token,
            1,
            0,
            true,
        );

        assert_eq!(result, Err(RewardErrorCode::InvalidTokenContract));
    });
}

#[test]
fn test_fund_pool_uses_correct_token() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let (usdc_token, _usdc_admin) = create_mock_token(&env);

    mint_tokens(&env, &usdc_token, &creator, 50_000_000);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            usdc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        let fund_amount = 50_000_000i128;
        let result = RewardManager::fund_reward_pool(env.clone(), creator.clone(), 1, fund_amount);

        assert!(result.is_ok());

        let pool_status = RewardManager::get_reward_pool(env.clone(), 1).unwrap();
        assert_eq!(pool_status.balance, fund_amount);
        assert_eq!(pool_status.total_deposited, fund_amount);
    });
}

#[test]
fn test_distribute_rewards_uses_pool_token() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let player = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let (usdc_token, _usdc_admin) = create_mock_token(&env);

    mint_tokens(&env, &usdc_token, &creator, 100_000_000);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            usdc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 1, 100_000_000).unwrap();

        let reward_config = RewardConfig {
            xlm_amount: Some(10_000_000),
            nft_contract: None,
            nft_title: String::from_str(&env, ""),
            nft_description: String::from_str(&env, ""),
            nft_image_uri: String::from_str(&env, ""),
            nft_hunt_title: String::from_str(&env, ""),
            nft_rarity: 0,
            nft_tier: 0,
            completion_rank: 0,
        };

        let result = RewardManager::distribute_rewards(
            env.clone(),
            authorized_distributor(&env),
            1,
            player.clone(),
            reward_config,
        );

        assert!(result.is_ok());

        let pool_status = RewardManager::get_reward_pool(env.clone(), 1).unwrap();
        assert_eq!(pool_status.balance, 90_000_000);
        assert_eq!(pool_status.total_distributed, 10_000_000);
    });
}

#[test]
fn test_refund_pool_uses_correct_token() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let (xlm_token, _xlm_admin) = create_mock_token(&env);
    let (usdc_token, _usdc_admin) = create_mock_token(&env);

    mint_tokens(&env, &usdc_token, &creator, 50_000_000);

    env.as_contract(&contract_id, || {
        init_contract(&env, &admin, &xlm_token);

        RewardManager::create_reward_pool_with_nft(
            env.clone(),
            creator.clone(),
            1,
            usdc_token.clone(),
            0,
            Some(Address::generate(&env)),
            0,
            true,
        )
        .unwrap();

        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 1, 50_000_000).unwrap();

        let result = RewardManager::refund_pool(env.clone(), creator.clone(), 1);

        assert!(result.is_ok());

        let pool_status = RewardManager::get_reward_pool(env.clone(), 1).unwrap();
        assert_eq!(pool_status.balance, 0);
    });
}

/// Test that `distribute_batch` uses each entry's pool token, not the global XLM token.
///
/// Regression test for the bug where `distribute_batch` fetched `Storage::get_xlm_token`
/// once before the loop and used it for every transfer, regardless of `pool_config.token_address`.
/// A USDC pool paid out XLM while its internal USDC balance was debited.
#[test]
fn test_distribute_batch_uses_pool_token_not_global_xlm() {
    use crate::types::BatchDistributionEntry;
    use soroban_sdk::testutils::MockAuth;
    use soroban_sdk::Vec;

    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let player_xlm = Address::generate(&env);
    let player_usdc = Address::generate(&env);

    // Two distinct tokens: XLM (global) and USDC (pool-specific)
    let xlm_token = create_mock_token(&env);
    let usdc_token = create_mock_token(&env);

    env.as_contract(&contract_id, || {
        RewardManager::initialize(env.clone(), admin.clone(), xlm_token.clone()).unwrap();

        // Pool 1 — denominated in XLM
        RewardManager::create_reward_pool(
            env.clone(),
            creator.clone(),
            1,
            xlm_token.clone(),
            0,
        )
        .unwrap();

        // Pool 2 — denominated in USDC (different from the global XLM token)
        RewardManager::create_reward_pool(
            env.clone(),
            creator.clone(),
            2,
            usdc_token.clone(),
            0,
        )
        .unwrap();

        let fund_amount = 100_000_000i128;
        let reward_amount = 10_000_000i128;

        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 1, fund_amount).unwrap();
        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 2, fund_amount).unwrap();

        // Snapshot on-chain balances before the batch
        let xlm_client = soroban_sdk::token::Client::new(&env, &xlm_token);
        let usdc_client = soroban_sdk::token::Client::new(&env, &usdc_token);

        let contract_xlm_before = xlm_client.balance(&contract_id);
        let contract_usdc_before = usdc_client.balance(&contract_id);
        let player_xlm_before = xlm_client.balance(&player_xlm);
        let player_usdc_before = usdc_client.balance(&player_usdc);

        // Build a batch with one entry per pool
        let xlm_config = RewardConfig {
            xlm_amount: Some(reward_amount),
            nft_contract: None,
            nft_title: String::from_str(&env, ""),
            nft_description: String::from_str(&env, ""),
            nft_image_uri: String::from_str(&env, ""),
            nft_hunt_title: String::from_str(&env, ""),
            nft_rarity: 0,
            nft_tier: 0,
        };
        let usdc_config = RewardConfig {
            xlm_amount: Some(reward_amount),
            nft_contract: None,
            nft_title: String::from_str(&env, ""),
            nft_description: String::from_str(&env, ""),
            nft_image_uri: String::from_str(&env, ""),
            nft_hunt_title: String::from_str(&env, ""),
            nft_rarity: 0,
            nft_tier: 0,
        };

        let mut batch = Vec::new(&env);
        batch.push_back(BatchDistributionEntry {
            hunt_id: 1,
            player_address: player_xlm.clone(),
            reward_config: xlm_config,
        });
        batch.push_back(BatchDistributionEntry {
            hunt_id: 2,
            player_address: player_usdc.clone(),
            reward_config: usdc_config,
        });

        RewardManager::distribute_batch(env.clone(), batch).unwrap();

        // ── Assertions: token flows must match each pool's denomination ──

        // Pool 1 (XLM): XLM moved from contract to player_xlm
        assert_eq!(
            xlm_client.balance(&contract_id),
            contract_xlm_before - reward_amount,
            "Contract XLM balance should decrease by reward_amount"
        );
        assert_eq!(
            xlm_client.balance(&player_xlm),
            player_xlm_before + reward_amount,
            "player_xlm should receive XLM"
        );

        // Pool 2 (USDC): USDC moved from contract to player_usdc — NOT XLM
        assert_eq!(
            usdc_client.balance(&contract_id),
            contract_usdc_before - reward_amount,
            "Contract USDC balance should decrease by reward_amount"
        );
        assert_eq!(
            usdc_client.balance(&player_usdc),
            player_usdc_before + reward_amount,
            "player_usdc should receive USDC, not XLM"
        );

        // XLM must NOT have been sent to the USDC player (the old buggy behaviour)
        assert_eq!(
            xlm_client.balance(&player_usdc),
            0,
            "player_usdc must not receive any XLM (regression: batch used global XLM token)"
        );

        // USDC must NOT have been sent to the XLM player
        assert_eq!(
            usdc_client.balance(&player_xlm),
            0,
            "player_xlm must not receive any USDC"
        );

        // Internal pool balances should be decremented correctly
        let pool1 = RewardManager::get_reward_pool(env.clone(), 1).unwrap();
        assert_eq!(pool1.balance, fund_amount - reward_amount);

        let pool2 = RewardManager::get_reward_pool(env.clone(), 2).unwrap();
        assert_eq!(pool2.balance, fund_amount - reward_amount);
    });
}

/// Verify that a batch with only USDC entries never touches the XLM token at all.
#[test]
fn test_distribute_batch_usdc_only_does_not_move_xlm() {
    use crate::types::BatchDistributionEntry;
    use soroban_sdk::Vec;

    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(RewardManager, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let xlm_token = create_mock_token(&env);
    let usdc_token = create_mock_token(&env);

    env.as_contract(&contract_id, || {
        RewardManager::initialize(env.clone(), admin.clone(), xlm_token.clone()).unwrap();

        // Two pools, both USDC-denominated
        RewardManager::create_reward_pool(env.clone(), creator.clone(), 10, usdc_token.clone(), 0)
            .unwrap();
        RewardManager::create_reward_pool(env.clone(), creator.clone(), 11, usdc_token.clone(), 0)
            .unwrap();

        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 10, 50_000_000).unwrap();
        RewardManager::fund_reward_pool(env.clone(), creator.clone(), 11, 50_000_000).unwrap();

        let xlm_client = soroban_sdk::token::Client::new(&env, &xlm_token);
        let contract_xlm_before = xlm_client.balance(&contract_id);

        let reward_cfg = RewardConfig {
            xlm_amount: Some(5_000_000),
            nft_contract: None,
            nft_title: String::from_str(&env, ""),
            nft_description: String::from_str(&env, ""),
            nft_image_uri: String::from_str(&env, ""),
            nft_hunt_title: String::from_str(&env, ""),
            nft_rarity: 0,
            nft_tier: 0,
        };

        let mut batch = Vec::new(&env);
        batch.push_back(BatchDistributionEntry {
            hunt_id: 10,
            player_address: player1.clone(),
            reward_config: reward_cfg.clone(),
        });
        batch.push_back(BatchDistributionEntry {
            hunt_id: 11,
            player_address: player2.clone(),
            reward_config: reward_cfg,
        });

        RewardManager::distribute_batch(env.clone(), batch).unwrap();

        // XLM contract balance must be completely unchanged
        assert_eq!(
            xlm_client.balance(&contract_id),
            contract_xlm_before,
            "XLM contract balance must not change when all pools are USDC-denominated"
        );
        assert_eq!(
            xlm_client.balance(&player1),
            0,
            "player1 must not receive XLM from a USDC pool"
        );
        assert_eq!(
            xlm_client.balance(&player2),
            0,
            "player2 must not receive XLM from a USDC pool"
        );
    });
}

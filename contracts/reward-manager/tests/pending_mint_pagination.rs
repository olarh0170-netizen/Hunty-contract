use reward_manager::storage::Storage;
use reward_manager::{PendingNftMint, RewardManager};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};

#[test]
fn oversized_limit_saturates_and_is_capped() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let xlm_token = Address::generate(&env);
    let hunty_core = Address::generate(&env);
    let contract_id = env.register(RewardManager, (admin, xlm_token, hunty_core));

    env.as_contract(&contract_id, || {
        for hunt_id in 0..52 {
            let player = Address::generate(&env);
            let pending = PendingNftMint {
                hunt_id,
                player: player.clone(),
                nft_contract: Address::generate(&env),
                nft_title: String::from_str(&env, "Reward"),
                nft_description: String::from_str(&env, "Description"),
                nft_image_uri: String::from_str(&env, "https://example.com/reward.png"),
                nft_hunt_title: String::from_str(&env, "Hunt"),
                nft_rarity: 0,
                nft_tier: 0,
                completion_rank: 0,
            };
            Storage::set_pending_nft_mint(&env, hunt_id, &player, &pending);
        }

        let page = Storage::list_pending_nft_mints(&env, 1, u32::MAX);
        assert_eq!(page.len(), Storage::MAX_PENDING_NFT_PAGE_SIZE);
    });
}

#[test]
fn many_pending_mints_remain_indexed_and_removable() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let xlm_token = Address::generate(&env);
    let hunty_core = Address::generate(&env);
    let contract_id = env.register(RewardManager, (admin, xlm_token, hunty_core));
    let mut players = soroban_sdk::Vec::new(&env);

    env.as_contract(&contract_id, || {
        for hunt_id in 0..512 {
            let player = Address::generate(&env);
            let pending = PendingNftMint {
                hunt_id,
                player: player.clone(),
                nft_contract: Address::generate(&env),
                nft_title: String::from_str(&env, "R"),
                nft_description: String::from_str(&env, "D"),
                nft_image_uri: String::from_str(&env, "u"),
                nft_hunt_title: String::from_str(&env, "H"),
                nft_rarity: 0,
                nft_tier: 0,
                completion_rank: 0,
            };
            Storage::set_pending_nft_mint(&env, hunt_id, &player, &pending);
            players.push_back(player);
        }

        let first_page = Storage::list_pending_nft_mints(&env, 0, 50);
        let later_page = Storage::list_pending_nft_mints(&env, 450, 50);
        assert_eq!(first_page.len(), 50);
        assert_eq!(later_page.len(), 50);

        Storage::remove_pending_nft_mint(&env, 250, &players.get(250).unwrap());

        let remaining = Storage::list_pending_nft_mints(&env, 0, u32::MAX);
        assert_eq!(remaining.len(), 50);
        let moved_entry = Storage::list_pending_nft_mints(&env, 250, 1);
        assert_eq!(moved_entry.get(0).unwrap().hunt_id, 511);
        let tail = Storage::list_pending_nft_mints(&env, 500, u32::MAX);
        assert_eq!(tail.len(), 11);
        assert!(Storage::get_pending_nft_mint(&env, 250, &players.get(250).unwrap()).is_none());
    });
}

#[test]
fn legacy_pending_mint_list_migrates_without_loss() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let xlm_token = Address::generate(&env);
    let hunty_core = Address::generate(&env);
    let contract_id = env.register(RewardManager, (admin, xlm_token, hunty_core));

    env.as_contract(&contract_id, || {
        let mut legacy = soroban_sdk::Vec::new(&env);
        for hunt_id in [41, 42] {
            let pending = PendingNftMint {
                hunt_id,
                player: Address::generate(&env),
                nft_contract: Address::generate(&env),
                nft_title: String::from_str(&env, "Reward"),
                nft_description: String::from_str(&env, "Description"),
                nft_image_uri: String::from_str(&env, "uri"),
                nft_hunt_title: String::from_str(&env, "Hunt"),
                nft_rarity: 0,
                nft_tier: 0,
                completion_rank: 0,
            };
            legacy.push_back(pending);
        }
        env.storage()
            .persistent()
            .set(&soroban_sdk::symbol_short!("PNFTLST"), &legacy);

        let migrated = Storage::list_pending_nft_mints(&env, 0, 10);
        assert_eq!(migrated.len(), 2);
        assert_eq!(migrated.get(0).unwrap().hunt_id, 41);
        assert_eq!(migrated.get(1).unwrap().hunt_id, 42);
        assert!(!env
            .storage()
            .persistent()
            .has(&soroban_sdk::symbol_short!("PNFTLST")));
    });
}

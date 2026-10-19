use soroban_sdk::{symbol_short, Env};

/// Minimal persistent-storage helpers required by the migration module.
///
/// The NFT CRUD paths live in the contract impl; this module only exposes the
/// counter and per-NFT schema-version keys that `migration::NftRewardMigration`
/// reads and writes during a v0 → v1 data migration.
pub struct Storage;

impl Storage {
    const NFT_COUNTER_KEY: soroban_sdk::Symbol = symbol_short!("CN");
    const NFT_VERSION_KEY: soroban_sdk::Symbol = symbol_short!("NFTV");

    fn nft_version_key(nft_id: u64) -> (soroban_sdk::Symbol, u64) {
        (Self::NFT_VERSION_KEY, nft_id)
    }

    /// Current value of the collection-wide NFT id counter (0 when unset).
    pub fn get_nft_counter(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&Self::NFT_COUNTER_KEY)
            .unwrap_or(0)
    }

    /// True when an explicit schema-version key exists for `nft_id`.
    /// Used by migration to detect NFTs that still need a version assigned.
    pub fn has_nft_version_key(env: &Env, nft_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&Self::nft_version_key(nft_id))
    }

    /// Writes the metadata schema version for an NFT.
    pub fn set_nft_version(env: &Env, nft_id: u64, version: u32) {
        let key = Self::nft_version_key(nft_id);
        env.storage().persistent().set(&key, &version);
    }
}

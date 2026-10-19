use crate::METADATA_SCHEMA_VERSION;
use hunty_migration::{
    MigrationFramework, UpgradeAuthError, UpgradeAuthorization, UpgradeCancelledEvent,
    UpgradeExecutedEvent, UpgradeHistoryEntry, UpgradeProposal, UpgradeProposedEvent,
};
use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

pub use hunty_migration::MigrationReport;

const NFT_COUNTER_KEY: Symbol = symbol_short!("CN");
const NFT_VERSION_KEY: Symbol = symbol_short!("NFTV");

/// Per-contract migration steps for NftReward storage layouts.
pub struct NftRewardMigration;

impl NftRewardMigration {
    pub fn get_schema_version(env: &Env) -> u32 {
        MigrationFramework::detect_version(env)
    }

    pub fn initialize_schema(env: &Env, admin: &Address) {
        MigrationFramework::init_version_on_deploy(env);
        if UpgradeAuthorization::get_upgrade_admin(env).is_none() {
            UpgradeAuthorization::set_upgrade_admin(env, admin);
        }
    }

    pub fn propose_upgrade(
        env: &Env,
        admin: &Address,
        target_version: u32,
        wasm_hash: BytesN<32>,
    ) -> Result<UpgradeProposal, UpgradeAuthError> {
        UpgradeAuthorization::require_admin(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        let now = env.ledger().timestamp();
        UpgradeAuthorization::propose_upgrade(env, admin, target_version, wasm_hash, now)
    }

    pub fn upgrade(
        env: &Env,
        admin: &Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), UpgradeAuthError> {
        UpgradeAuthorization::require_admin(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        let now = env.ledger().timestamp();
        let proposal = UpgradeAuthorization::validate_upgrade(env, &new_wasm_hash, now)?;
        let from_version = MigrationFramework::detect_version(env);
        let to_version = proposal.target_version;

        // Run pending data-migration steps before swapping WASM so that
        // the stored data matches the layout expected by the new code.
        if from_version < to_version {
            MigrationFramework::save_rollback_point(env, from_version);
            let mut current = from_version;
            while current < to_version {
                match current {
                    0 => {
                        Self::migrate_v0_to_v1(env);
                        current = 1;
                    }
                    _ => {
                        return Err(UpgradeAuthError::VersionMismatch);
                    }
                }
            }
            MigrationFramework::set_version(env, current);
        }

        UpgradeAuthorization::finalize_upgrade_run(
            env,
            admin,
            from_version,
            to_version,
            &new_wasm_hash,
            now,
        );

        let event = Self::upgrade_executed_event(
            from_version,
            to_version,
            &new_wasm_hash,
            now,
            admin.clone(),
        );
        env.events()
            .publish(Self::upgrade_executed_topic(env), event);

        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Ok(())
    }

    /// Sets the upgrade timelock to `delay_seconds`.
    ///
    /// Rejects values below `MIN_UPGRADE_TIMELOCK_SECONDS` (24 hours). Timelock
    /// reductions only take effect after the current timelock elapses, so users
    /// keep the warning window they were promised.
    pub fn set_upgrade_timelock(
        env: &Env,
        admin: &Address,
        delay_seconds: u64,
    ) -> Result<(), UpgradeAuthError> {
        UpgradeAuthorization::require_admin(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        let now = env.ledger().timestamp();
        UpgradeAuthorization::set_timelock_seconds(env, now, delay_seconds)
    }

    pub fn get_upgrade_proposal(env: &Env) -> Option<UpgradeProposal> {
        UpgradeAuthorization::get_proposal(env)
    }

    pub fn get_upgrade_timelock(env: &Env) -> u64 {
        UpgradeAuthorization::get_timelock_seconds(env)
    }

    /// Returns the pending timelock reduction, if one is queued.
    #[allow(dead_code)]
    pub fn get_pending_timelock_change(env: &Env) -> Option<hunty_migration::TimelockChange> {
        UpgradeAuthorization::get_pending_timelock_change(env)
    }

    pub fn get_upgrade_history(
        env: &Env,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<UpgradeHistoryEntry> {
        let bounded_limit = limit.min(50);
        UpgradeAuthorization::get_history(env, offset, bounded_limit)
    }

    /// Runs migrations up to `target_version`. When `dry_run` is true, no storage writes occur.
    pub fn run_migration(
        env: &Env,
        admin: &Address,
        target_version: u32,
        dry_run: bool,
    ) -> Result<MigrationReport, UpgradeAuthError> {
        let now = env.ledger().timestamp();
        UpgradeAuthorization::prepare_migration_run(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
            target_version,
            dry_run,
            now,
        )?;

        let mut current = MigrationFramework::detect_version(env);
        if current >= target_version {
            return Ok(MigrationFramework::build_report(
                env,
                current,
                target_version,
                0,
                dry_run,
                true,
                "already at target",
            ));
        }

        if !dry_run {
            MigrationFramework::save_rollback_point(env, current);
        }

        let from_version = current;
        let mut steps = 0u32;
        while current < target_version {
            steps += 1;
            match current {
                0 => {
                    if !dry_run {
                        Self::migrate_v0_to_v1(env);
                    }
                    current = 1;
                }
                // v1 -> v2: not yet implemented.
                // Add the arm here (and the migrate_v1_to_v2 fn below) once
                // the new metadata storage layout is defined. Until then the
                // `_ =>` catchall correctly refuses to bump the version counter.
                _ => {
                    return Ok(MigrationFramework::build_report(
                        env,
                        MigrationFramework::detect_version(env),
                        target_version,
                        steps,
                        dry_run,
                        false,
                        "unsupported version step",
                    ));
                }
            }
        }

        if !dry_run {
            MigrationFramework::set_version(env, current);
            UpgradeAuthorization::finalize_migration_run(env, admin, from_version, current, now);
        }

        Ok(MigrationFramework::build_report(
            env,
            MigrationFramework::detect_version(env),
            target_version,
            steps,
            dry_run,
            true,
            "migration complete",
        ))
    }

    /// Restores the schema version saved before the last migration.
    pub fn rollback_migration(
        env: &Env,
        admin: &Address,
    ) -> Result<MigrationReport, UpgradeAuthError> {
        UpgradeAuthorization::require_admin(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        let previous =
            MigrationFramework::rollback_version(env).ok_or(UpgradeAuthError::NoRollbackPoint)?;
        let current = MigrationFramework::detect_version(env);
        MigrationFramework::set_version(env, previous);
        MigrationFramework::clear_rollback(env);
        Ok(MigrationFramework::build_report(
            env,
            current,
            previous,
            1,
            false,
            true,
            "rolled back",
        ))
    }

    pub fn upgrade_proposed_event(proposal: &UpgradeProposal) -> UpgradeProposedEvent {
        UpgradeProposedEvent {
            target_version: proposal.target_version,
            wasm_hash: proposal.wasm_hash.clone(),
            proposed_at: proposal.proposed_at,
            effective_at: proposal.effective_at,
            proposer: proposal.proposer.clone(),
        }
    }

    pub fn upgrade_executed_event(
        from_version: u32,
        to_version: u32,
        wasm_hash: &BytesN<32>,
        executed_at: u64,
        executor: Address,
    ) -> UpgradeExecutedEvent {
        UpgradeExecutedEvent {
            from_version,
            to_version,
            wasm_hash: wasm_hash.clone(),
            executed_at,
            executor,
        }
    }

    pub fn migration_executed_event(
        from_version: u32,
        to_version: u32,
        executed_at: u64,
        executor: Address,
    ) -> MigrationExecutedEvent {
        MigrationExecutedEvent {
            from_version,
            to_version,
            executed_at,
            executor,
        }
    }

    pub fn get_migration_history(
        env: &Env,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<MigrationHistoryEntry> {
        UpgradeAuthorization::get_migration_history(env, offset, limit)
    }

    pub fn upgrade_proposed_topic(env: &Env) -> (Symbol,) {
        (Symbol::new(env, "UpgradeProposed"),)
    }

    pub fn upgrade_executed_topic(env: &Env) -> (Symbol,) {
        (Symbol::new(env, "UpgradeExecuted"),)
    }

    pub fn upgrade_cancelled_topic(env: &Env) -> (Symbol,) {
        (Symbol::new(env, "UpgradeCancelled"),)
    }

    pub fn upgrade_cancelled_event(
        proposal: &UpgradeProposal,
        cancelled_by: Address,
        cancelled_at: u64,
    ) -> UpgradeCancelledEvent {
        UpgradeCancelledEvent {
            cancelled_at,
            proposer: proposal.proposer.clone(),
            cancelled_by,
            target_version: proposal.target_version,
            wasm_hash: proposal.wasm_hash.clone(),
        }
    }

    pub fn cancel_upgrade(
        env: &Env,
        admin: &Address,
    ) -> Result<UpgradeProposal, UpgradeAuthError> {
        UpgradeAuthorization::require_admin(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        let now = env.ledger().timestamp();
        let proposal = UpgradeAuthorization::cancel_upgrade(
            env,
            admin,
            UpgradeAuthorization::get_upgrade_admin(env),
        )?;
        env.events().publish(
            Self::upgrade_cancelled_topic(env),
            Self::upgrade_cancelled_event(&proposal, admin.clone(), now),
        );
        Ok(proposal)
    }

    /// v0 -> v1: retroactively set metadata version key on legacy NFTs.
    fn migrate_v0_to_v1(env: &Env) {
        let total: u64 = env
            .storage()
            .persistent()
            .get(&NFT_COUNTER_KEY)
            .unwrap_or(0);
        for nft_id in 1..=total {
            let version_key = (NFT_VERSION_KEY, nft_id);
            // Skip NFTs that already have an explicit version key.
            if env.storage().persistent().has(&version_key) {
                continue;
            }
            env.storage()
                .persistent()
                .set(&version_key, &METADATA_SCHEMA_VERSION);
        }
    }

    // v1 -> v2: NOT YET IMPLEMENTED.
    // Define migrate_v1_to_v2(env: &Env) here and add the corresponding
    // `1 => { ... current = 2; }` arm to run_migration once the new
    // metadata layout and transformation logic are ready.
    // Until then this step intentionally does not exist so run_migration
    // rejects any attempt to target version 2 rather than silently
    // bumping the stored schema counter without touching any data.
}

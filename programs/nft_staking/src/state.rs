use anchor_lang::prelude::*;

/// Global configuration for the staking program. There is one config per
/// deployment, tied to a single Metaplex Core collection.
#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    /// Authority that created the config.
    pub admin: Pubkey,
    /// Core collection whose assets can be staked here.
    pub collection: Pubkey,
    /// Reward points accrued per second for each staked asset.
    pub points_per_stake: u8,
    /// Maximum number of assets a single user may stake at once.
    pub max_stake: u8,
    /// Seconds an asset must remain staked before it can be unstaked.
    pub freeze_period: i64,
    /// Number of assets currently staked in the collection. Mirrored into the
    /// collection's on-chain Attributes plugin.
    pub staked_count: u32,
    /// Bump for the rewards mint PDA.
    pub rewards_bump: u8,
    /// Bump for the config PDA.
    pub bump: u8,
}

/// Per-user staking record.
#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    /// Reward points earned but not yet claimed.
    pub points: u64,
    /// Number of assets this user currently has staked.
    pub amount_staked: u8,
    /// Bump for the user PDA.
    pub bump: u8,
}

/// Per-asset staking record, created on stake and closed on unstake.
#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    /// Owner who staked the asset.
    pub owner: Pubkey,
    /// The staked Core asset.
    pub asset: Pubkey,
    /// Unix time the asset was staked. Used to enforce the freeze period.
    pub staked_at: i64,
    /// Unix time rewards were last settled for this asset.
    pub last_update: i64,
    /// Bump for the stake PDA.
    pub bump: u8,
}

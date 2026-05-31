use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod mpl_core;
pub mod state;

use instructions::*;

declare_id!("FvXZrMHU2wGF93Va4qHrCUCQmkkSiTkWMSq8RRYBViS4");

#[program]
pub mod nft_staking {
    use super::*;

    /// Create the program config and the rewards mint. One config per deployment,
    /// tied to a single Core collection.
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        points_per_stake: u8,
        max_stake: u8,
        freeze_period: i64,
    ) -> Result<()> {
        instructions::initialize_config::handler(ctx, points_per_stake, max_stake, freeze_period)
    }

    /// Attach an Attributes plugin to the collection to track its staked count.
    pub fn initialize_collection(ctx: Context<InitializeCollection>) -> Result<()> {
        instructions::initialize_collection::handler(ctx)
    }

    /// Create a staking record for a user.
    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        instructions::initialize_user::handler(ctx)
    }

    /// Stake an asset: freeze it and start accruing reward points.
    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        instructions::stake::handler(ctx)
    }

    /// Claim accrued reward points as tokens without unstaking the asset.
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        instructions::claim::handler(ctx)
    }

    /// Unstake an asset: thaw it, release the freeze, and close the record.
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        instructions::unstake::handler(ctx)
    }
}

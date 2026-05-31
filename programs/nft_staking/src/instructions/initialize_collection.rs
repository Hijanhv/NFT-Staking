use anchor_lang::prelude::*;

use crate::instructions::staked_attribute;
use crate::mpl_core::{self, MPL_CORE_ID};
use crate::state::StakeConfig;

/// Adds an Attributes plugin to the managed collection, seeded with a `staked`
/// counter set to zero and owned by the config PDA so the program can keep it
/// in sync as assets are staked and unstaked.
#[derive(Accounts)]
pub struct InitializeCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
        has_one = collection,
    )]
    pub config: Account<'info, StakeConfig>,

    /// CHECK: Core collection, validated by the Core program during the CPI.
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: current update authority of the collection; must sign to allow a
    /// new plugin to be added.
    pub update_authority: Signer<'info>,

    /// CHECK: address-checked against the known Core program id.
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeCollection>) -> Result<()> {
    let config_key = ctx.accounts.config.key();
    mpl_core::add_collection_attributes(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.update_authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        staked_attribute(0),
        config_key,
    )
}

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::state::StakeConfig;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [b"config"],
        bump,
        space = 8 + StakeConfig::INIT_SPACE,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(
        init,
        payer = admin,
        seeds = [b"rewards", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub rewards_mint: Account<'info, Mint>,

    /// CHECK: the Core collection this config manages. Its address is recorded
    /// and later checked with `has_one`; the Core program validates it on use.
    pub collection: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitializeConfig>,
    points_per_stake: u8,
    max_stake: u8,
    freeze_period: i64,
) -> Result<()> {
    ctx.accounts.config.set_inner(StakeConfig {
        admin: ctx.accounts.admin.key(),
        collection: ctx.accounts.collection.key(),
        points_per_stake,
        max_stake,
        freeze_period,
        staked_count: 0,
        rewards_bump: ctx.bumps.rewards_mint,
        bump: ctx.bumps.config,
    });
    Ok(())
}

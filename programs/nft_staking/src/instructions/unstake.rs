use anchor_lang::prelude::*;

use crate::error::StakeError;
use crate::instructions::{settle_rewards, staked_attribute};
use crate::mpl_core::{self, MPL_CORE_ID};
use crate::state::{StakeAccount, StakeConfig, UserAccount};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump,
        has_one = collection,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
        seeds = [b"user", owner.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        close = owner,
        seeds = [b"stake", asset.key().as_ref(), config.key().as_ref()],
        bump = stake_account.bump,
        has_one = owner,
        has_one = asset,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    /// CHECK: Core asset, validated by the Core program during the CPI.
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core collection, checked against the config via `has_one`.
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: address-checked against the known Core program id.
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // Settle any rewards earned since the last claim so they survive unstaking.
    settle_rewards(
        &mut ctx.accounts.user_account,
        &mut ctx.accounts.stake_account,
        ctx.accounts.config.points_per_stake,
        now,
    )?;

    let staked_duration = now.saturating_sub(ctx.accounts.stake_account.staked_at);
    require!(
        staked_duration >= ctx.accounts.config.freeze_period,
        StakeError::FreezePeriodNotElapsed
    );

    // Thaw then remove the freeze plugin, signed by the stake PDA.
    let asset_key = ctx.accounts.asset.key();
    let config_key = ctx.accounts.config.key();
    let stake_bump = [ctx.accounts.stake_account.bump];
    let seeds: [&[u8]; 4] = [
        b"stake",
        asset_key.as_ref(),
        config_key.as_ref(),
        &stake_bump,
    ];
    let signer: [&[&[u8]]; 1] = [&seeds];

    mpl_core::set_frozen(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.asset.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.stake_account.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        false,
        &signer,
    )?;
    // Once thawed, the owner-managed freeze plugin is removed by the asset owner.
    mpl_core::remove_freeze(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.asset.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &[],
    )?;

    ctx.accounts.user_account.amount_staked = ctx
        .accounts
        .user_account
        .amount_staked
        .checked_sub(1)
        .ok_or(StakeError::Overflow)?;
    ctx.accounts.config.staked_count = ctx
        .accounts
        .config
        .staked_count
        .checked_sub(1)
        .ok_or(StakeError::Overflow)?;

    // Mirror the new count into the collection's Attributes plugin.
    let count = ctx.accounts.config.staked_count;
    let config_bump = [ctx.accounts.config.bump];
    let config_seeds: [&[u8]; 2] = [b"config", &config_bump];
    let config_signer: [&[&[u8]]; 1] = [&config_seeds];

    mpl_core::update_collection_attributes(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.config.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        staked_attribute(count),
        &config_signer,
    )?;

    Ok(())
}

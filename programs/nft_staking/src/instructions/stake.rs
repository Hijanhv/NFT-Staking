use anchor_lang::prelude::*;

use crate::error::StakeError;
use crate::instructions::staked_attribute;
use crate::mpl_core::{self, MPL_CORE_ID};
use crate::state::{StakeAccount, StakeConfig, UserAccount};

#[derive(Accounts)]
pub struct Stake<'info> {
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
        init,
        payer = owner,
        seeds = [b"stake", asset.key().as_ref(), config.key().as_ref()],
        bump,
        space = 8 + StakeAccount::INIT_SPACE,
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

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    require!(
        u16::from(ctx.accounts.user_account.amount_staked) < u16::from(ctx.accounts.config.max_stake),
        StakeError::MaxStakeReached
    );

    let now = Clock::get()?.unix_timestamp;
    ctx.accounts.stake_account.set_inner(StakeAccount {
        owner: ctx.accounts.owner.key(),
        asset: ctx.accounts.asset.key(),
        staked_at: now,
        last_update: now,
        bump: ctx.bumps.stake_account,
    });

    // Freeze the asset and hand thaw rights to the stake PDA, so the owner can
    // no longer move it while it is staked but the program can release it later.
    let freeze_authority = ctx.accounts.stake_account.key();
    mpl_core::freeze_asset(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.asset.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        freeze_authority,
    )?;

    ctx.accounts.user_account.amount_staked = ctx
        .accounts
        .user_account
        .amount_staked
        .checked_add(1)
        .ok_or(StakeError::Overflow)?;
    ctx.accounts.config.staked_count = ctx
        .accounts
        .config
        .staked_count
        .checked_add(1)
        .ok_or(StakeError::Overflow)?;

    sync_collection_count(&ctx)?;
    Ok(())
}

/// Write the live staked count into the collection's Attributes plugin, signed
/// by the config PDA that owns the plugin.
fn sync_collection_count(ctx: &Context<Stake>) -> Result<()> {
    let count = ctx.accounts.config.staked_count;
    let bump = [ctx.accounts.config.bump];
    let seeds: [&[u8]; 2] = [b"config", &bump];
    let signer: [&[&[u8]]; 1] = [&seeds];

    mpl_core::update_collection_attributes(
        &ctx.accounts.mpl_core_program.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        &ctx.accounts.config.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        staked_attribute(count),
        &signer,
    )
}

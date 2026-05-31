use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};

use crate::error::StakeError;
use crate::instructions::settle_rewards;
use crate::state::{StakeAccount, StakeConfig, UserAccount};

/// Mints accrued reward points to the user as SPL tokens. This is independent
/// of unstaking: the asset stays frozen and keeps earning afterwards.
#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump,
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
        seeds = [b"stake", stake_account.asset.as_ref(), config.key().as_ref()],
        bump = stake_account.bump,
        has_one = owner,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(
        mut,
        seeds = [b"rewards", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
    )]
    pub rewards_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Claim>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    settle_rewards(
        &mut ctx.accounts.user_account,
        &mut ctx.accounts.stake_account,
        ctx.accounts.config.points_per_stake,
        now,
    )?;

    let points = ctx.accounts.user_account.points;
    require!(points > 0, StakeError::NothingToClaim);

    let amount = points
        .checked_mul(10u64.pow(u32::from(ctx.accounts.rewards_mint.decimals)))
        .ok_or(StakeError::Overflow)?;

    let bump = [ctx.accounts.config.bump];
    let seeds: [&[u8]; 2] = [b"config", &bump];
    let signer: [&[&[u8]]; 1] = [&seeds];

    let cpi = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.rewards_mint.to_account_info(),
            to: ctx.accounts.rewards_ata.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        },
        &signer,
    );
    mint_to(cpi, amount)?;

    ctx.accounts.user_account.points = 0;
    Ok(())
}

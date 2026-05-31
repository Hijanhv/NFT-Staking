// Each instruction module exposes a `handler` fn (always called by full path)
// plus its `#[derive(Accounts)]` context and the helper modules Anchor's
// `#[program]` macro expects at the crate root, so we re-export everything.
#![allow(ambiguous_glob_reexports)]

pub mod claim;
pub mod initialize_collection;
pub mod initialize_config;
pub mod initialize_user;
pub mod stake;
pub mod unstake;

pub use claim::*;
pub use initialize_collection::*;
pub use initialize_config::*;
pub use initialize_user::*;
pub use stake::*;
pub use unstake::*;

use crate::error::StakeError;
use crate::mpl_core::Attribute;
use crate::state::{StakeAccount, UserAccount};
use anchor_lang::prelude::*;

/// Build the single-entry attribute list the collection plugin stores.
pub fn staked_attribute(count: u32) -> Vec<Attribute> {
    vec![Attribute {
        key: "staked".to_string(),
        value: count.to_string(),
    }]
}

/// Move the points an asset has accrued since `last_update` into the user's
/// unclaimed balance, then advance `last_update`. Shared by claim and unstake.
pub fn settle_rewards(
    user: &mut UserAccount,
    stake: &mut StakeAccount,
    points_per_stake: u8,
    now: i64,
) -> Result<()> {
    let elapsed = now.saturating_sub(stake.last_update);
    if elapsed > 0 {
        let earned = (elapsed as u64)
            .checked_mul(points_per_stake as u64)
            .ok_or(StakeError::Overflow)?;
        user.points = user.points.checked_add(earned).ok_or(StakeError::Overflow)?;
        stake.last_update = now;
    }
    Ok(())
}

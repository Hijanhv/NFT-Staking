use anchor_lang::prelude::*;

#[error_code]
pub enum StakeError {
    #[msg("User has reached the maximum number of staked assets")]
    MaxStakeReached,
    #[msg("The freeze period has not elapsed yet")]
    FreezePeriodNotElapsed,
    #[msg("There are no reward points to claim")]
    NothingToClaim,
    #[msg("Arithmetic overflow")]
    Overflow,
}

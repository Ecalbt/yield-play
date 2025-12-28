use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid round seed")]
    InvalidRoundSeed,
    #[msg("Invalid start time")]
    InvalidStartTime,
    #[msg("Gap time invalid")]
    GapTimeInvalid,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Round not active")]
    RoundNotActive,
    #[msg("Invalid round")]
    InvalidRound,
    #[msg("Round not completed")]
    RoundNotCompleted,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("CPI to lending program failed")]
    CpiLendingProgramFailed,
    #[msg("Overflow occurred")]
    Overflow,
    #[msg("User does not participate in this round")]
    UserDoesNotParticipate,
    #[msg("Randomness account deserialization failed")]
    RandomnessAccountDeserializeFailed,
    #[msg("Randomness not fulfilled")]
    RandomnessNotFulfilled,
    #[msg("Invalid prize distribution")]
    InvalidPrizeDistribution,
    #[msg("No farmed amount to distribute")]
    NoFarmedAmount,
    #[msg("Nothing to claim")]
    NothingToClaim,
}

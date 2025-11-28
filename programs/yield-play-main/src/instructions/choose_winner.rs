use anchor_lang::{
    prelude::*,
    solana_program::clock::Clock,
};

use crate::{
    state::*,
    constant::*,
    errors::ErrorCode,
};

#[derive(Accounts)]
pub struct ChooseWinner<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,

    ///CHECK:
    pub first_prize: UncheckedAccount<'info>,
    ///CHECK:
    pub second_prize: UncheckedAccount<'info>,
    ///CHECK:
    pub third_prize: UncheckedAccount<'info>,


    pub system_program: Program<'info, System>,
}
impl<'info> ChooseWinner<'info> {
    pub fn process(ctx: Context<ChooseWinner>) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;
        let now_ts = Clock::get()?.unix_timestamp as u64;
        round_state.update_status(now_ts);
        require!(round_state.admin == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        require!(round_state.status == RoundStatus::ChoosingWinners.to_u8(), ErrorCode::RoundNotActive);


        msg!("now_ts: {}", now_ts);
        msg!("round_state.end_ts: {}", round_state.end_ts);
        msg!("round_state.status: {}", round_state.status);
        // Update round state with winners
        round_state.first_prize = ctx.accounts.first_prize.key();
        round_state.second_prize =  ctx.accounts.second_prize.key();
        round_state.third_prize =  ctx.accounts.third_prize.key();
        round_state.status = RoundStatus::RewardsDistributed.to_u8();

        Ok(())
    }
}
use anchor_lang::{
    prelude::*,
    solana_program::clock::Clock,
};

use crate::{
    state::*,
    constant::*,
    errors::ErrorCode,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ChooseWinnerArgs {
    first_prize_rate: u64,  //1000 = 1%
    second_prize_rate: u64, 
    third_prize_rate: u64,
}

#[derive(Accounts)]
#[instruction(arg: ChooseWinnerArgs)]
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

    #[account(
        init_if_needed,
        payer = authority,
        seeds = [USER_STATE_SEED, &round_state.round_id.to_le_bytes(), &first_prize.key().to_bytes()],
        bump,
        space = 8 + std::mem::size_of::<UserRoundState>()
    )]
    pub first_prize_round_state: Account<'info, UserRoundState>,

    #[account(
        init_if_needed,
        payer = authority,
        seeds = [USER_STATE_SEED, &round_state.round_id.to_le_bytes(), &second_prize.key().to_bytes()],
        bump,
        space = 8 + std::mem::size_of::<UserRoundState>()
    )]
    pub second_prize_round_state: Account<'info, UserRoundState>,

    #[account(
        init_if_needed,
        payer = authority,
        seeds = [USER_STATE_SEED, &round_state.round_id.to_le_bytes(), &third_prize.key().to_bytes()],
        bump,
        space = 8 + std::mem::size_of::<UserRoundState>()
    )]
    pub third_prize_round_state: Account<'info, UserRoundState>,

    pub system_program: Program<'info, System>,
}
impl<'info> ChooseWinner<'info> {
    pub fn process(ctx: Context<ChooseWinner>, arg: ChooseWinnerArgs) -> Result<()> {
        
        let round_state = &mut ctx.accounts.round_state;
        let now_ts = Clock::get()?.unix_timestamp as u64;
        let first_prize_round_state = &mut ctx.accounts.first_prize_round_state;
        let second_prize_round_state = &mut ctx.accounts.second_prize_round_state;
        let third_prize_round_state = &mut ctx.accounts.third_prize_round_state;

        first_prize_round_state.amount_to_claim = first_prize_round_state.deposit_amount; //1
        second_prize_round_state.amount_to_claim = second_prize_round_state.deposit_amount; // 1
        third_prize_round_state.amount_to_claim = third_prize_round_state.deposit_amount;

        round_state.update_status(now_ts);
        require!(round_state.admin == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        require!(round_state.status == RoundStatus::ChoosingWinners.to_u8(), ErrorCode::RoundNotActive);
        require!(first_prize_round_state.ticket_count > 0, ErrorCode::UserDoesNotParticipate);
        require!(second_prize_round_state.ticket_count > 0, ErrorCode::UserDoesNotParticipate);
        require!(third_prize_round_state.ticket_count > 0, ErrorCode::UserDoesNotParticipate);

        msg!("now_ts: {}", now_ts);
        msg!("round_state.end_ts: {}", round_state.end_ts);
        msg!("round_state.status: {}", round_state.status);
        // Update round state with winners
        round_state.first_prize = ctx.accounts.first_prize.key();
        round_state.second_prize =  ctx.accounts.second_prize.key();
        round_state.third_prize =  ctx.accounts.third_prize.key();
        let first_prize_percent = arg.first_prize_rate as f64 / 1000.0;
        let second_prize_percent = arg.second_prize_rate as f64 / 1000.0;
        let third_prize_percent = arg.third_prize_rate as f64 / 1000.0;

        require!(
            (first_prize_percent + second_prize_percent + third_prize_percent - 1.0).abs() < f64::EPSILON,
            ErrorCode::InvalidPrizeDistribution
        );

        let first_prize_bonus = (round_state.total_farmed_amount as f64 * first_prize_percent) as u64;
        let second_prize_bonus = (round_state.total_farmed_amount as f64 * second_prize_percent) as u64;
        let third_prize_bonus = (round_state.total_farmed_amount as f64 * third_prize_percent) as u64;


        if first_prize_round_state.key() == second_prize_round_state.key() {
            second_prize_round_state.amount_to_claim += second_prize_bonus + first_prize_bonus;
        } 

        if first_prize_round_state.key() == third_prize_round_state.key() {
            third_prize_round_state.amount_to_claim += third_prize_bonus + first_prize_bonus;
        } 

        if second_prize_round_state.key() == third_prize_round_state.key() {
            third_prize_round_state.amount_to_claim += third_prize_bonus + second_prize_bonus;
        }

        if first_prize_round_state.key() == second_prize_round_state.key() && first_prize_round_state.key() == third_prize_round_state.key() {
            first_prize_round_state.amount_to_claim += first_prize_bonus + second_prize_bonus + third_prize_bonus;
        }
        
        round_state.status = RoundStatus::RewardsDistributed.to_u8();

        Ok(())
    }
}
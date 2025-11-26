use anchor_lang::{
    prelude::*,
    solana_program::keccak::hash,
    solana_program::clock::Clock,
    system_program::{self, Transfer},
};

use crate::state::*;
use crate::constant::*; 
use crate::errors::ErrorCode;


#[derive(Accounts, AnchorDeserialize, AnchorSerialize)]
pub struct UpdatePrice<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [LOTTERY_STATE_SEED],
        bump,
    )]
    pub lottery_state: Account<'info, LotteryState>,

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,

    
    pub system_program: Program<'info, System>,
}
impl<'info> UpdatePrice<'info> {
    pub fn process(ctx: Context<UpdatePrice>) -> Result<()> {
        let lottery_state = &mut ctx.accounts.lottery_state;

        let round_state = &mut ctx.accounts.round_state;
        let now_ts = Clock::get()?.unix_timestamp as u64;

        round_state.update_status(now_ts);
        require!(round_state.admin == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
        require!(round_state.status == RoundStatus::Started.to_u8(), ErrorCode::RoundNotActive);

        // Convert start_ts and end_ts to u64 for safe calculation
        let start_ts = round_state.start_ts as u64;
        let end_ts = round_state.end_ts as u64;
        
        // Check if we haven't reached start time yet
        if now_ts < start_ts {  
            msg!(now_ts.to_string().as_str());
            msg!(start_ts.to_string().as_str());
            round_state.price_per_ticket = 1;
            return Ok(());
        }

        let elapsed_time = now_ts.saturating_sub(start_ts);
        let total_duration = end_ts.saturating_sub(start_ts);
        
        
        let rate: f64 = elapsed_time as f64 / total_duration as f64;
        let mut price = lottery_state.ticket_base_price as f64;
        price += (rate * lottery_state.ticket_price_jump as f64);
        round_state.price_per_ticket = price as u64;

        Ok(())
    }
}

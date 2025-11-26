use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ Mint, TokenAccount, TokenInterface };

use crate::state::LotteryState;
use crate::constant::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [LOTTERY_STATE_SEED],
        bump,
        space = 8 + std::mem::size_of::<LotteryState>(),
    )]
    pub lottery_state: Account<'info, LotteryState>,

    // pub associated_token_program: Program<'info, AssociatedToken>,
    // pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> Initialize<'info> {
    pub fn process(ctx: Context<Initialize>) -> Result<()> {
        let lottery_state = &mut ctx.accounts.lottery_state;
        lottery_state.admin = ctx.accounts.authority.key();
        lottery_state.global_round_counter = 0;
        lottery_state.is_pause = false;
        Ok(())
    }
}


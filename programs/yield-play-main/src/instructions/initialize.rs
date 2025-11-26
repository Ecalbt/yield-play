use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ Mint, TokenAccount, TokenInterface };

use crate::state::LotteryState;
use crate::constant::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitArgs {
    pub ticket_base_price: u64,
    pub ticket_price_jump: u64,
    pub ticket_time_jump: i64,
}
#[derive(Accounts)]
#[instruction(args: InitArgs)]
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
    pub fn process(ctx: Context<Initialize>, args: InitArgs) -> Result<()> {
        let lottery_state = &mut ctx.accounts.lottery_state;
        lottery_state.admin = ctx.accounts.authority.key();
        lottery_state.global_round_counter = 0;
        lottery_state.ticket_base_price = args.ticket_base_price;
        lottery_state.ticket_price_jump = args.ticket_price_jump;
        lottery_state.ticket_time_jump = args.ticket_time_jump;
        lottery_state.is_pause = false;
        Ok(())
    }
}


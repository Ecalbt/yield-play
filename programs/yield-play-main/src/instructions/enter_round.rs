use anchor_lang::{
    prelude::*,
    solana_program::clock::Clock,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenInterface, TokenAccount, Mint, TransferChecked, transfer_checked},
};

use crate::{
    constant::*,
    state::*,
    errors::ErrorCode,
};

#[event]
pub struct TicketPurchase {
    pub round_id: u64,
    pub user: Pubkey,
    pub ticket_start_index: u64,
    pub ticket_count: u64,
}
#[derive(Accounts)]
pub struct EnterRound<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

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

    #[account(
        init_if_needed,
        payer = user,
        seeds = [USER_STATE_SEED, &round_state.round_id.to_le_bytes(), &user.key().to_bytes()],
        bump,
        space = 8 + std::mem::size_of::<UserRoundState>()
    )]
    pub user_round_state: Account<'info, UserRoundState>,


    //vault to hold payment tokens deposited by users
    pub payment_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,  
        associated_token::mint = payment_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = user,
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK:
    #[account(
        seeds = [ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub vault_round_signer: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> EnterRound<'info> {
    pub fn process(ctx: Context<EnterRound>, amount: u64) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;
        let user_round_state = &mut ctx.accounts.user_round_state;
        let lottery_state = &ctx.accounts.lottery_state;
        let now_ts = Clock::get()?.unix_timestamp as u64;

        round_state.update_status(now_ts);
        require!(round_state.status == RoundStatus::Started.to_u8(), ErrorCode::RoundNotActive);

        if user_round_state.deposit_amount == 0 {
            user_round_state.user = ctx.accounts.user.key();
            user_round_state.round_id = round_state.round_id;
            user_round_state.deposit_amount = 0;
            user_round_state.ticket_count = 0;
            user_round_state.is_claimed = false;
        }

        
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.user_ata.to_account_info(),
            mint: ctx.accounts.payment_mint.to_account_info(),
            to: ctx.accounts.round_vault_ata.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        let token_amount = round_state.price_per_ticket
            .checked_mul(amount)
            .ok_or(ErrorCode::Overflow)? as u64;
        transfer_checked(cpi_ctx, token_amount, ctx.accounts.payment_mint.decimals)?;

        //update round state
        let ticket_start_index = round_state.total_tickets;
        round_state.total_deposit += token_amount;
        round_state.total_tickets += amount;

        //update user round state
        user_round_state.deposit_amount += token_amount;
        user_round_state.ticket_count += amount;

        emit!(TicketPurchase {
            round_id: round_state.round_id,
            user: ctx.accounts.user.key(),
            ticket_start_index,
            ticket_count: amount,
        });

        Ok(())
    }
}
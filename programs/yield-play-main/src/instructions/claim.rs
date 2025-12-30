use anchor_lang::{
    prelude::*,
    solana_program::program::{invoke_signed,invoke},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenInterface, TokenAccount, Mint},
};

use crate::{
    constant::*,
    state::*,
    errors::ErrorCode,
};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,

    /// CHECK: Vault round signer PDA
    #[account(
        seeds = [ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub vault_round_signer: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [USER_STATE_SEED, &round_state.round_id.to_le_bytes(), &user.key().to_bytes()],
        bump,
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

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Claim<'info> {
    pub fn process(ctx: Context<Claim>) -> Result<()> {
        let user_round_state = &mut ctx.accounts.user_round_state;
        let round_state = &mut ctx.accounts.round_state;


        require!(round_state.status == RoundStatus::RewardsDistributed.to_u8(), ErrorCode::RoundNotCompleted);
        require!(!user_round_state.is_claimed, ErrorCode::AlreadyClaimed);
        require!(round_state.payment_mint == ctx.accounts.payment_mint.key(), ErrorCode::InvalidPaymentMint);
        
        
        let mut amount_to_claim = user_round_state.amount_to_claim + user_round_state.deposit_amount;
        

        if amount_to_claim > 0 {
            let (_, vault_signer_bump) = Pubkey::find_program_address(
                &[ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
                ctx.program_id,
            );
            let signer_seeds: &[&[&[u8]]] = &[&[
                ROUND_VAULT_SIGNER_SEED,
                &round_state.round_id.to_le_bytes(),
                &[vault_signer_bump],
            ]];

            let cpi_accounts = anchor_spl::token_interface::Transfer {
                from: ctx.accounts.round_vault_ata.to_account_info(),
                to: ctx.accounts.user_ata.to_account_info(),
                authority: ctx.accounts.vault_round_signer.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
            anchor_spl::token_interface::transfer(cpi_ctx, amount_to_claim)?;
        }

        user_round_state.is_claimed = true;
        round_state.total_refunded += amount_to_claim;
        Ok(())
    }
}
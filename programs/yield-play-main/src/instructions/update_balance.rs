use anchor_lang::{
    prelude::*,
    solana_program::keccak::hash,
    solana_program::clock::Clock,
    system_program::{self, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenInterface, TokenAccount, Mint},
};
use crate::state::*;
use crate::constant::*; 
use crate::errors::ErrorCode;


#[derive(Accounts, AnchorDeserialize, AnchorSerialize)]
pub struct UpdateBalance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,

    // Payment mint and vault ATA
    pub payment_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Vault round signer PDA
    #[account(
        seeds = [ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub vault_round_signer: AccountInfo<'info>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> UpdateBalance<'info> {
    pub fn process(ctx: Context<UpdateBalance>) -> Result<()> {

        require!(
            ctx.accounts.authority.key() == ctx.accounts.round_state.admin,
            ErrorCode::Unauthorized
        );
        let round_state = &mut ctx.accounts.round_state;
        let vault_balance = ctx.accounts.round_vault_ata.amount;
        let vault_farmed = vault_balance.saturating_sub(round_state.total_deposit);
        let net_farmed = (vault_farmed as f64) * (1.0 - PERFORMANCE_FEE_RATE);
        round_state.performance_fee = vault_farmed.saturating_sub(net_farmed as u64);
        round_state.total_farmed_amount = net_farmed as u64;
        msg!("Vault balance: {}", vault_balance);
        msg!("Total deposit: {}", round_state.total_deposit);
        msg!("Vault farmed: {}", vault_farmed);
        msg!("Total farmed amount (after fee): {}", round_state.total_farmed_amount);

        Ok(())
    }
}

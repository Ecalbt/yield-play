use anchor_lang::{
    prelude::*,
    solana_program::program::{invoke_signed,invoke},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenInterface, TokenAccount, Mint},
};
use port_variable_rate_lending_instructions::instruction::{
    deposit_reserve_liquidity, 
    redeem_reserve_collateral, 
    refresh_reserve,
};

use crate::{
    constant::*,
    state::*,
    errors::ErrorCode,
};

#[derive(Accounts)]
pub struct DepositToLending<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

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

    // Source liquidity (payment tokens in round vault)
    pub payment_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

    // Destination collateral (tokens received from lending protocol)
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub collateral_vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Lending program ID (Port Finance)
    pub lending_program: AccountInfo<'info>,

    /// CHECK: Reserve account
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    /// CHECK: Reserve liquidity supply
    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,

    /// CHECK: Reserve collateral mint
    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    /// CHECK: Reserve liquidity oracle (Pyth or Switchboard)
    #[account(mut)]
    pub reserve_liquidity_oracle: AccountInfo<'info>,

    /// CHECK: Lending market
    pub lending_market: AccountInfo<'info>,

    /// CHECK: Lending market authority
    pub lending_market_authority: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositToLending<'info> {
    pub fn process(
        ctx: Context<'_, '_, '_, 'info, DepositToLending<'info>>,
        amount: u64,
    ) -> Result<()> {
        let round_state = &ctx.accounts.round_state;
        
        require!(
            round_state.admin == ctx.accounts.authority.key(),
            ErrorCode::Unauthorized
        );

        let (_, vault_signer_bump) = Pubkey::find_program_address(
            &[ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
            ctx.program_id,
        );

        let refresh_ix = refresh_reserve(
            *ctx.accounts.lending_program.key,
            *ctx.accounts.reserve.key,
            Some(*ctx.accounts.reserve_liquidity_oracle.key),
        );

        let refresh_accounts = vec![
            ctx.accounts.reserve.to_account_info(),
            ctx.accounts.reserve_liquidity_oracle.to_account_info(),
        ];

        invoke(&refresh_ix, &refresh_accounts)?;

        let deposit_ix = deposit_reserve_liquidity(
            *ctx.accounts.lending_program.key,
            amount,
            *ctx.accounts.round_vault_ata.to_account_info().key,
            *ctx.accounts.collateral_vault_ata.to_account_info().key,
            *ctx.accounts.reserve.key,
            *ctx.accounts.reserve_liquidity_supply.key,
            *ctx.accounts.reserve_collateral_mint.key,
            *ctx.accounts.lending_market.key,
            *ctx.accounts.lending_market_authority.key,
            *ctx.accounts.vault_round_signer.key,
        );

        let deposit_accounts = vec![
            ctx.accounts.round_vault_ata.to_account_info(),
            ctx.accounts.collateral_vault_ata.to_account_info(),
            ctx.accounts.reserve.to_account_info(),
            ctx.accounts.reserve_liquidity_supply.to_account_info(),
            ctx.accounts.reserve_collateral_mint.to_account_info(),
            ctx.accounts.lending_market.to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            ctx.accounts.vault_round_signer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ];

        let signer_seeds: &[&[&[u8]]] = &[&[
            ROUND_VAULT_SIGNER_SEED,
            &round_state.round_id.to_le_bytes(),
            &[vault_signer_bump],
        ]];

        invoke_signed(&deposit_ix, &deposit_accounts, signer_seeds)?;

        msg!("Deposited {} tokens to Port Finance", amount);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct WithdrawFromLending<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

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

    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub collateral_vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub payment_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Lending program ID (Port Finance)
    pub lending_program: AccountInfo<'info>,

    /// CHECK: Reserve account
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    /// CHECK: Reserve collateral mint
    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info>,

    /// CHECK: Reserve liquidity supply
    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,

    /// CHECK: Lending market
    pub lending_market: AccountInfo<'info>,

    /// CHECK: Lending market authority
    pub lending_market_authority: AccountInfo<'info>,

    /// CHECK: Reserve liquidity oracle
    #[account(mut)]
    pub reserve_liquidity_oracle: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawFromLending<'info> {
    pub fn process(
        ctx: Context<'_, '_, '_, 'info, WithdrawFromLending<'info>>,
        collateral_amount: u64,
    ) -> Result<()> {
        let round_state = &ctx.accounts.round_state;
        
        require!(
            round_state.admin == ctx.accounts.authority.key(),
            ErrorCode::Unauthorized
        );
        let (_, vault_signer_bump) = Pubkey::find_program_address(
            &[ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
            ctx.program_id,
        );

        let refresh_ix = refresh_reserve(
            *ctx.accounts.lending_program.key,
            *ctx.accounts.reserve.key,
            Some(*ctx.accounts.reserve_liquidity_oracle.key),
        );

        let refresh_accounts = vec![
            ctx.accounts.reserve.to_account_info(),
            ctx.accounts.reserve_liquidity_oracle.to_account_info(),
        ];

        anchor_lang::solana_program::program::invoke(&refresh_ix, &refresh_accounts)?;

        let redeem_ix = redeem_reserve_collateral(
            *ctx.accounts.lending_program.key,
            collateral_amount,
            *ctx.accounts.collateral_vault_ata.to_account_info().key,
            *ctx.accounts.round_vault_ata.to_account_info().key,
            *ctx.accounts.reserve.key,
            *ctx.accounts.reserve_collateral_mint.key,
            *ctx.accounts.reserve_liquidity_supply.key,
            *ctx.accounts.lending_market.key,
            *ctx.accounts.lending_market_authority.key,
            *ctx.accounts.vault_round_signer.key,
        );

        let redeem_accounts = vec![
            ctx.accounts.collateral_vault_ata.to_account_info(),
            ctx.accounts.round_vault_ata.to_account_info(),
            ctx.accounts.reserve.to_account_info(),
            ctx.accounts.reserve_collateral_mint.to_account_info(),
            ctx.accounts.reserve_liquidity_supply.to_account_info(),
            ctx.accounts.lending_market.to_account_info(),
            ctx.accounts.lending_market_authority.to_account_info(),
            ctx.accounts.vault_round_signer.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ];

        let signer_seeds: &[&[&[u8]]] = &[&[
            ROUND_VAULT_SIGNER_SEED,
            &round_state.round_id.to_le_bytes(),
            &[vault_signer_bump],
        ]];

        invoke_signed(&redeem_ix, &redeem_accounts, signer_seeds)?;

        msg!("Redeemed {} collateral tokens from Port Finance", collateral_amount);

        
        round_state.total_farmed_amount = ctx.accounts.round_vault_ata.amount - round_state.total_deposit;
        
        Ok(())
    }
}

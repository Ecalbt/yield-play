use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
	instruction::{AccountMeta, Instruction},
	program::invoke_signed,
};
use anchor_spl::{
	associated_token::AssociatedToken,
	token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
	constant::*,
	errors::ErrorCode,
	state::*,
};

/// sha256("global:deposit")[0..8]
fn deposit_discriminator() -> [u8; 8] {
	[242, 35, 198, 137, 82, 225, 242, 182]
}

/// sha256("global:withdraw")[0..8] (from Jupiter Lend IDL)
fn withdraw_discriminator() -> [u8; 8] {
	[183, 18, 70, 156, 148, 109, 161, 34]
}

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

	/// CHECK: PDA that owns the round vault
	#[account(
		seeds = [ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
		bump,
	)]
	pub vault_round_signer: AccountInfo<'info>,

	pub payment_mint: InterfaceAccount<'info, Mint>,

	#[account(
		mut,
		associated_token::mint = payment_mint,
		associated_token::authority = vault_round_signer,
	)]
	pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

	/// CHECK: ATA (or wallet) that receives the f-token collateral
	#[account(mut)]
	pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

	/// CHECK: Jupiter Lend configuration/admin account
	pub lending_admin: AccountInfo<'info>,

	/// CHECK: Jupiter Lend state account
	#[account(mut)]
	pub lending: AccountInfo<'info>,

	/// CHECK: Mint for the deposit receipt token
	#[account(mut)]
	pub f_token_mint: AccountInfo<'info>,

	/// CHECK: Liquidity reserve account that holds deposited tokens
	#[account(mut)]
	pub supply_token_reserves_liquidity: AccountInfo<'info>,

	/// CHECK: Position account tracking this pool's liquidity
	#[account(mut)]
	pub lending_supply_position_on_liquidity: AccountInfo<'info>,

	/// CHECK: Rate model describing how interest accrues
	pub rate_model: AccountInfo<'info>,

	/// CHECK: Vault account used internally by Jupiter Lend
	#[account(mut)]
	pub vault: AccountInfo<'info>,

	/// CHECK: Liquidity account used in the CPI
	#[account(mut)]
	pub liquidity: AccountInfo<'info>,

	/// CHECK: Liquidity program invoked by Jupiter Lend
	pub liquidity_program: AccountInfo<'info>,

	/// CHECK: Rewards model account
	pub rewards_rate_model: AccountInfo<'info>,

	pub token_program: Interface<'info, TokenInterface>,
	pub associated_token_program: Program<'info, AssociatedToken>,
	pub system_program: Program<'info, System>,

	/// CHECK: Jupiter Lend program ID
	pub lending_program: AccountInfo<'info>,
}


impl<'info> DepositToLending<'info> {
	pub fn process(ctx: Context<DepositToLending>) -> Result<()> {
		let round_state = &ctx.accounts.round_state;

		require!(round_state.admin == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
		let amount = ctx.accounts.round_vault_ata.amount;
		require!(amount > 0, ErrorCode::InvalidAmount);

		let round_id_bytes = round_state.round_id.to_le_bytes();
		let (_, vault_signer_bump) = Pubkey::find_program_address(
			&[ROUND_VAULT_SIGNER_SEED, &round_id_bytes],
			ctx.program_id,
		);

		let signer_seeds: &[&[&[u8]]] = &[&[
			ROUND_VAULT_SIGNER_SEED,
			&round_id_bytes,
			&[vault_signer_bump],
		]];

		let mut data = deposit_discriminator().to_vec();
		data.extend_from_slice(&amount.to_le_bytes());

		let accounts = vec![
			AccountMeta::new(*ctx.accounts.vault_round_signer.key, true),
			AccountMeta::new(ctx.accounts.round_vault_ata.key(), false),
			AccountMeta::new(ctx.accounts.recipient_token_account.key(), false),
			AccountMeta::new(ctx.accounts.payment_mint.key(), false),
			AccountMeta::new_readonly(*ctx.accounts.lending_admin.key, false),
			AccountMeta::new(*ctx.accounts.lending.key, false),
			AccountMeta::new(*ctx.accounts.f_token_mint.key, false),
			AccountMeta::new(*ctx.accounts.supply_token_reserves_liquidity.key, false),
			AccountMeta::new(*ctx.accounts.lending_supply_position_on_liquidity.key, false),
			AccountMeta::new_readonly(*ctx.accounts.rate_model.key, false),
			AccountMeta::new(*ctx.accounts.vault.key, false),
			AccountMeta::new(*ctx.accounts.liquidity.key, false),
			AccountMeta::new_readonly(*ctx.accounts.liquidity_program.key, false),
			AccountMeta::new_readonly(*ctx.accounts.rewards_rate_model.key, false),
			AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
			AccountMeta::new_readonly(ctx.accounts.associated_token_program.key(), false),
			AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
		];

		let instruction = Instruction {
			program_id: *ctx.accounts.lending_program.key,
			accounts,
			data,
		};

		invoke_signed(
			&instruction,
			&[
				ctx.accounts.vault_round_signer.clone(),
				ctx.accounts.round_vault_ata.to_account_info(),
				ctx.accounts.recipient_token_account.to_account_info(),
				ctx.accounts.payment_mint.to_account_info(),
				ctx.accounts.lending_admin.clone(),
				ctx.accounts.lending.clone(),
				ctx.accounts.f_token_mint.clone(),
				ctx.accounts.supply_token_reserves_liquidity.clone(),
				ctx.accounts.lending_supply_position_on_liquidity.clone(),
				ctx.accounts.rate_model.clone(),
				ctx.accounts.vault.clone(),
				ctx.accounts.liquidity.clone(),
				ctx.accounts.liquidity_program.clone(),
				ctx.accounts.rewards_rate_model.clone(),
				ctx.accounts.token_program.to_account_info(),
				ctx.accounts.associated_token_program.to_account_info(),
				ctx.accounts.system_program.to_account_info(),
			],
			signer_seeds,
		)
		.map_err(|e| {
			msg!("Jupiter Lend deposit CPI failed: {:?}", e);
			error!(ErrorCode::CpiLendingProgramFailed)
		})?;

		msg!(
			"Deposited entire round vault ({} tokens) from round {} into Jupiter Lend",
			amount,
			round_state.round_id
		);
        
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

	/// CHECK: PDA signer for all vault interactions
	#[account(
		seeds = [ROUND_VAULT_SIGNER_SEED, &round_state.round_id.to_le_bytes()],
		bump,
	)]
	pub vault_round_signer: AccountInfo<'info>,

	pub payment_mint: InterfaceAccount<'info, Mint>,

	#[account(
		mut,
		associated_token::mint = payment_mint,
		associated_token::authority = vault_round_signer,
	)]
	pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

	#[account(
		mut,
		associated_token::mint = f_token_mint,
		associated_token::authority = vault_round_signer,
	)]
	pub collateral_token_account: InterfaceAccount<'info, TokenAccount>,

	/// CHECK: Jupiter Lend configuration/admin account
	pub lending_admin: AccountInfo<'info>,

	/// CHECK: Jupiter Lend state account
	#[account(mut)]
	pub lending: AccountInfo<'info>,

	/// CHECK: Mint for the deposit receipt token
	#[account(mut)]
	pub f_token_mint: AccountInfo<'info>,

	/// CHECK: Liquidity reserve account that holds deposited tokens
	#[account(mut)]
	pub supply_token_reserves_liquidity: AccountInfo<'info>,

	/// CHECK: Position account tracking this pool's liquidity
	#[account(mut)]
	pub lending_supply_position_on_liquidity: AccountInfo<'info>,

	/// CHECK: Rate model describing how interest accrues
	pub rate_model: AccountInfo<'info>,

	/// CHECK: Vault account used internally by Jupiter Lend
	#[account(mut)]
	pub vault: AccountInfo<'info>,

    /// CHECK: Claim account PDA theo tài liệu CPI của Jupiter Lend
    #[account(mut)]
    pub claim_account: AccountInfo<'info>,

	/// CHECK: Liquidity account used in the CPI
	#[account(mut)]
	pub liquidity: AccountInfo<'info>,

	/// CHECK: Liquidity program invoked by Jupiter Lend
	pub liquidity_program: AccountInfo<'info>,

	/// CHECK: Rewards model account
	pub rewards_rate_model: AccountInfo<'info>,

	pub token_program: Interface<'info, TokenInterface>,
	pub associated_token_program: Program<'info, AssociatedToken>,
	pub system_program: Program<'info, System>,

	/// CHECK: Jupiter Lend program ID
	pub lending_program: AccountInfo<'info>,
}

impl<'info> WithdrawFromLending<'info> {
	pub fn process(ctx: Context<WithdrawFromLending>) -> Result<()> {
		let round_state = &mut ctx.accounts.round_state;

		require!(round_state.admin == ctx.accounts.authority.key(), ErrorCode::Unauthorized);

		let collateral_amount = ctx.accounts.collateral_token_account.amount;
		require!(collateral_amount > 0, ErrorCode::InvalidAmount);

		let round_id_bytes = round_state.round_id.to_le_bytes();
		let (_, vault_signer_bump) = Pubkey::find_program_address(
			&[ROUND_VAULT_SIGNER_SEED, &round_id_bytes],
			ctx.program_id,
		);

		let signer_seeds: &[&[&[u8]]] = &[&[
			ROUND_VAULT_SIGNER_SEED,
			&round_id_bytes,
			&[vault_signer_bump],
		]];

		let mut data = withdraw_discriminator().to_vec();
		data.extend_from_slice(&collateral_amount.to_le_bytes());

		let accounts = vec![
			AccountMeta::new(*ctx.accounts.vault_round_signer.key, true),
			AccountMeta::new(ctx.accounts.collateral_token_account.key(), false),
			AccountMeta::new(ctx.accounts.round_vault_ata.key(), false),
			AccountMeta::new(ctx.accounts.payment_mint.key(), false),
			AccountMeta::new_readonly(*ctx.accounts.lending_admin.key, false),
			AccountMeta::new(*ctx.accounts.lending.key, false),
			AccountMeta::new(*ctx.accounts.f_token_mint.key, false),
			AccountMeta::new(*ctx.accounts.supply_token_reserves_liquidity.key, false),
			AccountMeta::new(*ctx.accounts.lending_supply_position_on_liquidity.key, false),
			AccountMeta::new_readonly(*ctx.accounts.rate_model.key, false),
			AccountMeta::new(*ctx.accounts.vault.key, false),
			AccountMeta::new(*ctx.accounts.claim_account.key, false),
			AccountMeta::new(*ctx.accounts.liquidity.key, false),
			AccountMeta::new_readonly(*ctx.accounts.liquidity_program.key, false),
			AccountMeta::new_readonly(*ctx.accounts.rewards_rate_model.key, false),
			AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
			AccountMeta::new_readonly(ctx.accounts.associated_token_program.key(), false),
			AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
		];

		let instruction = Instruction {
			program_id: *ctx.accounts.lending_program.key,
			accounts,
			data,
		};

		invoke_signed(
			&instruction,
			&[
				ctx.accounts.vault_round_signer.clone(),
				ctx.accounts.collateral_token_account.to_account_info(),
				ctx.accounts.round_vault_ata.to_account_info(),
				ctx.accounts.payment_mint.to_account_info(),
				ctx.accounts.lending_admin.clone(),
				ctx.accounts.lending.clone(),
				ctx.accounts.f_token_mint.clone(),
				ctx.accounts.supply_token_reserves_liquidity.clone(),
				ctx.accounts.lending_supply_position_on_liquidity.clone(),
				ctx.accounts.rate_model.clone(),
				ctx.accounts.vault.clone(),
				ctx.accounts.claim_account.clone(),
				ctx.accounts.liquidity.clone(),
				ctx.accounts.liquidity_program.clone(),
				ctx.accounts.rewards_rate_model.clone(),
				ctx.accounts.token_program.to_account_info(),
				ctx.accounts.associated_token_program.to_account_info(),
				ctx.accounts.system_program.to_account_info(),
		],
			signer_seeds,
		)
		.map_err(|e| {
			msg!("Jupiter Lend withdraw CPI failed: {:?}", e);
			error!(ErrorCode::CpiLendingProgramFailed)
		})?;

		ctx.accounts.round_vault_ata.reload()?; // cập nhật lại số dư sau khi CPI trả token về
		let current_vault_balance = ctx.accounts.round_vault_ata.amount;
		round_state.total_farmed_amount = current_vault_balance
			.saturating_sub(round_state.total_deposit);

		msg!(
			"Withdrew {} collateral tokens back to vault for round {}",
			collateral_amount,
			round_state.round_id
		);

		Ok(())
	}
}

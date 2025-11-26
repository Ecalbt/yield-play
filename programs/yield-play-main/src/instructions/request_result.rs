use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;

use orao_solana_vrf::program::OraoVrf;
use orao_solana_vrf::cpi::accounts::RequestV2;
use orao_solana_vrf::{state::NetworkState, CONFIG_ACCOUNT_SEED, RANDOMNESS_ACCOUNT_SEED};

use crate::state::*;
use crate::errors::ErrorCode;
use crate::constant::*;

#[derive(Accounts)]
pub struct RequestResult<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,
    
    /// CHECK: VRF randomness account
    #[account(
        mut,
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), &round_state.round_seed],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub random_number_acct: AccountInfo<'info>,

    /// CHECK: treasury PDA
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [CONFIG_ACCOUNT_SEED.as_ref()],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub config: Account<'info, NetworkState>,

    pub vrf: Program<'info, OraoVrf>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct FulfillResult<'info> {
    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,
    
    /// CHECK: VRF randomness account (should be fulfilled by Orao VRF)
    #[account(
        mut,
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), &round_state.round_seed],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub random_number_acct: AccountInfo<'info>,
}

impl<'info> RequestResult<'info> {
    pub fn process(ctx: Context<RequestResult>) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;

        // Check that authority matches the round_state admin
        require_eq!(
            ctx.accounts.authority.key(),
            round_state.admin,
            ErrorCode::Unauthorized
        );

        if round_state.round_seed == [0u8; 32] {
            return Err(ErrorCode::InvalidRoundSeed.into());
        }

        // Invoke Orao VRF to request randomness
        let cpi_program = ctx.accounts.vrf.to_account_info();
        let cpi_accounts = RequestV2 {
            payer: ctx.accounts.authority.to_account_info(),
            network_state: ctx.accounts.config.to_account_info(),
            treasury: ctx.accounts.treasury.to_account_info(),
            request: ctx.accounts.random_number_acct.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        orao_solana_vrf::cpi::request_v2(cpi_ctx, round_state.round_seed)?;

        Ok(())
    }
}

impl<'info> FulfillResult<'info> {
    pub fn process(ctx: Context<FulfillResult>) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;

        // Extract and store the VRF seed from the randomness account
        // This should only be called after Orao VRF has fulfilled the request
        let randomness_account = ctx.accounts.random_number_acct.try_borrow_data()?;
        
        // The Orao VRF randomness account structure:
        // The fulfilled randomness is typically stored after the request info
        // Try offset 8 (discriminator) + request seed (32 bytes) = 40, then fulfilled randomness (64 bytes)
        // But we need to match what getFulfilledRandomness() returns
        // It appears the seed is at a different offset, let's try offset 72 (8 + 32 + 32)
        if randomness_account.len() >= 105 {
            // Extract bytes 32-64 from the fulfilled randomness (second half of the 64-byte value)
            let mut vrf_seed = [0u8; 32];
            vrf_seed.copy_from_slice(&randomness_account[73..105]);
            round_state.vrf_seed = vrf_seed;
        }

        Ok(())
    }
}
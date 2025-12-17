use anchor_lang::prelude::*;

use crate::{
    constant::*,
    state::*,
    errors::ErrorCode,
};

#[derive(Accounts, AnchorDeserialize, AnchorSerialize)]
pub struct NominateAuthority<'info> {
    #[account(mut)]
    pub current_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,
}

impl<'info> NominateAuthority<'info> {
    pub fn process(ctx: Context<NominateAuthority>, new_authority: Pubkey) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;

        require!(round_state.admin == ctx.accounts.current_authority.key(), ErrorCode::Unauthorized);

        round_state.pending_admin = new_authority;

        Ok(())
    }
}
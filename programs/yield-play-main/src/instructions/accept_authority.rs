use anchor_lang::prelude::*;
use crate::{
    constant::*,
    state::*,
    errors::ErrorCode,
};

#[derive(Accounts, AnchorDeserialize, AnchorSerialize)]
pub struct AcceptAuthority<'info> {
    #[account(mut)]
    pub pending_authority: Signer<'info>,  

    #[account(
        mut,
        seeds = [ROUND_STATE_SEED, &round_state.round_id.to_le_bytes()],
        bump,
    )]
    pub round_state: Account<'info, RoundState>,
}

impl<'info> AcceptAuthority<'info> {
    pub fn process(ctx: Context<AcceptAuthority>) -> Result<()> {
        let round_state = &mut ctx.accounts.round_state;

        require!(round_state.pending_admin == ctx.accounts.pending_authority.key(), ErrorCode::Unauthorized);

        round_state.admin = round_state.pending_admin;
        round_state.pending_admin = Pubkey::default();

        Ok(())
    }
}
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateArgs {
    //pub payment_mint: Option<Pubkey>, // None=SOL, Some(mint)=SPL
    pub round_id: u64,
    pub start_ts: i64,
    pub end_ts: i64,
    pub gap_time: i64,
    pub ticket_base_price: u64,
    pub ticket_price_jump: u64,

}
#[derive(Accounts, AnchorDeserialize, AnchorSerialize)]
#[instruction(arg: CreateArgs)]
pub struct CreateRound<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [LOTTERY_STATE_SEED],
        bump,
    )]
    pub lottery_state: Account<'info, LotteryState>,

    #[account(
        init,
        payer = authority,
        seeds = [ROUND_STATE_SEED, &lottery_state.global_round_counter.to_le_bytes()],
        bump,
        space = 8 + std::mem::size_of::<RoundState>(),
    )]
    pub round_state: Account<'info, RoundState>,

    /// CHECK: Vault round signer PDA
    #[account(
        seeds = [ROUND_VAULT_SIGNER_SEED, &lottery_state.global_round_counter.to_le_bytes()],
        bump,
    )]
    pub vault_round_signer: AccountInfo<'info>,

    // Payment mint and vault ATA
    pub payment_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub round_vault_ata: InterfaceAccount<'info, TokenAccount>,

    // Destination mint and ATA
    pub destination_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = destination_mint,
        associated_token::authority = vault_round_signer,
    )]
    pub destination_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> CreateRound<'info> {
    pub fn process(ctx: Context<CreateRound>, arg: CreateArgs) -> Result<()> {
        let lottery_state = &mut ctx.accounts.lottery_state;

        let round_state = &mut ctx.accounts.round_state;
        let now_ts = Clock::get()?.unix_timestamp as u64;
        msg!("Creating round at timestamp: {}", now_ts);
        msg!("With args: start_ts: {}, end_ts: {}, gap_time: {}", arg.start_ts, arg.end_ts, arg.gap_time);
        require!((arg.start_ts) as u64 >= now_ts, ErrorCode::InvalidStartTime);
        require!(arg.end_ts as u64 > arg.start_ts as u64, ErrorCode::GapTimeInvalid);

        round_state.admin = ctx.accounts.authority.key();
        //round_state.payment_mint = arg.payment_mint; // Default to SOL, can be updated later
        round_state.round_id = lottery_state.global_round_counter;

        // Compute round_seed = keccak(unix_timestamp || global_round_counter)
        let mut seed_data = Vec::new();
        seed_data.extend_from_slice(&now_ts.to_le_bytes());
        seed_data.extend_from_slice(&lottery_state.global_round_counter.to_le_bytes());
        let seed_hash = hash(&seed_data);

        round_state.price_per_ticket = arg.ticket_base_price;
        round_state.round_seed = seed_hash.0;
        
        round_state.vrf_seed = [0u8; 32];   

        round_state.total_deposit = 0;
        round_state.total_refunded = 0;
        round_state.total_farmed_amount = 0;
        round_state.total_tickets = 0;

        round_state.start_ts = arg.start_ts; 
        round_state.end_ts = arg.end_ts;   
        round_state.gap_time = arg.gap_time; 

        round_state.ticket_base_price = arg.ticket_base_price;
        round_state.ticket_price_jump = arg.ticket_price_jump;

        round_state.status = 0; // Placeholder for enum status

        round_state.first_prize = Pubkey::default();
        round_state.second_prize = Pubkey::default();   
        round_state.third_prize = Pubkey::default();


        lottery_state.global_round_counter += 1;
        // Increment global round counter

        Ok(())
    }
}

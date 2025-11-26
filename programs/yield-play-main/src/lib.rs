use anchor_lang::prelude::*;

pub mod state;
pub mod constant;
pub mod instructions;
pub mod errors;


pub use instructions::*;


declare_id!("7Q1x87fvJii5EvgqeJPw8MWJXRVrgsqyvr4QpUqVpBAY");

#[program]
pub mod yield_play_main {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitArgs) -> Result<()> {
        Initialize::process(ctx, args)
    }

    pub fn create_round(ctx: Context<CreateRound>, arg: CreateArgs) -> Result<()> {
        CreateRound::process(ctx, arg)
    }

    pub fn request_result(ctx: Context<RequestResult>) -> Result<()> {
        RequestResult::process(ctx)
    }

    pub fn fulfill_result(ctx: Context<FulfillResult>) -> Result<()> {
        FulfillResult::process(ctx)
    }

    pub fn enter_round(ctx: Context<EnterRound>, amount: u64) -> Result<()> {
        EnterRound::process(ctx, amount)
    }
    
    pub fn choose_winner(ctx: Context<ChooseWinner>) -> Result<()> {
        ChooseWinner::process(ctx)
    }

    pub fn update_price(ctx: Context<UpdatePrice>) -> Result<()> {
        UpdatePrice::process(ctx)
    }

    pub fn deposit_to_lending(ctx: Context<DepositToLending>, amount: u64) -> Result<()> {
        DepositToLending::process(ctx, amount)
    }

    pub fn withdraw_from_lending(ctx: Context<WithdrawFromLending>, collateral_amount: u64) -> Result<()> {
        WithdrawFromLending::process(ctx, collateral_amount)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        Claim::process(ctx)
    }
}   



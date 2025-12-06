use anchor_lang::prelude::*;

#[account]
pub struct LotteryState {
    pub admin: Pubkey,
    pub global_round_counter: u64,
    pub is_pause: bool
}

#[account]
pub struct RoundState {
    pub admin: Pubkey,
    pub round_id: u64,

    pub round_seed: [u8; 32],         // (=keccak(unix_timestamp || global_round_counter)
    pub vrf_seed: [u8; 32],        //(seed lấy từ OraoVRF) 

    pub ticket_base_price: u64,
    pub ticket_price_jump: u64,
    pub price_per_ticket: u64,
    
    pub total_deposit: u64,
    pub total_refunded: u64,
    pub total_farmed_amount: u64,    // tổng số tiền farm được trong round
    pub total_tickets: u64,

    pub start_ts: i64,
    pub end_ts: i64, 
    pub gap_time: i64,

    pub first_prize: Pubkey,
    pub second_prize: Pubkey,
    pub third_prize: Pubkey,
    
    pub status: u8,
}

#[account]
pub struct UserRoundState {
    pub user: Pubkey,
    pub round_id: u64,
    pub deposit_amount: u64,
    pub ticket_count: u64, 
    pub is_claimed: bool,
    pub amount_to_claim: u64,
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum RoundStatus {
    Started,
    Ended,
    ChoosingWinners,
    RewardsDistributed,
}
impl RoundStatus {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(RoundStatus::Started),
            1 => Some(RoundStatus::Ended),
            2 => Some(RoundStatus::ChoosingWinners),
            3 => Some(RoundStatus::RewardsDistributed),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            RoundStatus::Started => 0,
            RoundStatus::Ended => 1,
            RoundStatus::ChoosingWinners => 2,
            RoundStatus::RewardsDistributed => 3,
        }
    }
}
impl<'info>  RoundState {
    pub fn update_status(&mut self, now_ts: u64) {
        if(self.status == RoundStatus::RewardsDistributed.to_u8()) {
            return;
        }
        if now_ts >= (self.end_ts + self.gap_time) as u64 {
            self.status = RoundStatus::ChoosingWinners.to_u8();
            return;
        }
        else if now_ts >= self.end_ts as u64 {
            self.status = RoundStatus::Ended.to_u8();
            return;
        }
    }
}
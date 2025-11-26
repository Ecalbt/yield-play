use anchor_lang::prelude::*;

#[constant]
pub const LOTTERY_STATE_SEED: &[u8] = b"LOTTERY_STATE_SEED_3";
pub const ROUND_STATE_SEED: &[u8] = b"ROUND_STATE_SEED_2";
pub const ROUND_VAULT_SEED: &[u8] = b"ROUND_VAULT_SEED_2";
pub const ROUND_VAULT_SIGNER_SEED: &[u8] = b"ROUND_VAULT_SIGNER_SEED_2";
pub const USER_STATE_SEED: &[u8] = b"USER_STATE_SEED_2";
pub const NATIVE_MINT: Pubkey = Pubkey::new_from_array([0u8; 32]);
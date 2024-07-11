use {
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{program_error::ProgramError, pubkey::Pubkey},
};

pub const STATE_SIZE: usize = 129;
pub const USER_DATA_SIZE: usize = 42;

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct WhitelistState {
	pub bump: u8,
	pub authority: Pubkey,
	pub token_vault: Pubkey,
	pub token_mint: Pubkey,
	pub token_price: u64,
	pub whitelist_size: u64,
	pub purchase_limit: u64,
	pub sale_start_time: i64,
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct UserData {
    pub bump: u8,
    pub whitelisted: bool,
    pub owner: Pubkey,
	pub amount_purchased: u64,
}

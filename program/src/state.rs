use {
	crate::error::WhitelistError,
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{
		entrypoint::ProgramResult,
		pubkey::Pubkey,
		sysvar::{clock::Clock, Sysvar},
	},
};

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct Whitelist {
	pub bump: u8,
	pub authority: Pubkey,
	pub vault: Pubkey,
	pub treasury: Pubkey,
	pub mint: Pubkey,
	pub token_price: u64,
	pub buy_limit: u64,
	pub deposited: u64,
	pub whitelist_size: Option<u64>,
	pub allow_registration: bool,
	pub registration_start_timestamp: Option<i64>,
	pub registration_duration: Option<i64>,
	pub sale_start_timestamp: Option<i64>,
	pub sale_duration: Option<i64>,
}

impl Whitelist {
	pub const LEN: usize = 194;

	pub fn check_times(&self) -> ProgramResult {
		let clock = Clock::get()?;
		// Perform safety checks if a `registration_start_timestamp` is not `None`
		if let Some(registration_start_timestamp) = self.registration_start_timestamp {
			if registration_start_timestamp < clock.unix_timestamp {
				return Err(WhitelistError::InvalidRegistrationStartTime.into());
			}
		}

		// Perform safety checks if a `sale_start_timestamp` is not `None`
		if let Some(sale_start_timestamp) = self.sale_start_timestamp {
			if sale_start_timestamp < clock.unix_timestamp {
				return Err(WhitelistError::InvalidSaleStartTime.into());
			}
			if self
				.registration_start_timestamp
				.is_some_and(|t| t > sale_start_timestamp)
			{
				return Err(WhitelistError::SaleBeforeRegistration.into());
			}
		}

		Ok(())
	}

	pub fn check_sale_time(&self) -> ProgramResult {
		let clock = Clock::get()?;
		if self
			.sale_start_timestamp
			.is_some_and(|t| t >= clock.unix_timestamp)
		{
			Ok(())
		} else {
			Err(WhitelistError::SaleOngoing.into())
		}
	}
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct Ticket {
	pub bump: u8,
    pub whitelist: Pubkey,
	pub owner: Pubkey,
	pub payer: Pubkey,
	pub allowance: u64,
	pub amount_bought: u64,
}

impl Ticket {
	pub const LEN: usize = 124;
}

use {
	crate::error::WhitelistError,
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{
		entrypoint::ProgramResult,
		pubkey::Pubkey,
		sysvar::{clock::Clock, Sysvar},
	},
};

pub const STATE_SIZE: usize = 162;
pub const USER_DATA_SIZE: usize = 74;

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct WhitelistState {
	pub bump: u8,
	pub authority: Pubkey,
	pub vault: Pubkey,
	pub mint: Pubkey,
	pub token_price: u64,
	pub buy_limit: u64,
	pub whitelist_size: Option<u64>,
	pub whitelisted_users: u64,
	pub allow_registration: bool,
	pub registration_start_timestamp: Option<i64>,
	pub registration_end_timestamp: Option<i64>,
	pub sale_start_timestamp: Option<i64>,
	pub sale_end_timestamp: Option<i64>,
}

impl WhitelistState {
	pub fn check_times(&self) -> ProgramResult {
		let clock = Clock::get()?;
		// Perform safety checks if a `registration_start_timestamp` is not `None`
		if let Some(registration_start_timestamp) = self.registration_start_timestamp {
			if registration_start_timestamp < clock.unix_timestamp {
				return Err(WhitelistError::InvalidRegistrationStartTime.into());
			}
			if self
				.registration_end_timestamp
				.is_some_and(|t| t < registration_start_timestamp)
			{
				return Err(WhitelistError::RegistrationStartAfterRegistrationEnd.into());
			}
			if self
				.sale_end_timestamp
				.is_some_and(|t| t < registration_start_timestamp)
			{
				return Err(WhitelistError::RegistrationStartAfterSaleEnd.into());
			}
		}

		// Perform safety checks if a `sale_start_timestamp` is not `None`
		if let Some(sale_start_timestamp) = self.sale_start_timestamp {
			if sale_start_timestamp < clock.unix_timestamp {
				return Err(WhitelistError::InvalidSaleStartTime.into());
			}
			if self
				.sale_end_timestamp
				.is_some_and(|t| t < sale_start_timestamp)
			{
				return Err(WhitelistError::SaleStartAfterSaleEnd.into());
			}
			if self
				.registration_start_timestamp
				.is_some_and(|t| t > sale_start_timestamp)
			{
				return Err(WhitelistError::SaleBeforeRegistration.into());
			}
		}

		// Ensure the end timestamps for registration and sale are greater than the current time
		if self
			.registration_end_timestamp
			.is_some_and(|t| t <= clock.unix_timestamp)
			|| self
				.sale_end_timestamp
				.is_some_and(|t| t <= clock.unix_timestamp)
		{
			return Err(WhitelistError::InvalidTimestamp.into());
		}

		Ok(())
	}

	pub fn check_sale_time(&self) -> ProgramResult {
		let clock = Clock::get()?;
		if self
			.sale_start_timestamp
			.is_some_and(|t| t >= clock.unix_timestamp)
			|| self
				.sale_end_timestamp
				.is_some_and(|t| t <= clock.unix_timestamp)
		{
			Ok(())
		} else {
			Err(WhitelistError::SaleOngoing.into())
		}
	}
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Debug, PartialEq)]
pub struct UserData {
	pub bump: u8,
	pub owner: Pubkey,
	pub payer: Pubkey,
	pub amount_bought: u64,
}

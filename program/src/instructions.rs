use {
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{
		instruction::{AccountMeta, Instruction},
		program_error::ProgramError,
		pubkey::Pubkey,
		system_program,
	},
	spl_token_2022,
};

#[derive(BorshDeserialize, BorshSerialize, BorshSchema, Debug, PartialEq)]
pub enum WhitelistInstruction {
	/// Initialises an instance of a whitelist
	///
	/// `whitelist_size`: defines the number of users that can be registered for the
	/// token sale, if no value is passed then the number of users is unrestricited.
	/// i.e. Any amount may be added
	///
	/// `allow_registration`: allows users to register for the whitelist themselves using the
	/// `Register` instruction. If set to false only the authority may add users
	///  using the `AddUser` instruction
	///
	///  `registration_start_timestamp` an optional unixtimestamp of when registration for the
	///  whitelist commences this value must be less than the unixtimestamp of the
	///  `sale_start_timestamp`, if set to `None` then users will be able to register for the
	///  whitelist immediately after initialisation of the whitelist
	///
	///  `registration_end_timestamp`: an optional unixtimestamp of when registration for the
	///  whitelist is prohibited. This value must be larger than the timestamp provided for
	///  `registration_start_timestamp` or else initialisation will throw an error. If set to
	///  `None` there will be no limit for when a user can register for the whitelist
	///
	///  `buy_limit`: the amount of the token that can be bought by an individual user, there are
	///  no checks against the whitelist size or amounts deposited into the vault vs this value
	///
	///  `sale_start_timestamp`: an optional unixtimestamp of when the token sale commences, if a
	///  `registration_start_timestamp` is set, then this value must be equal to or greater than
	///  that of the `registration_start_timestamp` or initialisation will fail. If set to `None`,
	///  then this value will be set to the value of the `registration_start_timestamp` which, if
	///  also set to `None` will commence the token sale immediately upon initialisation
	///
	///  `sale_end_timestamp`: an optional unixtimestamp of when the token sale ends, this value
	///  must be greater than the `sale_start_timestamp` or initialisation will fail. This permits
	///  the withdrawal of any remaining tokens in the vault after the sale time has elapsed.
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[]` Token program
	/// 5. `[]` System program
	InitialiseWhitelist {
		token_price: u64,
		whitelist_size: Option<u64>,
		buy_limit: u64,
		allow_registration: bool,
		registration_start_timestamp: Option<i64>,
		registration_end_timestamp: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_end_timestamp: Option<i64>,
	},

	/// Adds a user to the whitelist
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Token mint
	/// 3. `[]` User account
	/// 4. `[writable]` User whitelist account
	/// 5. `[]` System program
	AddUser,

	/// Reclaims rent from an initialised `UserData` account
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable]` Authority
	/// 2. `[]` Token mint
	/// 3. `[]` User account
	/// 4. `[writable]` User whitelist account
	/// 5. `[writable, signer]` Payer account
	RemoveUser,

	/// Permits the authority to change the whitelist size
	/// Attempting to reduce the whitelist size after registration has commenced will
	/// result in an error if the current number of whitelisted users is greater than
	/// the value provided, setting this value to `None` will enable an unlimited number of
	/// registrants
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	AmendWhitelistSize { size: Option<u64> },

	/// Permits the authority to amend to start or end time of registration or the token sale
	/// Note that this can only be called before the respective current start times
	/// attempting to amend start times after they have already elapsed will result in an error
	/// There is a slight eccentricity here as the `None` values have meaning during
	/// initialisation and in the program itself, but are different in this instruction. In this
	/// instruction a `None` value simply means that the field will not be updated. If you wish
	/// to set a value to `None` instead pass a `0` value.
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	AmendTimes {
		registration_start_timestamp: Option<i64>,
		registration_end_timestamp: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_end_timestamp: Option<i64>,
	},

	/// Allow users to register for the whitelist
	/// This instruction is for editing the `allow_registration` state after initialisation
	/// i.e. should we want to stop users from registering for whatever reason or vice versa
	///
	/// Accounts expected:
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Mint accuont
	AllowRegister { allow_registration: bool },

	/// Permits users to register for the whitelist
	/// This instruction replicated the functionality for `AddUser`
	/// it's intended usage is for users themselves to register,
	/// it can be turned off by calling `AllowRegister` and setting it to false
	/// or setting `allow_registration` to false on initialisation of the whitelist
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[]` Mint account
	/// 2. `[writable, signer]` User account
	/// 3. `[writable]` User whitelist account
	/// 4. `[]` System program
	Register,

	/// Allows a user to deregister from the whitelist and reclaim lamports used for rent
	/// Note that this will only reclaim lamports for the user if they are the payer for
	/// the account, else this will return the lamports to the authority
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable]` Authority
	/// 2. `[]` Mint account
	/// 3. `[writable, signer]` User account
	/// 4. `[writable]` User whitelist account
	/// 5. `[]` System program
	Unregister,

	/// Buy tokens
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable]` Token vault
	/// 2. `[]` Token mint
	/// 3. `[writable, signer]` User account
	/// 4. `[writable]` User whitelist account
	/// 5. `[writable]` User token account
	/// 6. `[]` Token program
	/// 7. `[]` System program
	/// 8. `[]` Associated token account program
	Buy { amount: u64 },

	/// Deposits tokens into the vault
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable]` Token vault
	/// 2. `[writable, signer]` Depositor account
	/// 3. `[writable]` Depositor token account
	/// 4. `[]` Token mint
	/// 5. `[]` Token program
	/// 6. `[]` System program
	/// 7. `[]` Associated token account program
	DepositTokens { amount: u64 },

	/// Withdraw SOL from the vault
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Recipient account
	/// 3. `[]` System program
	WithdrawSol { amount: u64 },

	/// Withdraw tokens from the vault
	/// Tokens can only be withdrawn before the start of the token sale, or after the token sale
	/// has finished. Attempting to withdraw tokens at any other time will throw an error.
	/// A workaround, if you have not set an `sale_end_timestamp`, to withdraw remaining tokens,
	/// should there be no more users who wish to buy the tokens, is to purchase them yourself
	/// and use the `WithdrawSol` instruction to withdraw the SOL used to purchase the token.
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[writable]` Recipient token account
	/// 5. `[]` Token program
	/// 6. `[]` System program
	/// 7. `[]` Associated token account program
	WithdrawTokens { amount: u64 },

	/// Close the whitelist account
	/// This instruction zeroes the whitelist account and returns
	/// rent back to the Authority.
    /// Terminating a whitelist can only occur when one of two events occur:
    /// 1. The vault has been drained of tokens
    /// 2. The token sale has ended
    /// In the second event, this instruction will transfer any remaining tokens to a
    /// recipient token account
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[writable]` Recipient account
	/// 5. `[writable]` Recipient token account
	/// 6. `[]` Token program
	/// 7. `[]` System program
	/// 8. `[]` Associated token account program
	TerminateWhitelist,
}

/// Creates an 'InitialiseWhitelist' instruction
pub fn init_whitelist(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	token_price: u64,
	buy_limit: u64,
	whitelist_size: Option<u64>,
	allow_registration: bool,
	registration_start_timestamp: Option<i64>,
	registration_end_timestamp: Option<i64>,
	sale_start_timestamp: Option<i64>,
	sale_end_timestamp: Option<i64>,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::InitialiseWhitelist {
			token_price,
			whitelist_size,
			buy_limit,
			allow_registration,
			registration_start_timestamp,
			registration_end_timestamp,
			sale_start_timestamp,
			sale_end_timestamp,
		},
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new(*vault, false),
			AccountMeta::new(*mint, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn add_user(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AddUser,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new_readonly(*user, false),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn remove_user(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::RemoveUser,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new_readonly(*user, false),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn buy_tokens(
	whitelist: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
	user_token_account: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Buy { amount },
		vec![
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*vault, false),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new(*user, true),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new(*user_token_account, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn amend_whitelist_size(
	whitelist: &Pubkey,
	authority: &Pubkey,
	size: Option<u64>,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AmendWhitelistSize { size },
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
		],
	))
}

pub fn amend_times(
	whitelist: &Pubkey,
	authority: &Pubkey,
	registration_start_timestamp: Option<i64>,
	registration_end_timestamp: Option<i64>,
	sale_start_timestamp: Option<i64>,
	sale_end_timestamp: Option<i64>,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AmendTimes {
			registration_start_timestamp,
			registration_end_timestamp,
			sale_start_timestamp,
			sale_end_timestamp,
		},
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
		],
	))
}

pub fn allow_registration(
	whitelist: &Pubkey,
	authority: &Pubkey,
	allow_registration: bool,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AllowRegister { allow_registration },
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
		],
	))
}

pub fn register(
	whitelist: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Register,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*user, true),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn unregister(
	whitelist: &Pubkey,
	authority: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Unregister,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new_readonly(*authority, false),
			AccountMeta::new(*user, true),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn deposit_tokens(
	whitelist: &Pubkey,
	vault: &Pubkey,
	depositor_key: &Pubkey,
	depositor_token_account_key: &Pubkey,
	mint: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::DepositTokens { amount },
		vec![
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*vault, false),
			AccountMeta::new(*depositor_key, true),
			AccountMeta::new(*depositor_token_account_key, false),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn withdraw_sol(
	whitelist: &Pubkey,
	authority: &Pubkey,
	recipient: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::WithdrawSol { amount },
		vec![
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new(*recipient, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn withdraw_tokens(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	recipient_token_account: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::WithdrawTokens { amount },
		vec![
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new(*vault, false),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new(*recipient_token_account, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn terminate_whitelist(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	recipient: &Pubkey,
	recipient_token_account: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateWhitelist,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new(*vault, false),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new(*recipient, false),
			AccountMeta::new(*recipient_token_account, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

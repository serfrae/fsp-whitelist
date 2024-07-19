use {
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{
		instruction::{AccountMeta, Instruction},
		program_error::ProgramError,
		pubkey::Pubkey,
		system_program,
	},
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
	///  `registration_start_timestamp` a unixtimestamp of when registration for the
	///  whitelist commences this value must be less than the unixtimestamp of the
	///  `sale_start_timestamp`, if set to `0` then users will be able to register for the
	///  whitelist immediately after initialisation of the whitelist
	///
	///  `registration_duration`: a duration in milliseconds when registration for the
	///  whitelist is allowed.  If set to `0` there will be no limit for when a user can register
	///  for the whitelist
	///
	///  `buy_limit`: the amount of the token that can be bought by an individual user, there are
	///  no checks against the whitelist size or amounts deposited into the vault vs this value
	///
	///  `sale_start_timestamp`: a unixtimestamp of when the token sale commences, if a
	///  `registration_start_timestamp` is set, then this value must be equal to or greater than
	///  that of the `registration_start_timestamp` or initialisation will fail. If set to `0`,
	///  then this value will be set to the value of the `registration_start_timestamp` which, if
	///  also set to `0` will commence the token sale immediately upon initialisation
	///
	///  `sale_duration`: a duration in millsiseconds for the duration of the sale, this
	///  value. This permits the withdrawal of any remaining tokens in the vault after the sale
	///  time has elapsed. Failing to set this value will not allow termination of the whitelist
	///  until all tokens are sold (not recommended).
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[]` Token program
	/// 5. `[]` System program
	/// 6. `[]` Assoc token program
	InitialiseWhitelist {
		treasury: Pubkey,
		token_price: u64,
		whitelist_size: u64,
		buy_limit: u64,
		allow_registration: bool,
		registration_start_timestamp: i64,
		registration_duration: i64,
		sale_start_timestamp: i64,
		sale_duration: i64,
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
	AmendWhitelistSize { size: u64 },

	/// Permits the authority to amend to start or end time of registration or the token sale
	/// Note: This can only be called before the respective current start times.
	/// Attempting to amend start times after they have already elapsed will result in an error
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	AmendTimes {
		registration_start_timestamp: Option<i64>,
		registration_duration: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_duration: Option<i64>,
	},

	/// Allow users to register for the whitelist
	/// This instruction is for editing the `allow_registration` state after initialisation
	/// i.e. should we want to stop users from registering for whatever reason or vice versa
	/// This instruction may also be used to freeze further registrations
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
	/// 2. `[writable]` Token vault
	/// 3. `[]` Mint account
	/// 4. `[writable, signer]` User account
	/// 5. `[writable]` User whitelist account
	/// 6. `[]` Token program
	/// 7. `[]` System program
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
	/// 5. `[writable]` Ticket token account
	/// 6. `[writable]` User token account
	/// 7. `[]` Token program
	/// 8. `[]` System program
	/// 9. `[]` Associated token account program
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

	/// Manually start presale registration
	/// Warning: This instruction executes even if a `registration_start_time` is provided and
	/// will set the corresponding field in program state to `None`
	///
	/// Accounts expected:
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	StartRegistration,

	/// Manually start the token sale
	/// Warning: This instruction executes even if a `sale_start_time` is provided and will set
	/// the corresponding field in the program state to `None`. If used before registration has
	/// commenced, this will also set the corresponding field to `None`.
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Token vault
	StartTokenSale,

	/// Transfers tokens to Ticket PDA
	/// This instruction transfers tokens to the ticket PDA before the token sale commences.
	/// The inteded use of which is to relieve bottlenecks during token sale events as it allows
	/// parallel execution of token transfers from the PDA to the user's token account instead
	/// of multiple writes to the vault requesting transfers.
	///
	/// Accounts expected:
	/// 0. `[]` Whitelist address
	/// 1. `[writable, signer] Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[]` User account
	/// 5. `[]` Ticket account
	/// 6. `[writable]` Ticket token account
	/// 7. `[]` Token program
	/// 8. `[]` System program
	/// 9. `[]` Assoc token program
	TransferTokens,

	/// Withdraw tokens from the vault
	/// Tokens can only be withdrawn before the start of the token sale, or after the token sale
	/// has finished. Attempting to withdraw tokens at any other time will throw an error.
	/// A workaround, if you have not set an `sale_duration`, to withdraw remaining tokens,
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

	/// Burns ticket and transfers tokens and lamports into the treasury
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Mint
	/// 3. `[writable]` Treasury
	/// 4. `[writable]` Treasury token account
	/// 5. `[writable]` Ticket
	/// 6. `[writable]` Ticket token account
	/// 7. `[]` Token program
	/// 8. `[]` System program
	/// 9. `[]` Associated token account program
	BurnTicket,

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
	treasury: &Pubkey,
	token_price: u64,
	buy_limit: u64,
	whitelist_size: u64,
	allow_registration: bool,
	registration_start_timestamp: i64,
	registration_duration: i64,
	sale_start_timestamp: i64,
	sale_duration: i64,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(7);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));
	accounts.push(AccountMeta::new_readonly(
		spl_associated_token_account::id(),
		false,
	));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::InitialiseWhitelist {
			treasury: *treasury,
			token_price,
			whitelist_size,
			buy_limit,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		},
		accounts,
	))
}

pub fn add_user(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_ticket: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(6);

	accounts.push(AccountMeta::new_readonly(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new_readonly(*user, false));
	accounts.push(AccountMeta::new(*user_ticket, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AddUser,
		accounts,
	))
}

pub fn remove_user(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_ticket: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(6);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new_readonly(*user, false));
	accounts.push(AccountMeta::new(*user_ticket, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::RemoveUser,
		accounts,
	))
}

pub fn buy_tokens(
	whitelist: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_ticket: &Pubkey,
	ticket_token_account: &Pubkey,
	user_token_account: &Pubkey,
	amount: u64,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(10);

	accounts.push(AccountMeta::new_readonly(*whitelist, false));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new(*user, true));
	accounts.push(AccountMeta::new(*user_ticket, false));
	accounts.push(AccountMeta::new(*ticket_token_account, false));
	accounts.push(AccountMeta::new(*user_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));
	accounts.push(AccountMeta::new_readonly(
		spl_associated_token_account::id(),
		false,
	));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Buy { amount },
		accounts,
	))
}

pub fn amend_whitelist_size(
	whitelist: &Pubkey,
	authority: &Pubkey,
	size: u64,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(2);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AmendWhitelistSize { size },
		accounts,
	))
}

pub fn amend_times(
	whitelist: &Pubkey,
	authority: &Pubkey,
	registration_start_timestamp: Option<i64>,
	registration_duration: Option<i64>,
	sale_start_timestamp: Option<i64>,
	sale_duration: Option<i64>,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(2);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AmendTimes {
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		},
		accounts,
	))
}

pub fn allow_registration(
	whitelist: &Pubkey,
	authority: &Pubkey,
	allow_registration: bool,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(2);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AllowRegister { allow_registration },
		accounts,
	))
}

pub fn register(
	whitelist: &Pubkey,
	user: &Pubkey,
	user_ticket: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(4);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*user, true));
	accounts.push(AccountMeta::new(*user_ticket, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Register,
		accounts,
	))
}

pub fn unregister(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_ticket: &Pubkey,
	ticket_token_account: &Pubkey,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(9);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new_readonly(*authority, false));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new(*user, true));
	accounts.push(AccountMeta::new(*user_ticket, false));
	accounts.push(AccountMeta::new(*ticket_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Unregister,
		accounts,
	))
}

pub fn deposit_tokens(
	whitelist: &Pubkey,
	vault: &Pubkey,
	depositor_key: &Pubkey,
	depositor_token_account_key: &Pubkey,
	mint: &Pubkey,
	amount: u64,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(6);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new(*depositor_key, true));
	accounts.push(AccountMeta::new(*depositor_token_account_key, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::DepositTokens { amount },
		accounts,
	))
}

pub fn start_registration(
	whitelist: &Pubkey,
	authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(2);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::StartRegistration,
		accounts,
	))
}

pub fn start_token_sale(
	whitelist: &Pubkey,
	authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(2);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::StartTokenSale,
		accounts,
	))
}

pub fn transfer_tokens(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	user_account: &Pubkey,
	ticket_account: &Pubkey,
	ticket_token_account: &Pubkey,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(10);

	accounts.push(AccountMeta::new_readonly(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new_readonly(*user_account, false));
	accounts.push(AccountMeta::new_readonly(*ticket_account, false));
	accounts.push(AccountMeta::new(*ticket_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));
	accounts.push(AccountMeta::new_readonly(
		spl_associated_token_account::id(),
		false,
	));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TransferTokens,
		accounts,
	))
}

pub fn withdraw_tokens(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	recipient_token_account: &Pubkey,
	amount: u64,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(6);

	accounts.push(AccountMeta::new_readonly(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new(*recipient_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::WithdrawTokens { amount },
		accounts,
	))
}

pub fn burn_ticket(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	treasury: &Pubkey,
	treasury_token_account: &Pubkey,
	ticket: &Pubkey,
	ticket_token_account: &Pubkey,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(10);

	accounts.push(AccountMeta::new_readonly(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new(*treasury, false));
	accounts.push(AccountMeta::new(*treasury_token_account, false));
	accounts.push(AccountMeta::new(*ticket, false));
	accounts.push(AccountMeta::new(*ticket_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));
	accounts.push(AccountMeta::new_readonly(
		spl_associated_token_account::id(),
		false,
	));
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::BurnTicket,
		accounts,
	))
}

pub fn terminate_whitelist(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	recipient: &Pubkey,
	recipient_token_account: &Pubkey,
	token_program: &Pubkey,
) -> Result<Instruction, ProgramError> {
	let mut accounts = Vec::with_capacity(8);

	accounts.push(AccountMeta::new(*whitelist, false));
	accounts.push(AccountMeta::new(*authority, true));
	accounts.push(AccountMeta::new(*vault, false));
	accounts.push(AccountMeta::new_readonly(*mint, false));
	accounts.push(AccountMeta::new(*recipient, false));
	accounts.push(AccountMeta::new(*recipient_token_account, false));
	accounts.push(AccountMeta::new_readonly(*token_program, false));
	accounts.push(AccountMeta::new_readonly(system_program::id(), false));

	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateWhitelist,
		accounts,
	))
}

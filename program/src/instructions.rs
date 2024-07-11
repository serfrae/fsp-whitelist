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
		whitelist_size: u64,
		buy_limit: u64,
		sale_start_time: i64,
	},

	/// Adds a user to the whitelist
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Token mint
	/// 3. `[]` User account
	/// 4. `[writable]` User whitelist account
	/// 5. `[]` System program
	AddUser,

	/// Removes a user from the whitelist
	/// This instruction only flips the `whitelist` field
	/// in `UserData` to `false`, to reclaim rent use
	/// `TerminateUser` instead
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Token mint
	/// 3. `[]` User account
	/// 4. `[writable]` User whitelist account
	RemoveUser,

	/// Reclaims rent from an initialised `UserData` account
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[]` Token mint
	/// 3. `[]` User account
	/// 4. `[writable]` User whitelist account
	TerminateUser,

	/// buy tokens
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
	/// 2. `[]` System program
	WithdrawSol { amount: u64 },

	/// Withdraw tokens from the vault
	///
	/// Accounts expected:
	///
	/// 0. `[]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[writable]` Authority token account
	/// 5. `[]` Token program
	/// 6. `[]` System program
	/// 7. `[]` Associated token account program
	WithdrawTokens { amount: u64 },

	/// Close the whitelist account
	/// This instruction zeroes the whitelist account and returns
	/// rent back to the Authority
	///
	/// Accounts expected:
	///
	/// 0. `[writable]` Whitelist account
	/// 1. `[writable, signer]` Authority
	/// 2. `[writable]` Token vault
	/// 3. `[]` Token mint
	/// 4. `[writable]` Authority token account
	/// 5. `[]` Token program
	/// 6. `[]` System program
	/// 7. `[]` Associated token account program
	TerminateWhitelist,
}

/// Creates an 'InitialiseWhitelist' instruction
pub fn init_whitelist(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	token_price: u64,
	whitelist_size: u64,
	buy_limit: u64,
	sale_start_time: i64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::InitialiseWhitelist {
			token_price,
			whitelist_size,
			buy_limit,
			sale_start_time,
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
			AccountMeta::new_readonly(*whitelist, false),
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
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new_readonly(*user, false),
			AccountMeta::new(*user_whitelist, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn terminate_user(
	whitelist: &Pubkey,
	authority: &Pubkey,
	mint: &Pubkey,
	user: &Pubkey,
	user_whitelist: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateUser,
		vec![
			AccountMeta::new_readonly(*whitelist, false),
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
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::WithdrawSol { amount },
		vec![
			AccountMeta::new_readonly(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn withdraw_tokens(
	whitelist: &Pubkey,
	authority: &Pubkey,
	vault: &Pubkey,
	mint: &Pubkey,
	authority_token_account: &Pubkey,
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
			AccountMeta::new(*authority_token_account, false),
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
	authority_token_account: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateWhitelist,
		vec![
			AccountMeta::new(*whitelist, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new(*vault, false),
			AccountMeta::new_readonly(*mint, false),
			AccountMeta::new(*authority_token_account, false),
			AccountMeta::new_readonly(spl_token_2022::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

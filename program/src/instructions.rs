use {
	borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
	solana_program::{
		instruction::{AccountMeta, Instruction},
		program_error::ProgramError,
		pubkey::Pubkey,
		system_program,
	},
	spl_token,
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
		purchase_limit: u64,
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

	/// Purchase tokens
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
	Purchase { amount: u64 },

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
	whitelist_key: &Pubkey,
	whitelist_authority_key: &Pubkey,
	token_vault_key: &Pubkey,
	token_mint_key: &Pubkey,
	token_price: u64,
	whitelist_size: u64,
	purchase_limit: u64,
	sale_start_time: i64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::InitialiseWhitelist {
			token_price,
			whitelist_size,
			purchase_limit,
			sale_start_time,
		},
		vec![
			AccountMeta::new(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new(*token_vault_key, false),
			AccountMeta::new(*token_mint_key, false),
			AccountMeta::new_readonly(spl_token::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn add_user(
	whitelist_key: &Pubkey,
	authority_key: &Pubkey,
	token_mint_key: &Pubkey,
	user_key: &Pubkey,
	user_whitelist_key: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::AddUser,
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new_readonly(*user_key, false),
			AccountMeta::new(*user_whitelist_key, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn remove_user(
	whitelist_key: &Pubkey,
	authority_key: &Pubkey,
	token_mint_key: &Pubkey,
	user_key: &Pubkey,
	user_whitelist_key: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::RemoveUser,
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new_readonly(*user_key, false),
			AccountMeta::new(*user_whitelist_key, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn terminate_user(
	whitelist_key: &Pubkey,
	authority_key: &Pubkey,
	token_mint_key: &Pubkey,
	user_key: &Pubkey,
	user_whitelist_key: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateUser,
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new_readonly(*user_key, false),
			AccountMeta::new(*user_whitelist_key, false),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn purchase(
	whitelist_key: &Pubkey,
	token_vault_key: &Pubkey,
	token_mint_key: &Pubkey,
	user_key: &Pubkey,
	user_whitelist_key: &Pubkey,
	user_token_account_key: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Purchase { amount },
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*token_vault_key, false),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new(*user_key, true),
			AccountMeta::new(*user_whitelist_key, false),
			AccountMeta::new(*user_token_account_key, false),
			AccountMeta::new_readonly(spl_token::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn deposit(
	whitelist_key: &Pubkey,
	token_vault_key: &Pubkey,
	depositor_key: &Pubkey,
	depositor_token_account_key: &Pubkey,
	token_mint_key: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	OK(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Deposit { amount },
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*token_vault_key, false),
			AccountMeta::new(*depositor_key, true),
			AccountMeta::new(*depositor_token_account_key, false),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new_readonly(spl_token::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn withdraw_sol(
	whitelist_key: &Pubkey,
	authority: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::WithdrawSol { amount },
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*authority, true),
			AccountMeta::new_readonly(system_program::id(), false),
		],
	))
}

pub fn withdraw_tokens(
	whitelist_key: &Pubkey,
	authority: &Pubkey,
	token_vault_key: &Pubkey,
	token_mint_key: &Pubkey,
	authority_token_account_key: &Pubkey,
	amount: u64,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::Withdraw { amount },
		vec![
			AccountMeta::new_readonly(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new(*token_vault_key, false),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new(*authority_token_account_key, false),
			AccountMeta::new_readonly(spl_token::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_reaodnly(spl_associated_token_account::id(), false),
		],
	))
}

pub fn terminate_whitelist(
	whitelist_key: &Pubkey,
	authority_key: &Pubkey,
	token_vault_key: &Pubkey,
	token_mint_key: &Pubkey,
	authority_token_account_key: &Pubkey,
) -> Result<Instruction, ProgramError> {
	Ok(Instruction::new_with_borsh(
		crate::id(),
		&WhitelistInstruction::TerminateWhitelist,
		vec![
			AccountMeta::new(*whitelist_key, false),
			AccountMeta::new(*authority_key, true),
			AccountMeta::new(*token_vault_key, false),
			AccountMeta::new_readonly(*token_mint_key, false),
			AccountMeta::new(*authority_token_account_key, false),
			AccountMeta::new_readonly(spl_token::id(), false),
			AccountMeta::new_readonly(system_program::id(), false),
			AccountMeta::new_readonly(spl_associated_token_program::id(), false),
		],
	))
}

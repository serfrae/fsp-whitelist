use {
	crate::{
		error::WhitelistError,
		instructions::WhitelistInstruction,
		state::{UserData, WhitelistState, STATE_SIZE, USER_DATA_SIZE},
	},
	borsh::{BorshDeserialize, BorshSerialize},
	solana_program::{
		account_info::{next_account_info, AccountInfo},
		clock,
		entrypoint::ProgramResult,
		msg,
		program::{invoke, invoke_signed},
		program_error::ProgramError,
		pubkey::Pubkey,
		system_instruction, system_program,
		sysvar::{clock::Clock, rent::Rent, Sysvar},
	},
	spl_token,
};

const MINT_SIZE: usize = 82;

pub struct WhitelistProcessor;

impl WhitelistProcessor {
	pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
		if program_id != &crate::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let instruction: WhitelistInstruction = WhitelistInstruction::try_from_slice(data)
			.map_err(|_| ProgramError::InvalidInstructionData)?;

		match instruction {
			WhitelistInstruction::Initialise {
				token_price,
				whitelist_size,
				purchase_limit,
				sale_start_time,
			} => Self::process_init(
				accounts,
				token_price,
				whitelist_size,
				purchase_limit,
				sale_start_time,
			),
			WhitelistInstruction::AddUser => Self::process_add_user(accounts),
			WhitelistInstruction::RemoveUser => Self::process_remove_user(accounts),
			WhitelistInstruction::TerminateUser => Self::process_terminate_user(accounts),
			WhitelistInstruction::Purchase { amount } => Self::process_purchase(accounts, amount),
			WhitelistInstruction::DepositTokens { amount } => Self::process_deposit_tokens(accounts, amount),
			WhitelistInstruction::WithdrawTokens { amount } => Self::process_withdraw_tokens(accounts, amount),
			WhitelistInstruction::WithdrawSol { amount } => Self::processs_withdraw_sol(accounts, amount),
			WhitelistInstruction::TerminateWhitelist => Self::process_terminate(accounts),
		}
		Ok(())
	}

	fn process_init(
		accounts: &[AccountInfo],
		token_price: u64,
		whitelist_size: u64,
		purchase_limit: u64,
		sale_start_time: i64,
	) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		let rent = Rent::get()?;
		let clock = Clock::get()?;

		let (wl, bump) = crate::get_whitelist_address(authority.key, mint.key);

		// Safety dance
		if whitelist_account.key != wl {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !authority.is_signer {
			return Err(WhitelistError::NoSigners.into());
		}

		if vault.key
			!= spl_associated_token_account::get_associated_token_address(
				&whitelist_account.key,
				&mint.key,
			) {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.owner != spl_token::id() {
			return Err(WhitelistError::IllegalMintOwner.into());
		}

		if token_program.key != spl_token::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if system_program.key != system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if assc_token_program.key != spl_associated_token_account::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if clock.unix_timestamp > sale_start_time {
			return Err(WhitelistError::InvalidSaleStartTime.into());
		}

		if whitelist_account.owner != &crate::id() {
			msg!("Initialising whitelist account");
			invoke_signed(
				&system_instruction::create_account(
					authority.key,
					&wl,
					rent.minimum_balance(STATE_SIZE)
						.max(1)
						.saturating_sub(whitelist_account.lamports()),
					STATE_SIZE as u64,
					&crate::id(),
				),
				&[
					authority.clone(),
					whitelist_account.clone(),
					system_program.clone(),
				],
				&[&[SEED, authority.as_ref(), &[bump]]],
			)?;

			msg!("Initialising vault");
			invoke_signed(
				&spl_associated_token_account::instruction::create_associated_token_account(
					authority.key,
					wl,
					mint.key,
					token_program.key,
				),
				&[
					authority.clone(),
					vault.clone(),
					whitelist_account.clone(),
					mint.clone(),
					system_program.clone(),
					token_program.clone(),
					assc_token_program.clone(),
				],
				&[&[SEED, authority.as_ref(), &[bump]]],
			)?;
		}

		let whitelist_state = WhitelistState {
			bump,
			authority: *authority.key,
			token_vault: *vault.key,
			token_mint: *mint.key,
			token_price,
			whitelist_size,
			purchase_limit,
			sale_start_time,
		};

		whitelist_state.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;
		msg!("Whitelist initialised");

		Ok(())
	}

	fn process_add_user(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_whitelist_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;

		let (wl, bump) = crate::get_whitelist_address(authority.key, mint.key);
		let (user_wl, user_bump) = crate::get_user_whitelist_address(user_account.key, wl);

		if whitelist_account.key != wl {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if !authority.is_signer || authority.key != wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if mint.key != wl_data.token_mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if user_whitelist_account.key != user_wl {
			return Err(WhitelistError::InvalidUserAccount.into());
		}

		if system_program.key != system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if user_whitelist_account.owner != crate::id() {
			msg!("Creating user whitelist account");
			invoke_signed(
				&system_instruction::create_account(
					authority.key,
					&user_wl,
					rent.minimum_balance(USER_DATA_SIZE)
						.max(1)
						.saturating_sub(user_whitelist_account.lamports()),
					USER_DATA_SIZE as u64,
					&crate::id(),
				),
				&[
					authority.clone(),
					user_whitelist_account.clone(),
					system_program.clone(),
				],
				&[&[
					SEED,
					user_account.key.as_ref(),
					whitelist_account.key.as_ref(),
					&[user_bump],
				]],
			)?;
		}

		let user_data = UserData {
			bump: user_bump,
			whitelisted: true,
			owner: user_account.key,
			amount_purchased: 0,
		};

		user_data.serialize(&mut &mut user_whitelist_account.data.borrow_mut()[..])?;

		msg!("User initialised");

		Ok(())
	}

	fn process_remove_user(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_whitelist_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let (wl, wl_bump) = get_whitelist_address(&authority.key, &mint.key);
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, wl);

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if whitelist_account.key != wl || wl_bump = wl_data.bump {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if user_whitelist_account.key != user_wl || user_bump != user_data.bump {
			return Err(WhitelistError::InvalidUserAccount.into());
		}

		if system_program.key != system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		user_data.whitelisted = false;

		user_data.serialize(&mut &mut user_account.data.borrow_mut()[..])?;

		msg!("User removed from whitelist");

		Ok(())
	}

	fn process_terminate_user(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_whitelist_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let (wl, wl_bump) = get_whitelist_address(&authority.key, &mint.key);
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, wl);

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if whitelist_account.key != wl || wl_bump = wl_data.bump {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if user_whitelist_account.key != user_wl || user_bump != user_data.bump {
			return Err(WhitelistError::InvalidUserAccount.into());
		}

		if system_program.key != system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		msg!("User terminated, reclaimed sol: lamports");
		Ok(())
	}

	fn process_purchase(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_whitelist_account = next_account_info(accounts_iter)?;
		let user_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;
		let mint_data = spl_token::state::Mint::unpack_from_slice(mint.data.borrow()[..])?;
		let vault_data = spl_token::state::Account::unpack_from_slice(vault.data.borrow()[..])?;

		let token_amount = spl_token::ui_amount_to_amount(amount, mint_data.decimals);

		let (wl, wl_bump) = get_whitelist_address(&mint.key, &wl_data.authority);
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, wl);

		if !user_account.is_signer {
			return Err(WhitelistError::Unauthorised.into());
		}

		if vault_data.amount < token_amount {
			return Err(WhitelistError::InsufficientFunds.into());
		}

		if !user_data.whitelisted {
			return Err(WhitelistError::Unauthorised.into());
		}

		let sol_amount = match token_amount.checked_mul(wl_data.token_price) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};

		invoke(
			&system_instruction::transfer(user_account.key, whitelist_account.key, sol_amount),
			&[user_account.clone(), whitelist_account.clone()],
		)?;

		invoke_signed(
			&spl_token::instruction::transfer(
				token_program.key,
				vault.key,
				user_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
			),
			&[
				vault.clone(),
				user_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), wl_data.authority.as_ref()]],
		)?;

		user_data.amount_purchased = match user_data.amount_purchased.checked_add(token_amount) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow),
		};
		user_data.serialize(&mut &mut user_account.data.borrow_mut()[..])?;
		msg!("Purchased: {}", amount);
		Ok(())
	}

	fn process_deposit_tokens(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let depositor_account = next_account_info(accounts_iter)?;
		let depositor_token_account = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		invoke(
			&spl_token::instruction::transfer(
				token_program.key,
				depositor_token_account.key,
				vault.key,
				depositor_account.key,
				&[],
				token_amount,
			),
			&[
				depositor_token_account.clone(),
				vault.clone(),
				depositor_account.clone(),
			],
		)?;

		msg!("Deposited: {}", token_amount);
		Ok(())
	}

	fn process_withdraw_tokens(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let authority_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		invoke_signed(
			&spl_token::instruction::transfer(
				token_program.key,
				vault.key,
				authority_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
			),
			&[
				vault.clone(),
				authority_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;

		msg!("Withdrawn: {}", token_amount);
		Ok(())
	}

	fn process_withdraw_sol(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		invoke_signed(
			&system_instruction::transfer(whitelist_account.key, authority.key, amount),
			&[
				whitelist_account.clone(),
				authority.clone(),
				system_program.clone(),
			],
			&[&[SEED, wl_data.mint.as_ref(), authority.key.as_ref()]],
		)?;
		Ok(())
	}

	fn process_terminate_whitelist(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let authority_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		msg!("Terminated whitelist reclaimed sol: lamports");
		Ok(())
	}
}

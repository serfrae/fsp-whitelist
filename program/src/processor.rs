use {
	crate::{
		error::WhitelistError,
		get_user_whitelist_address, get_whitelist_address,
		instructions::WhitelistInstruction,
		state::{UserData, WhitelistState, STATE_SIZE, USER_DATA_SIZE},
		SEED,
	},
	borsh::{BorshDeserialize, BorshSerialize},
	solana_program::{
		account_info::{next_account_info, AccountInfo},
		entrypoint::ProgramResult,
		msg,
		program::{invoke, invoke_signed},
		program_error::ProgramError,
		pubkey::Pubkey,
		system_instruction, system_program,
		sysvar::{clock::Clock, rent::Rent, Sysvar},
	},
	spl_token_2022::{state::{Account, Mint}, extension::StateWithExtensions},
};

pub struct WhitelistProcessor;

impl WhitelistProcessor {
	pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
		if program_id != &crate::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let instruction: WhitelistInstruction = WhitelistInstruction::try_from_slice(data)
			.map_err(|_| ProgramError::InvalidInstructionData)?;

		let _ = match instruction {
			WhitelistInstruction::InitialiseWhitelist {
				token_price,
				whitelist_size,
				buy_limit,
				sale_start_time,
			} => Self::process_init(
				accounts,
				token_price,
				whitelist_size,
				buy_limit,
				sale_start_time,
			),
			WhitelistInstruction::AddUser => Self::process_add_user(accounts),
			WhitelistInstruction::RemoveUser => Self::process_remove_user(accounts),
			WhitelistInstruction::TerminateUser => Self::process_terminate_user(accounts),
			WhitelistInstruction::Buy { amount } => Self::process_buy(accounts, amount),
			WhitelistInstruction::DepositTokens { amount } => {
				Self::process_deposit_tokens(accounts, amount)
			}
			WhitelistInstruction::WithdrawTokens { amount } => {
				Self::process_withdraw_tokens(accounts, amount)
			}
			WhitelistInstruction::WithdrawSol { amount } => {
				Self::process_withdraw_sol(accounts, amount)
			}
			WhitelistInstruction::TerminateWhitelist => Self::process_terminate_whitelist(accounts),
		};

		Ok(())
	}

	fn process_init(
		accounts: &[AccountInfo],
		token_price: u64,
		whitelist_size: u64,
		buy_limit: u64,
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
		if whitelist_account.key != &wl {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !authority.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault.key
			!= &spl_associated_token_account::get_associated_token_address(
				&whitelist_account.key,
				&mint.key,
			) {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.owner != &spl_token_2022::id() {
			return Err(WhitelistError::IllegalMintOwner.into());
		}

		if token_program.key != &spl_token_2022::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if assc_token_program.key != &spl_associated_token_account::id() {
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
				&[&[SEED, authority.key.as_ref(), &[bump]]],
			)?;

			msg!("Initialising vault");
			invoke_signed(
				&spl_associated_token_account::instruction::create_associated_token_account(
					authority.key,
					&wl,
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
				&[&[SEED, authority.key.as_ref(), &[bump]]],
			)?;
		}

		let whitelist_state = WhitelistState {
			bump,
			authority: *authority.key,
			vault: *vault.key,
			mint: *mint.key,
			token_price,
			whitelist_size,
			buy_limit,
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

		let rent = Rent::get()?;

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;

		let (wl, bump) = crate::get_whitelist_address(authority.key, mint.key);
		let (user_wl, user_bump) = crate::get_user_whitelist_address(user_account.key, &wl);

		if whitelist_account.key != &wl {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::SignerError.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if user_whitelist_account.key != &user_wl {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if user_whitelist_account.owner != &crate::id() {
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
			owner: *user_account.key,
			amount_bought: 0,
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
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, &wl);

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let mut user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::SignerError.into());
		}

		if whitelist_account.key != &wl || wl_bump != wl_data.bump {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if user_whitelist_account.key != &user_wl || user_bump != user_data.bump {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
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
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, &wl);

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::SignerError.into());
		}

		if whitelist_account.key != &wl || wl_bump != wl_data.bump {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if user_whitelist_account.key != &user_wl || user_bump != user_data.bump {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let user_whitelist_lamports = user_whitelist_account.lamports();

		invoke_signed(
			&system_instruction::transfer(
				user_whitelist_account.key,
				authority.key,
				user_whitelist_lamports,
			),
			&[
				user_whitelist_account.clone(),
				authority.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;
		user_whitelist_account.assign(&system_program::id());
		user_whitelist_account.realloc(0, false)?;

		msg!(
			"User terminated, reclaimed sol: {} lamports",
			user_whitelist_lamports
		);
		Ok(())
	}

	fn process_buy(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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
		let mut user_data = UserData::try_from_slice(&user_whitelist_account.data.borrow()[..])?;
        let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
        let borrowed_vault_data = vault.data.borrow();
		let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;

		let token_amount = spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		let (wl, wl_bump) = get_whitelist_address(&mint.key, &wl_data.authority);
		let (user_wl, user_bump) = get_user_whitelist_address(&user_account.key, &wl);

		if !user_account.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault_data.base.amount < token_amount {
			return Err(WhitelistError::InsufficientFunds.into());
		}

		if !user_data.whitelisted {
			return Err(WhitelistError::Unauthorised.into());
		}

		let sol_amount = match token_amount.checked_mul(wl_data.token_price) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};

        // We'll check for a `user_token_account` and create one if it doesn't exist
		invoke(
			&system_instruction::transfer(user_account.key, whitelist_account.key, sol_amount),
			&[user_account.clone(), whitelist_account.clone()],
		)?;

		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				user_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
				mint_data.base.decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
				user_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), wl_data.authority.as_ref()]],
		)?;

		user_data.amount_bought = match user_data.amount_bought.checked_add(token_amount) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};
		user_data.serialize(&mut &mut user_account.data.borrow_mut()[..])?;
		msg!("Bought: {}", amount);
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

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;
        let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;

		let token_amount = spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		let (wl, wl_bump) = get_whitelist_address(mint.key, &wl_data.authority);

		if whitelist_account.key != &wl || wl_bump != wl_data.bump {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !depositor_account.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault.key != &wl_data.vault
			|| vault.key
				!= &spl_associated_token_account::get_associated_token_address(
					whitelist_account.key,
					mint.key,
				) {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if token_program.key != &spl_token_2022::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		invoke(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				depositor_token_account.key,
				mint.key,
				vault.key,
				depositor_account.key,
				&[],
				token_amount,
				mint_data.base.decimals,
			)?,
			&[
				depositor_token_account.clone(),
				mint.clone(),
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

        let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let token_amount = spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				authority_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
				mint_data.base.decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
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

		let wl_data = WhitelistState::try_from_slice(&whitelist_account.data.borrow()[..])?;

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

		let whitelist_lamports = whitelist_account.lamports();
		let vault_lamports = vault.lamports();
        let borrowed_vault_data = vault.data.borrow();
		let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;
        let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;

		// Transfer remaining tokens out of the vault
		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				authority_token_account.key,
				whitelist_account.key,
				&[],
				vault_data.base.amount,
				mint_data.base.decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
				authority_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;

		// Close vault and reclaim lamports
		invoke_signed(
			&spl_token_2022::instruction::close_account(
				token_program.key,
				vault.key,
				authority.key,
				whitelist_account.key,
				&[],
			)?,
			&[vault.clone(), authority.clone(), whitelist_account.clone()],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;

		// Close whitelist and reclaim lamports
		invoke_signed(
			&system_instruction::transfer(whitelist_account.key, authority.key, whitelist_lamports),
			&[
				whitelist_account.clone(),
				authority.clone(),
				system_program.clone(),
			],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;

		msg!(
			"Terminated whitelist reclaimed sol: {} lamports",
			whitelist_lamports + vault_lamports
		);
		Ok(())
	}
}

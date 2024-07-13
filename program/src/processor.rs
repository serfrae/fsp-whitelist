use {
	crate::{
		error::WhitelistError,
		get_user_ticket_address, get_whitelist_address,
		instructions::WhitelistInstruction,
		state::{Ticket, Whitelist},
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
	spl_token_2022::{
		extension::StateWithExtensions,
		state::{Account, Mint},
	},
};

pub struct Processor;

impl Processor {
	pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
		if program_id != &crate::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let instruction: WhitelistInstruction = WhitelistInstruction::try_from_slice(data)
			.map_err(|_| ProgramError::InvalidInstructionData)?;

		let _ = match instruction {
			WhitelistInstruction::InitialiseWhitelist {
				treasury,
				token_price,
				whitelist_size,
				allow_registration,
				buy_limit,
				registration_start_timestamp,
				registration_duration,
				sale_start_timestamp,
				sale_duration,
			} => Self::process_init(
				accounts,
				&treasury,
				token_price,
				whitelist_size,
				buy_limit,
				allow_registration,
				registration_start_timestamp,
				registration_duration,
				sale_start_timestamp,
				sale_duration,
			),
			WhitelistInstruction::AddUser => Self::process_add_user(accounts),
			WhitelistInstruction::RemoveUser => Self::process_remove_user(accounts),
			WhitelistInstruction::AmendWhitelistSize { size } => {
				Self::process_amend_whitelist_size(accounts, size)
			}
			WhitelistInstruction::AmendTimes {
				registration_start_timestamp,
				registration_duration,
				sale_start_timestamp,
				sale_duration,
			} => Self::process_amend_times(
				accounts,
				registration_start_timestamp,
				registration_duration,
				sale_start_timestamp,
				sale_duration,
			),
			WhitelistInstruction::AllowRegister { allow_registration } => {
				Self::process_allow_register(accounts, allow_registration)
			}
			WhitelistInstruction::Register => Self::process_register(accounts),
			WhitelistInstruction::Unregister => Self::process_unregister(accounts),
			WhitelistInstruction::Buy { amount } => Self::process_buy(accounts, amount),
			WhitelistInstruction::DepositTokens { amount } => {
				Self::process_deposit_tokens(accounts, amount)
			}
			WhitelistInstruction::StartRegistration => Self::process_start_registration(accounts),
			WhitelistInstruction::StartTokenSale => Self::process_start_token_sale(accounts),
			WhitelistInstruction::TransferTokens => Self::process_transfer_tokens(accounts),
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
		treasury: &Pubkey,
		token_price: u64,
		whitelist_size: Option<u64>,
		buy_limit: u64,
		allow_registration: bool,
		registration_start_timestamp: Option<i64>,
		registration_duration: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_duration: Option<i64>,
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

		let (wl, bump) = crate::get_whitelist_address(mint.key);

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

		if whitelist_account.owner != &crate::id() {
			msg!("Initialising whitelist account");
			invoke_signed(
				&system_instruction::create_account(
					authority.key,
					&wl,
					rent.minimum_balance(Whitelist::LEN)
						.max(1)
						.saturating_sub(whitelist_account.lamports()),
					Whitelist::LEN as u64,
					&crate::id(),
				),
				&[
					authority.clone(),
					whitelist_account.clone(),
					system_program.clone(),
				],
				&[&[SEED, mint.key.as_ref(), &[bump]]],
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
				&[&[SEED, mint.key.as_ref(), &[bump]]],
			)?;
		}

		let whitelist_state = Whitelist {
			bump,
			authority: *authority.key,
			vault: *vault.key,
			mint: *mint.key,
			treasury: *treasury,
			token_price,
			buy_limit,
			deposited: 0,
			whitelist_size,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		};

		whitelist_state.check_times()?;

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
		let user_ticket_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let rent = Rent::get()?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		let (wl, _bump) = crate::get_whitelist_address(mint.key);
		let (user_ticket, user_bump) = crate::get_user_ticket_address(user_account.key, &wl);

		if whitelist_account.key != &wl {
			return Err(WhitelistError::IncorrectWhitelistAddress.into());
		}

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::SignerError.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if user_ticket_account.key != &user_ticket {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if user_ticket_account.owner != &crate::id() {
			msg!("Creating user whitelist account");
			invoke_signed(
				&system_instruction::create_account(
					authority.key,
					&user_ticket,
					rent.minimum_balance(Ticket::LEN)
						.max(1)
						.saturating_sub(user_ticket_account.lamports()),
					Ticket::LEN as u64,
					&crate::id(),
				),
				&[
					authority.clone(),
					user_ticket_account.clone(),
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

		let ticket_data = Ticket {
			bump: user_bump,
			owner: *user_account.key,
			allowance: wl_data.buy_limit,
			payer: *authority.key,
			amount_bought: 0,
		};

		ticket_data.serialize(&mut &mut user_ticket_account.data.borrow_mut()[..])?;

		msg!("User initialised");

		Ok(())
	}

	fn process_remove_user(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let (user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		let ticket_data = Ticket::try_from_slice(&user_ticket_account.data.borrow()[..])?;

		if user_ticket_account.key != &user_ticket || user_bump != ticket_data.bump {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let user_lamports = user_ticket_account.lamports();

		invoke_signed(
			&system_instruction::transfer(user_ticket_account.key, authority.key, user_lamports),
			&[
				user_ticket_account.clone(),
				authority.clone(),
				system_program.clone(),
			],
			&[&[
				SEED,
				user_account.key.as_ref(),
				whitelist_account.key.as_ref(),
			]],
		)?;

		user_ticket_account.assign(&system_program::id());
		user_ticket_account.realloc(0, false)?;
		msg!("User unregistered reclaimed: {} lamports", user_lamports);
		Ok(())
	}

	fn process_amend_whitelist_size(accounts: &[AccountInfo], size: Option<u64>) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.whitelist_size = size;
		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;
		Ok(())
	}

	fn process_amend_times(
		accounts: &[AccountInfo],
		registration_start_timestamp: Option<i64>,
		registration_duration: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_duration: Option<i64>,
	) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		// We generally don't need to check the end times as this will be handled by the state
		// method
		if registration_start_timestamp.is_some() {
			// Abort if registration has already started
			if wl_data
				.registration_start_timestamp
				.is_some_and(|t| t > clock.unix_timestamp)
			{
				return Err(WhitelistError::RegistrationStarted.into());
			}
		}

		// The same safety check as above for the sale
		if sale_start_timestamp.is_some() {
			if wl_data
				.sale_start_timestamp
				.is_some_and(|t| t > clock.unix_timestamp)
			{
				return Err(WhitelistError::SaleStarted.into());
			}
		}

		if registration_start_timestamp.is_some_and(|t| t != 0) {
			wl_data.registration_start_timestamp = registration_start_timestamp;
		} else if registration_start_timestamp.is_some_and(|t| t == 0) {
			wl_data.registration_start_timestamp = None;
		}

		if registration_duration.is_some_and(|t| t != 0) {
			wl_data.registration_duration = registration_duration;
		} else if registration_duration.is_some_and(|t| t == 0) {
			wl_data.registration_duration = None;
		}

		if sale_start_timestamp.is_some_and(|t| t != 0) {
			wl_data.sale_start_timestamp = sale_start_timestamp;
		} else if sale_start_timestamp.is_some_and(|t| t == 0) {
			wl_data.sale_start_timestamp = None;
		}

		if sale_duration.is_some_and(|t| t != 0) {
			wl_data.sale_duration = sale_duration;
		} else if sale_duration.is_some_and(|t| t == 0) {
			wl_data.sale_duration = None;
		}

		wl_data.check_times()?;

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;
		Ok(())
	}

	fn process_allow_register(accounts: &[AccountInfo], allow_registration: bool) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.allow_registration = allow_registration;
		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		msg!("Allow registration: {}", allow_registration);

		Ok(())
	}

	fn process_register(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let (_user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		if wl_data
			.registration_start_timestamp
			.is_some_and(|t| t > clock.unix_timestamp)
		{
			return Err(WhitelistError::RegistrationNotStarted.into());
		}

		if wl_data.registration_start_timestamp.is_some_and(|t| {
			wl_data
				.registration_duration
				.is_some_and(|u| t + u > clock.unix_timestamp)
		}) {
			return Err(WhitelistError::RegistrationFinished.into());
		}

		if user_ticket_account.owner != &crate::id() {
			let rent = Rent::get()?;
			invoke_signed(
				&system_instruction::create_account(
					user_account.key,
					user_ticket_account.key,
					rent.minimum_balance(Ticket::LEN)
						.max(1)
						.saturating_sub(user_ticket_account.lamports()),
					Ticket::LEN as u64,
					&crate::id(),
				),
				&[
					user_account.clone(),
					user_ticket_account.clone(),
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

		let ticket_data = Ticket {
			bump: user_bump,
			owner: *user_account.key,
			allowance: wl_data.buy_limit,
			payer: *user_account.key,
			amount_bought: 0,
		};

		ticket_data.serialize(&mut &mut user_ticket_account.data.borrow_mut()[..])?;

		Ok(())
	}

	fn process_unregister(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let ticket_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let (user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let ticket_data = Ticket::try_from_slice(&user_ticket_account.data.borrow()[..])?;

		if user_account.key != &ticket_data.owner {
			return Err(WhitelistError::Unauthorised.into());
		}

		let payer_account = if &ticket_data.payer == authority.key {
			authority
		} else if &ticket_data.payer == user_account.key {
			user_account
		} else {
			return Err(WhitelistError::IncorrectPayer.into());
		};

		if user_ticket_account.key != &user_ticket || user_bump != ticket_data.bump {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		// Check if the ticket token account exists if it does, we will transfer all the tokens
		// back to the vault and then lamports back to the authority
		if ticket_token_account.owner != &spl_token_2022::id()
			|| ticket_token_account.owner != &spl_token::id()
		{
			let borrowed_ticket_token_account_data = ticket_token_account.data.borrow();
			let ticket_token_account_data =
				StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_account_data)?;
			let borrowed_mint_data = mint.data.borrow();
			let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;

			if ticket_token_account_data.base.amount > 0 {
				//Transfer tokens
				//TODO: CHECK
				invoke_signed(
					&spl_token_2022::instruction::transfer_checked(
						&spl_token_2022::id(),
						ticket_token_account.key,
						mint.key,
						vault.key,
						whitelist_account.key,
						&[whitelist_account.key],
						ticket_token_account_data.base.amount,
						mint_data.base.decimals,
					)?,
					&[
						ticket_token_account.clone(),
						mint.clone(),
						vault.clone(),
						whitelist_account.clone(),
						token_program.clone(),
					],
					&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
				)?;
			}

			// Close the account
			invoke_signed(
				&spl_token_2022::instruction::close_account(
					&spl_token_2022::id(),
					ticket_token_account.key,
					authority.key,
					user_ticket_account.key,
					&[whitelist_account.key],
				)?,
				&[
					ticket_token_account.clone(),
					authority.clone(),
					user_ticket_account.clone(),
					token_program.clone(),
				],
				&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
			)?;
		}

		let user_lamports = user_ticket_account.lamports();

		invoke_signed(
			&system_instruction::transfer(
				user_ticket_account.key,
				payer_account.key,
				user_lamports,
			),
			&[
				user_ticket_account.clone(),
				payer_account.clone(),
				system_program.clone(),
			],
			&[&[
				SEED,
				user_account.key.as_ref(),
				whitelist_account.key.as_ref(),
			]],
		)?;

		user_ticket_account.assign(&system_program::id());
		user_ticket_account.realloc(0, false)?;
		msg!("User unregistered reclaimed: {} lamports", user_lamports);
		Ok(())
	}

	fn process_buy(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let ticket_token_account = next_account_info(accounts_iter)?;
		let user_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let mut ticket_data = Ticket::try_from_slice(&user_ticket_account.data.borrow()[..])?;
		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let borrowed_vault_data = vault.data.borrow();
		let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;
		let borrowed_ticket_token_account_data = ticket_token_account.data.borrow();
		let ticket_token_account_data =
			StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_account_data)?;

		let token_amount =
			spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		let (wl, _wl_bump) = get_whitelist_address(&mint.key);
		let (user_ticket, user_bump) = get_user_ticket_address(&user_account.key, &wl);

		if !user_account.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault_data.base.amount < token_amount {
			return Err(WhitelistError::InsufficientFunds.into());
		}

		let sol_amount = match token_amount.checked_mul(wl_data.token_price) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};

		if wl_data
			.sale_start_timestamp
			.is_some_and(|t| t > clock.unix_timestamp)
		{
			return Err(WhitelistError::SaleNotStarted.into());
		}

		if wl_data.sale_start_timestamp.is_some_and(|t| {
			wl_data
				.sale_duration
				.is_some_and(|u| t + u > clock.unix_timestamp)
		}) {
			return Err(WhitelistError::SaleEnded.into());
		}

		if ticket_data.allowance - ticket_data.amount_bought < token_amount {
			return Err(WhitelistError::BuyLimitExceeded.into());
		}

		// We'll check for a `user_token_account` and create one if it doesn't exist
		if user_token_account.owner != &spl_token_2022::id()
			|| user_token_account.owner != &spl_token::id()
		{
			invoke(
				&spl_associated_token_account::instruction::create_associated_token_account(
					user_account.key,
					user_token_account.key,
					mint.key,
					&spl_token_2022::id(),
				),
				&[
					user_account.clone(),
					user_token_account.clone(),
					user_account.clone(),
					mint.clone(),
					system_program.clone(),
					token_program.clone(),
					assc_token_program.clone(),
				],
			)?;
		}

		// We transfer to the Ticket PDA to allow for parallel execution this can later be
		// retrieved by the authority
		invoke(
			&system_instruction::transfer(user_account.key, user_ticket_account.key, sol_amount),
			&[user_account.clone(), user_ticket_account.clone()],
		)?;

		// We check to see if the tokens already exist in the ticket token account
		// if they do we transfer from that account to the user's token account, if they don't
		// we must transfer from the vault

		if ticket_token_account_data.base.amount > 0 {
			invoke_signed(
				&spl_token_2022::instruction::transfer_checked(
					token_program.key,
					ticket_token_account.key,
					mint.key,
					user_token_account.key,
					whitelist_account.key,
					&[],
					token_amount,
					mint_data.base.decimals,
				)?,
				&[
					ticket_token_account.clone(),
					mint.clone(),
					user_token_account.clone(),
					whitelist_account.clone(),
				],
				&[&[
					SEED,
					user_account.key.as_ref(),
					whitelist_account.key.as_ref(),
				]],
			)?;
		} else {
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
				&[&[SEED, mint.key.as_ref()]],
			)?;
		}

		ticket_data.amount_bought = match ticket_data.amount_bought.checked_add(token_amount) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};
		ticket_data.serialize(&mut &mut user_account.data.borrow_mut()[..])?;
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

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let borrowed_vault_data = vault.data.borrow();
		let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;

		let mut token_amount =
			spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		let (wl, wl_bump) = get_whitelist_address(mint.key);

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

		// Checks if the deposited amount will exceed the amount of tokens necessary to fulfil all
		// tickets and sends back excess tokens
		token_amount = if let Some(size) = wl_data.whitelist_size {
			let max_tokens = match size.checked_mul(wl_data.buy_limit) {
				Some(x) => x,
				None => return Err(WhitelistError::Overflow.into()),
			};

			let new_vault_amount = match token_amount.checked_add(vault_data.base.amount) {
				Some(x) => x,
				None => return Err(WhitelistError::Overflow.into()),
			};

			if max_tokens < new_vault_amount {
				msg!("Deposited tokens will be greater than the amount necessary to fulfill all tickets,
automatically setting the deposited token amount to fulfill the maximum required tokens");

				match max_tokens.checked_sub(vault_data.base.amount) {
					Some(x) => x,
					None => return Err(WhitelistError::Overflow.into()),
				}
			} else {
				token_amount
			}
		} else {
			token_amount
		};

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

		wl_data.deposited = match wl_data.deposited.checked_add(token_amount) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		msg!("Deposited: {}", token_amount);
		Ok(())
	}

	fn process_start_registration(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.registration_start_timestamp = None;
		if !wl_data.allow_registration {
			wl_data.allow_registration = true;
		}

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		Ok(())
	}

	fn process_start_token_sale(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.sale_start_timestamp = None;

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		Ok(())
	}

	fn process_transfer_tokens(accounts: &[AccountInfo]) -> ProgramResult {
		let transfer_amount: u64;
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let ticket_account = next_account_info(accounts_iter)?;
		let ticket_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		let (ticket_addr, _) = get_user_ticket_address(&user_account.key, &whitelist_account.key);

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let borrowed_ticket_token_account_data = ticket_token_account.data.borrow();
		let ticket_token_account_data =
			StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_account_data)?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}
		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}
		if vault.key != &wl_data.vault {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}
		if ticket_account.key != &ticket_addr {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		//Check to see if the `ticket_token_account` is initialised intialise it if not
		if ticket_token_account.owner != &spl_token_2022::id()
			|| ticket_token_account.owner != &spl_token::id()
		{
			invoke_signed(
				&spl_associated_token_account::instruction::create_associated_token_account(
					&authority.key,
					&ticket_token_account.key,
					&mint.key,
					&token_program.key,
				),
				&[
					authority.clone(),
					ticket_token_account.clone(),
					ticket_account.clone(),
					mint.clone(),
					system_program.clone(),
					token_program.clone(),
					assc_token_program.clone(),
				],
				&[&[
					SEED,
					user_account.key.as_ref(),
					whitelist_account.key.as_ref(),
				]],
			)?;
		}

		if ticket_token_account_data.base.amount > 0 {
			transfer_amount = match wl_data
				.buy_limit
				.checked_sub(ticket_token_account_data.base.amount)
			{
				Some(x) => x,
				None => return Err(WhitelistError::Overflow.into()),
			};
		} else {
			transfer_amount = wl_data.buy_limit;
		}

		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				ticket_token_account.key,
				authority.key,
				&[],
				transfer_amount,
				mint_data.base.decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
				ticket_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref()]],
		)?;

		Ok(())
	}

	// Only withdraws tokens from the vault, to withdraw tokens from ticket PDAs we terminate them
	fn process_withdraw_tokens(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let recipient_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		wl_data.check_sale_time()?;

		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let token_amount =
			spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals);

		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				recipient_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
				mint_data.base.decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
				recipient_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
		)?;

		msg!("Withdrawn: {}", token_amount);
		Ok(())
	}

	// Withdraw any sol accidentally transferred into the whitelist PDA
	fn process_withdraw_sol(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let recipient_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		invoke_signed(
			&system_instruction::transfer(whitelist_account.key, authority.key, amount),
			&[
				whitelist_account.clone(),
				recipient_account.clone(),
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
		let recipient_account = next_account_info(accounts_iter)?;
		let recipient_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let whitelist_lamports = whitelist_account.lamports();
		let vault_lamports = vault.lamports();
		let borrowed_vault_data = vault.data.borrow();
		let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;
		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		wl_data.check_sale_time()?;

		// Transfer remaining tokens out of the vault
		if vault_data.base.amount != 0 {
			invoke_signed(
				&spl_token_2022::instruction::transfer_checked(
					token_program.key,
					vault.key,
					mint.key,
					recipient_token_account.key,
					whitelist_account.key,
					&[],
					vault_data.base.amount,
					mint_data.base.decimals,
				)?,
				&[
					vault.clone(),
					mint.clone(),
					recipient_token_account.clone(),
					whitelist_account.clone(),
				],
				&[&[SEED, mint.key.as_ref(), authority.key.as_ref()]],
			)?;
		}

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
				recipient_account.clone(),
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

#[cfg(test)]
mod tests {
	use {
		super::*,
		//chrono::NaiveDateTime,
		solana_program_test::*,
		solana_sdk::{
			hash::Hash, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
		},
		test_case::test_case,
	};

	//let datetime = NaiveDateTime::parse_from_str(date_string.as_str(), "%Y-%m-%s %H:%M:%S")?;

	async fn setup_test_environment() -> (BanksClient, Keypair, Hash) {
		let mut program_test =
			ProgramTest::new("stuk_wl", crate::id(), processor!(Processor::process));

		program_test.add_program(
			"spl_token_2022",
			spl_token_2022::id(),
			processor!(spl_token_2022::processor::Processor::process),
		);
		program_test.add_program(
			"spl_token",
			spl_token::id(),
			processor!(spl_token::processor::Processor::process),
		);
		program_test.add_program(
			"spl_associated_token_account",
			spl_associated_token_account::id(),
			processor!(spl_associated_token_account::processor::process_instruction),
		);

		program_test.start().await
	}

	async fn create_mint(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		token_program_id: &Pubkey,
		decimals: u8,
	) -> Keypair {
		let mint_keypair = Keypair::new();
		let mint_rent = banks_client.get_rent().await.unwrap().minimum_balance(82);

		let init_mint = {
			if token_program_id == &spl_token_2022::id() {
				spl_token_2022::instruction::initialize_mint(
					&spl_token_2022::id(),
					&mint_keypair.pubkey(),
					&payer.pubkey(),
					None,
					decimals,
				)
			} else {
				spl_token::instruction::initialize_mint(
					&spl_token::id(),
					&mint_keypair.pubkey(),
					&payer.pubkey(),
					None,
					decimals,
				)
			}
		};

		let instructions = [
			system_instruction::create_account(
				&payer.pubkey(),
				&mint_keypair.pubkey(),
				mint_rent,
				82,
				&spl_token_2022::id(),
			),
			init_mint.unwrap(),
		];

		let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
		transaction.sign(&[payer, &mint_keypair], *recent_blockhash);

		banks_client.process_transaction(transaction).await.unwrap();

		println!("Mint created");
		mint_keypair
	}

	async fn mint_tokens(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		mint: &Keypair,
		token_program_id: &Pubkey,
	) -> Pubkey {
		let token_account = spl_associated_token_account::get_associated_token_address(
			&payer.pubkey(),
			&mint.pubkey(),
		);
		let rent = banks_client.get_rent().await.unwrap();
		let mut transaction = Transaction::new_with_payer(
			&[system_instruction::transfer(
				&payer.pubkey(),
				&token_account,
				rent.minimum_balance(0) + 1,
			)],
			Some(&payer.pubkey()),
		);
		transaction.sign(&[&payer], *recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		let create_token_account_ix = {
			if token_program_id == &spl_token_2022::id() {
				spl_associated_token_account::instruction::create_associated_token_account(
					&payer.pubkey(),
					&token_account,
					&mint.pubkey(),
					&spl_token_2022::id(),
				)
			} else {
				spl_associated_token_account::instruction::create_associated_token_account(
					&payer.pubkey(),
					&token_account,
					&mint.pubkey(),
					&spl_token::id(),
				)
			}
		};

		let mint_to_ix = {
			if token_program_id == &spl_token_2022::id() {
				spl_token_2022::instruction::mint_to(
					&spl_token_2022::id(),
					&mint.pubkey(),
					&token_account,
					&payer.pubkey(),
					&[],
					10_000,
				)
			} else {
				spl_token::instruction::mint_to(
					&spl_token::id(),
					&mint.pubkey(),
					&token_account,
					&payer.pubkey(),
					&[],
					10_000,
				)
			}
		}
		.unwrap();

		let mut transaction = Transaction::new_with_payer(
			&[create_token_account_ix, mint_to_ix],
			Some(&payer.pubkey()),
		);

		transaction.sign(&[payer], *recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
		println!("Tokens minted");
		token_account
	}

	async fn create_whitelist(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		whitelist: &Pubkey,
		vault: &Pubkey,
		mint: &Pubkey,
		treasury: &Pubkey,
		token_price: u64,
		buy_limit: u64,
		whitelist_size: Option<u64>,
		allow_registration: bool,
		registration_start_timestamp: Option<i64>,
		registration_duration: Option<i64>,
		sale_start_timestamp: Option<i64>,
		sale_duration: Option<i64>,
	) -> Result<(), ProgramError> {
		let init_whitelist = crate::instructions::init_whitelist(
			whitelist,
			&payer.pubkey(),
			vault,
			mint,
			treasury,
			token_price,
			buy_limit,
			whitelist_size,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[init_whitelist], Some(&payer.pubkey()));
		transaction.sign(&[payer], *recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
		Ok(())
	}

	async fn create_default_whitelist(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		token_program_id: &Pubkey,
	) -> (Keypair, Pubkey, Keypair, Keypair) {
		let whitelist = Keypair::new();
		let treasury = Keypair::new();
		let mint = create_mint(banks_client, &payer, &recent_blockhash, token_program_id, 9).await;
		let vault = spl_associated_token_account::get_associated_token_address(
			&mint.pubkey(),
			&payer.pubkey(),
		);

		let token_price = 1;
		let buy_limit = 10;
		let whitelist_size = Some(5);
		let allow_registration = true;
		let registration_start_timestamp = None;
		let registration_duration = None;
		let sale_start_timestamp = None;
		let sale_duration = None;

		let ix = crate::instructions::init_whitelist(
			&whitelist.pubkey(),
			&payer.pubkey(),
			&vault,
			&mint.pubkey(),
			&treasury.pubkey(),
			token_price,
			buy_limit,
			whitelist_size,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], *recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		println!("Whitelist initialised");
		(whitelist, vault, mint, treasury)
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_init_whitelist(token_program_id: Pubkey) {
		let whitelist_keypair = Keypair::new();
		let treasury_keypair = Keypair::new();

		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let mint_keypair = create_mint(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
			9,
		)
		.await;

		let vault = spl_associated_token_account::get_associated_token_address(
			&mint_keypair.pubkey(),
			&payer.pubkey(),
		);
		let token_price = 1;
		let buy_limit = 10;
		let whitelist_size = Some(5);
		let allow_registration = true;
		let registration_start_timestamp = None;
		let registration_duration = None;
		let sale_start_timestamp = None;
		let sale_duration = None;

		let ix = crate::instructions::init_whitelist(
			&whitelist_keypair.pubkey(),
			&payer.pubkey(),
			&vault,
			&mint_keypair.pubkey(),
			&treasury_keypair.pubkey(),
			token_price,
			buy_limit,
			whitelist_size,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
		)
		.unwrap();
		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_add_user(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _, mint, _) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let user_keypair = Keypair::new();
		let (user_ticket, _) = get_user_ticket_address(&user_keypair.pubkey(), &whitelist.pubkey());
		let ix = crate::instructions::add_user(
			&whitelist.pubkey(),
			&payer.pubkey(),
			&mint.pubkey(),
			&user_keypair.pubkey(),
			&user_ticket,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_remove_user(token_program_id: Pubkey) {
		let user_keypair = Keypair::new();
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let (user_ticket, _) = get_user_ticket_address(&user_keypair.pubkey(), &whitelist.pubkey());

		let ix = crate::instructions::remove_user(
			&whitelist.pubkey(),
			&payer.pubkey(),
			&mint.pubkey(),
			&user_keypair.pubkey(),
			&user_ticket,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}
	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_deposit_tokens(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (binary_program, vault, mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let payer_token_account = mint_tokens(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&mint,
			&token_program_id,
		)
		.await;

		let ix = crate::instructions::deposit_tokens(
			&whitelist.pubkey(),
			&vault,
			&payer.pubkey(),
			&payer_token_account,
			&mint.pubkey(),
			42,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_buy_tokens(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, vault, mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;
		let user_keypair = Keypair::new();
		let (user_ticket, _) = get_user_ticket_address(&user_keypair.pubkey(), &whitelist.pubkey());
		let user_token_account = spl_associated_token_account::get_associated_token_address(
			&user_keypair.pubkey(),
			&mint.pubkey(),
		);

		let ix = crate::instructions::buy_tokens(
			&whitelist.pubkey(),
			&vault,
			&mint.pubkey(),
			&user_keypair.pubkey(),
			&user_ticket,
			&user_token_account,
			&system_program::id(),
			42,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}
}

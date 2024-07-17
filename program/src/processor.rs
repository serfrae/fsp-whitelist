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

		match instruction {
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
			WhitelistInstruction::BurnTicket => Self::process_burn_ticket(accounts),
			WhitelistInstruction::TerminateWhitelist => Self::process_terminate_whitelist(accounts),
		}
	}

	fn process_init(
		accounts: &[AccountInfo],
		treasury: &Pubkey,
		token_price: u64,
		whitelist_size: u64,
		buy_limit: u64,
		allow_registration: bool,
		registration_start_timestamp: i64,
		registration_duration: i64,
		sale_start_timestamp: i64,
		sale_duration: i64,
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
		let mint_decimals = {
			let borrowed_mint_data = mint.data.borrow();
			let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
			mint_data.base.decimals
		};

		// Safety dance
		if whitelist_account.key != &wl {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !authority.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault.key
			!= &spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist_account.key,
				&mint.key,
				&token_program.key,
			) {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.owner != &spl_token_2022::id() && mint.owner != &spl_token::id() {
			return Err(WhitelistError::IllegalMintOwner.into());
		}

		if token_program.key != &spl_token_2022::id() && token_program.key != &spl_token::id() {
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
					&whitelist_account.key,
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
					&whitelist_account.key,
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

			let buy_limit = spl_token_2022::ui_amount_to_amount(buy_limit as f64, mint_decimals);

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
				registration_timestamp: registration_start_timestamp,
				registration_duration,
				sale_timestamp: sale_start_timestamp,
				sale_duration,
			};

			whitelist_state.check_times()?;

			whitelist_state.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;
			msg!("Whitelist initialised");

			Ok(())
		} else {
			return Err(WhitelistError::WhitelistAlreadyInitialized.into());
		}
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
			whitelist: *whitelist_account.key,
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
		msg!("Process: Remove user");
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let (wl, _bump) = get_whitelist_address(&mint.key);
		let (user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);
		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let ticket_data = Ticket::try_from_slice(&user_ticket_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if whitelist_account.key != &wl {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if user_ticket_account.key != &user_ticket || user_bump != ticket_data.bump {
			return Err(WhitelistError::IncorrectUserAccount.into());
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		let user_lamports = user_ticket_account.lamports();

		user_ticket_account.assign(&system_program::id());
		user_ticket_account.realloc(0, false)?;
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
				&[user_bump],
			]],
		)?;

		msg!("User unregistered reclaimed: {} lamports", user_lamports);
		Ok(())
	}

	fn process_amend_whitelist_size(accounts: &[AccountInfo], size: u64) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.whitelist_size = size;
		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;
		Ok(())
	}

	fn process_amend_times(
		accounts: &[AccountInfo],
		registration_timestamp: Option<i64>,
		registration_duration: Option<i64>,
		sale_timestamp: Option<i64>,
		sale_duration: Option<i64>,
	) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if registration_timestamp.is_some() && wl_data.registration_timestamp > clock.unix_timestamp
		{
			// Abort if registration has already started
			return Err(WhitelistError::RegistrationStarted.into());
		}

		// The same safety check as above for the sale
		if sale_timestamp.is_some() && wl_data.sale_timestamp > clock.unix_timestamp {
			return Err(WhitelistError::SaleStarted.into());
		}

		// safe to unwrap
		if registration_timestamp.is_some() {
			wl_data.registration_timestamp = registration_timestamp.unwrap();
		}

		if registration_duration.is_some() {
			wl_data.registration_duration = registration_duration.unwrap();
		}

		if sale_timestamp.is_some() {
			wl_data.sale_timestamp = sale_timestamp.unwrap();
		}

		if sale_duration.is_some() {
			wl_data.sale_duration = sale_duration.unwrap();
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

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.allow_registration = allow_registration;
		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		msg!("Allow registration: {}", allow_registration);

		Ok(())
	}

	fn process_register(accounts: &[AccountInfo]) -> ProgramResult {
		msg!("Process: Register");
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let user_account = next_account_info(accounts_iter)?;
		let user_ticket_account = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let (_user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		if wl_data.registration_timestamp > 0
			&& wl_data.registration_timestamp > clock.unix_timestamp
		{
			return Err(WhitelistError::RegistrationNotStarted.into());
		}

		if wl_data.registration_timestamp > 0
			&& wl_data.registration_timestamp + wl_data.registration_duration > clock.unix_timestamp
		{
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
			whitelist: *whitelist_account.key,
			owner: *user_account.key,
			allowance: wl_data.buy_limit,
			payer: *user_account.key,
			amount_bought: 0,
		};

		ticket_data.serialize(&mut &mut user_ticket_account.data.borrow_mut()[..])?;

		msg!("Registration successful");
		Ok(())
	}

	fn process_unregister(accounts: &[AccountInfo]) -> ProgramResult {
		msg!("Process: Unregister");
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

		let clock = Clock::get()?;

		let (user_ticket, user_bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let ticket_data = Ticket::try_from_slice(&user_ticket_account.data.borrow()[..])?;

		if authority.key != &wl_data.authority {
			return Err(WhitelistError::AccountMismatch.into());
		}

		if vault.key != &wl_data.vault {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

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

		if token_program.key != &spl_token_2022::id() && token_program.key != &spl_token::id() {
			return Err(ProgramError::IncorrectProgramId);
		}
		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		// As this PDA is expected to hold funds, and registration spaces are limited, a user
		// should only be able to unregister during the registration period, if the registration
		// period is occuring in parallel to the the sale period then a user should not be able to
		// unregister, we could check for lamports in excess of the minimum balance, but it is
		// simpler to not permit the user to unregister once a token sale has begun.
		if (wl_data.registration_timestamp > 0
			&& wl_data.registration_timestamp + wl_data.registration_duration
				> clock.unix_timestamp)
			|| wl_data.registration_duration == 0
			|| wl_data.sale_duration == 0
		{
			return Err(WhitelistError::CannotUnregister.into());
		}

		// Check if the ticket token account exists if it does, we will transfer all the tokens
		// back to the vault and then lamports back to the authority
		if ticket_token_account.owner != &spl_token_2022::id()
			&& ticket_token_account.owner != &spl_token::id()
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

		user_ticket_account.assign(&system_program::id());
		user_ticket_account.realloc(0, false)?;
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
				&[user_bump],
			]],
		)?;

		msg!("User unregistered reclaimed: {} lamports", user_lamports);
		Ok(())
	}

	fn process_buy(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		msg!("Process: Buy");
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

		if vault.key != &wl_data.vault {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		let ticket_account_token_amount = {
			if ticket_token_account.owner == &spl_token_2022::id()
				|| ticket_token_account.owner == &spl_token::id()
			{
				let borrowed_ticket_token_account_data = ticket_token_account.data.borrow();
				let ticket_token_account_data =
					StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_account_data)?;
				ticket_token_account_data.base.amount
			} else {
				0
			}
		};

		let (mint_decimals, token_amount) = {
			let borrowed_mint_data = mint.data.borrow();
			let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
			(
				mint_data.base.decimals,
				spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals),
			)
		};

		if !user_account.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		{
			let borrowed_vault_data = vault.data.borrow();
			let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;
			if vault_data.base.amount < token_amount {
				return Err(WhitelistError::InsufficientFunds.into());
			}
		}

		let sol_amount = match token_amount.checked_mul(wl_data.token_price) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};

		if wl_data.sale_timestamp > 0 && wl_data.sale_timestamp > clock.unix_timestamp {
			return Err(WhitelistError::SaleNotStarted.into());
		}

		if wl_data.sale_timestamp > 0
			&& wl_data.sale_timestamp + wl_data.sale_duration > clock.unix_timestamp
		{
			return Err(WhitelistError::SaleEnded.into());
		}

		if ticket_data.allowance - ticket_data.amount_bought < token_amount {
			return Err(WhitelistError::BuyLimitExceeded.into());
		}

		// We'll check for a `user_token_account` and create one if it doesn't exist
		if user_token_account.owner != &spl_token_2022::id()
			&& user_token_account.owner != &spl_token::id()
		{
			invoke(
				&spl_associated_token_account::instruction::create_associated_token_account(
					user_account.key,
					user_token_account.key,
					mint.key,
					token_program.key,
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
		if ticket_account_token_amount > 0 {
			invoke_signed(
				&spl_token_2022::instruction::transfer_checked(
					token_program.key,
					ticket_token_account.key,
					mint.key,
					user_token_account.key,
					whitelist_account.key,
					&[],
					token_amount,
					mint_decimals,
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
					&[ticket_data.bump],
				]],
			)?;
		}
		invoke_signed(
			&spl_token_2022::instruction::transfer_checked(
				token_program.key,
				vault.key,
				mint.key,
				user_token_account.key,
				whitelist_account.key,
				&[],
				token_amount,
				mint_decimals,
			)?,
			&[
				vault.clone(),
				mint.clone(),
				user_token_account.clone(),
				whitelist_account.clone(),
			],
			&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
		)?;

		ticket_data.amount_bought = match ticket_data.amount_bought.checked_add(token_amount) {
			Some(x) => x,
			None => return Err(WhitelistError::Overflow.into()),
		};
		ticket_data.serialize(&mut &mut user_ticket_account.data.borrow_mut()[..])?;
		msg!("Bought: {}", amount);
		Ok(())
	}

	fn process_deposit_tokens(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
		msg!("Process: Deposit");
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let vault = next_account_info(accounts_iter)?;
		let depositor_account = next_account_info(accounts_iter)?;
		let depositor_token_account = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		let (mint_decimals, mut token_amount) = {
			let borrowed_mint_data = mint.data.borrow();
			let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
			(
				mint_data.base.decimals,
				spl_token_2022::ui_amount_to_amount(amount as f64, mint_data.base.decimals),
			)
		};

		let (wl, wl_bump) = get_whitelist_address(mint.key);

		if whitelist_account.key != &wl || wl_bump != wl_data.bump {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !depositor_account.is_signer {
			return Err(WhitelistError::SignerError.into());
		}

		if vault.key != &wl_data.vault
			|| vault.key
				!= &spl_associated_token_account::get_associated_token_address_with_program_id(
					whitelist_account.key,
					mint.key,
					token_program.key,
				) {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if token_program.key != &spl_token_2022::id() && token_program.key != &spl_token::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		// Checks if the deposited amount will exceed the amount of tokens necessary to fulfil all
		// tickets and sends back excess tokens
		token_amount = {
			if wl_data.whitelist_size > 0 {
				let borrowed_vault_data = vault.data.borrow();
				let vault_data = StateWithExtensions::<Account>::unpack(&borrowed_vault_data)?;
				let max_tokens = match wl_data.whitelist_size.checked_mul(wl_data.buy_limit) {
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
			}
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
				mint_decimals,
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
		msg!("Process: Start registration");
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;

		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.registration_timestamp = clock.unix_timestamp;
		if !wl_data.allow_registration {
			wl_data.allow_registration = true;
		}

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		msg!("Registration commenced");
		Ok(())
	}

	fn process_start_token_sale(accounts: &[AccountInfo]) -> ProgramResult {
		msg!("Process: Start token sale");
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;

		let clock = Clock::get()?;
		let mut wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		wl_data.sale_timestamp = clock.unix_timestamp;

		wl_data.serialize(&mut &mut whitelist_account.data.borrow_mut()[..])?;

		msg!("Sale commenced");
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

		let (ticket_addr, bump) =
			get_user_ticket_address(&user_account.key, &whitelist_account.key);

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let borrowed_mint_data = mint.data.borrow();
		let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
		let borrowed_ticket_token_account_data = ticket_token_account.data.borrow();
		let ticket_token_account_data =
			StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_account_data)?;

		if whitelist_account.owner != &crate::id() {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

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
			&& ticket_token_account.owner != &spl_token::id()
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
					&[bump],
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
			&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
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

		if whitelist_account.owner != &crate::id() {
			return Err(WhitelistError::InvalidWhitelistAddress.into());
		}

		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if vault.key != &wl_data.vault {
			return Err(WhitelistError::IncorrectVaultAddress.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if token_program.key != &spl_token_2022::id() && token_program.key != &spl_token::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

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
			&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
		)?;

		msg!("Withdrawn: {}", token_amount);
		Ok(())
	}

	fn process_burn_ticket(accounts: &[AccountInfo]) -> ProgramResult {
		let accounts_iter = &mut accounts.iter();
		let whitelist_account = next_account_info(accounts_iter)?;
		let authority = next_account_info(accounts_iter)?;
		let mint = next_account_info(accounts_iter)?;
		let treasury = next_account_info(accounts_iter)?;
		let treasury_token_account = next_account_info(accounts_iter)?;
		let ticket_account = next_account_info(accounts_iter)?;
		let ticket_token_account = next_account_info(accounts_iter)?;
		let token_program = next_account_info(accounts_iter)?;
		let system_program = next_account_info(accounts_iter)?;
		let assc_token_program = next_account_info(accounts_iter)?;

		let wl_data = Whitelist::try_from_slice(&whitelist_account.data.borrow()[..])?;
		let ticket_data = Ticket::try_from_slice(&ticket_account.data.borrow()[..])?;
		let token_amount = {
			let borrowed_ticket_token_data = ticket_token_account.data.borrow();
			let ticket_data = StateWithExtensions::<Account>::unpack(&borrowed_ticket_token_data)?;
			ticket_data.base.amount
		};
		let mint_decimals = {
			let borrowed_mint_data = mint.data.borrow();
			let mint_data = StateWithExtensions::<Mint>::unpack(&borrowed_mint_data)?;
			mint_data.base.decimals
		};

		// Safety dance
		if !authority.is_signer || authority.key != &wl_data.authority {
			return Err(WhitelistError::Unauthorised.into());
		}

		if mint.key != &wl_data.mint {
			return Err(WhitelistError::IncorrectMintAddress.into());
		}

		if treasury.key != &wl_data.treasury {
			return Err(WhitelistError::IncorrectTreasuryAddress.into());
		}

		if token_program.key != &spl_token_2022::id() && token_program.key != &spl_token::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if system_program.key != &system_program::id() {
			return Err(ProgramError::IncorrectProgramId);
		}

		if assc_token_program.key != &spl_associated_token_account::id() {
			return Err(ProgramError::IncorrectProgramId);
		}
		let ticket_token_lamports = ticket_token_account.lamports();
		let ticket_lamports = ticket_account.lamports();

		if token_amount > 0 {
			// Create the treasury token account if it doesn't exist
			if treasury_token_account.owner != &spl_token_2022::id()
				&& treasury_token_account.owner != &spl_token::id()
			{
				invoke_signed(
					&spl_associated_token_account::instruction::create_associated_token_account(
						&authority.key,
						&treasury_token_account.key,
						&mint.key,
						&token_program.key,
					),
					&[
						authority.clone(),
						treasury_token_account.clone(),
						treasury.clone(),
						mint.clone(),
						system_program.clone(),
						token_program.clone(),
						assc_token_program.clone(),
					],
					&[&[SEED, whitelist_account.key.as_ref(), &[wl_data.bump]]],
				)?
			}
			// Transfer tokens from the ticket token account
			invoke_signed(
				&spl_token_2022::instruction::transfer_checked(
					&token_program.key,
					&ticket_token_account.key,
					&mint.key,
					&treasury_token_account.key,
					&whitelist_account.key,
					&[],
					token_amount,
					mint_decimals,
				)?,
				&[
					ticket_token_account.clone(),
					mint.clone(),
					treasury_token_account.clone(),
					whitelist_account.clone(),
				],
				&[&[
					SEED,
					ticket_data.owner.as_ref(),
					whitelist_account.key.as_ref(),
					&[ticket_data.bump],
				]],
			)?;
		}

		// Close ticket token account
		invoke_signed(
			&spl_token_2022::instruction::close_account(
				&token_program.key,
				&ticket_token_account.key,
				&treasury.key,
				&ticket_account.key,
				&[],
			)?,
			&[
				ticket_token_account.clone(),
				treasury.clone(),
				ticket_account.clone(),
			],
			&[&[
				SEED,
				ticket_data.owner.as_ref(),
				whitelist_account.key.as_ref(),
				&[ticket_data.bump],
			]],
		)?;

		// Zero ticket data
		ticket_account.assign(&system_program::id());
		ticket_account.realloc(0, false)?;

		// Transfer SOL from the ticket
		invoke_signed(
			&system_instruction::transfer(ticket_account.key, treasury.key, ticket_lamports),
			&[
				ticket_account.clone(),
				treasury.clone(),
				system_program.clone(),
			],
			&[&[
				SEED,
				ticket_data.owner.as_ref(),
				whitelist_account.key.as_ref(),
				&[ticket_data.bump],
			]],
		)?;

		msg!(
			"Ticket burned. {} tokens & {} lamports transferred to: {}",
			token_amount,
			(ticket_lamports + ticket_token_lamports),
			treasury.key
		);
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
				&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
			)?;
		}

		invoke_signed(
			&spl_token_2022::instruction::close_account(
				token_program.key,
				vault.key,
				authority.key,
				whitelist_account.key,
				&[],
			)?,
			&[vault.clone(), authority.clone(), whitelist_account.clone()],
			&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
		)?;

		// Close whitelist and reclaim lamports
		whitelist_account.assign(&system_program::id());
		whitelist_account.realloc(0, false)?;
		invoke_signed(
			&system_instruction::transfer(whitelist_account.key, authority.key, whitelist_lamports),
			&[
				whitelist_account.clone(),
				recipient_account.clone(),
				system_program.clone(),
			],
			&[&[SEED, mint.key.as_ref(), &[wl_data.bump]]],
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

		program_test.start().await
	}

	async fn create_mint(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		mint_keypair: &Keypair,
		token_program_id: &Pubkey,
		decimals: u8,
	) {
		let space =
			spl_token_2022::extension::ExtensionType::try_calculate_account_len::<Mint>(&[])
				.unwrap();
		let mint_rent = banks_client
			.get_rent()
			.await
			.unwrap()
			.minimum_balance(space);

		let init_mint = spl_token_2022::instruction::initialize_mint(
			&token_program_id,
			&mint_keypair.pubkey(),
			&payer.pubkey(),
			None,
			decimals,
		);

		let instructions = [
			system_instruction::create_account(
				&payer.pubkey(),
				&mint_keypair.pubkey(),
				mint_rent,
				space as u64,
				&token_program_id,
			),
			init_mint.unwrap(),
		];

		let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
		transaction.sign(&[payer, &mint_keypair], *recent_blockhash);

		banks_client.process_transaction(transaction).await.unwrap();
		let mint_test = banks_client
			.get_account(mint_keypair.pubkey())
			.await
			.expect("get mint account")
			.expect("mint account is none");
		assert_eq!(mint_test.data.len(), space);

		println!("Mint created");
	}

	async fn create_default_whitelist(
		banks_client: &mut BanksClient,
		payer: &Keypair,
		recent_blockhash: &Hash,
		token_program_id: &Pubkey,
	) -> (Pubkey, Pubkey, Keypair, Keypair) {
		let treasury = Keypair::new();
		let mint_keypair = Keypair::new();
		let (whitelist, _) = get_whitelist_address(&mint_keypair.pubkey());
		create_mint(
			banks_client,
			&payer,
			&recent_blockhash,
			&mint_keypair,
			token_program_id,
			9,
		)
		.await;
		let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
			&whitelist,
			&mint_keypair.pubkey(),
			token_program_id,
		);

		let token_price = 1;
		let buy_limit = 10;
		let whitelist_size = 5;
		let allow_registration = true;
		let registration_start_timestamp = 0;
		let registration_duration = 0;
		let sale_start_timestamp = 0;
		let sale_duration = 0;

		let ix = crate::instructions::init_whitelist(
			&whitelist,
			&payer.pubkey(),
			&vault,
			&mint_keypair.pubkey(),
			&treasury.pubkey(),
			token_price,
			buy_limit,
			whitelist_size,
			allow_registration,
			registration_start_timestamp,
			registration_duration,
			sale_start_timestamp,
			sale_duration,
			token_program_id,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], *recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		println!("Whitelist initialised");
		(whitelist, vault, mint_keypair, treasury)
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_init_whitelist(token_program_id: Pubkey) {
		let treasury_keypair = Keypair::new();
		let mint_keypair = Keypair::new();

		let (whitelist, _) = get_whitelist_address(&mint_keypair.pubkey());

		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;

		create_mint(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&mint_keypair,
			&token_program_id,
			9,
		)
		.await;

		let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
			&whitelist,
			&mint_keypair.pubkey(),
			&token_program_id,
		);

		let token_price = 1;
		let buy_limit = 10;
		let whitelist_size = 5;
		let allow_registration = true;
		let registration_start_timestamp = 0;
		let registration_duration = 0;
		let sale_start_timestamp = 0;
		let sale_duration = 0;

		let ix = crate::instructions::init_whitelist(
			&whitelist,
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
			&token_program_id,
		)
		.unwrap();
		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
		let whitelist_account = banks_client
			.get_account(whitelist)
			.await
			.expect("get_account")
			.expect("whitelist account not none");
		let rent = banks_client.get_rent().await.unwrap();
		assert_eq!(whitelist_account.data.len(), Whitelist::LEN);
		assert_eq!(whitelist_account.owner, crate::id());
		assert_eq!(
			whitelist_account.lamports,
			rent.minimum_balance(Whitelist::LEN)
		);
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
		let (user_ticket, _) = get_user_ticket_address(&user_keypair.pubkey(), &whitelist);
		let ix = crate::instructions::add_user(
			&whitelist,
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

		let (user_ticket, _) = get_user_ticket_address(&user_keypair.pubkey(), &whitelist);

		let add_ix = crate::instructions::add_user(
			&whitelist,
			&payer.pubkey(),
			&mint.pubkey(),
			&user_keypair.pubkey(),
			&user_ticket,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[add_ix], Some(&payer.pubkey()));
		transaction.sign(&[&payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		let remove_ix = crate::instructions::remove_user(
			&whitelist,
			&payer.pubkey(),
			&mint.pubkey(),
			&user_keypair.pubkey(),
			&user_ticket,
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[remove_ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_amend_whitelist_size(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix =
			crate::instructions::amend_whitelist_size(&whitelist, &payer.pubkey(), 42).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_amend_times(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix = crate::instructions::amend_times(
			&whitelist,
			&payer.pubkey(),
			None,
			Some(259200000),
			None,
			Some(604800000),
		)
		.unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_allow_registration_false(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix_false =
			crate::instructions::allow_registration(&whitelist, &payer.pubkey(), false).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix_false], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		let whitelist_account = banks_client
			.get_account(whitelist)
			.await
			.expect("get_account")
			.expect("whitelist account not none");
		let wl_data = Whitelist::try_from_slice(&whitelist_account.data[..]).unwrap();
		assert_eq!(wl_data.allow_registration, false);
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_allow_registration_true(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix_true =
			crate::instructions::allow_registration(&whitelist, &payer.pubkey(), true).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix_true], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		let whitelist_account = banks_client
			.get_account(whitelist)
			.await
			.expect("get_account")
			.expect("whitelist account not none");
		let wl_data = Whitelist::try_from_slice(&whitelist_account.data[..]).unwrap();
		assert_eq!(wl_data.allow_registration, true);
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_register(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let (ticket, _) = get_user_ticket_address(&payer.pubkey(), &whitelist);

		let ix = crate::instructions::register(&whitelist, &payer.pubkey(), &ticket).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();

		let ticket_account = banks_client
			.get_account(ticket)
			.await
			.expect("get_account")
			.expect("associated_account not none");

		let rent = banks_client.get_rent().await.unwrap();
		assert_eq!(ticket_account.data.len(), Ticket::LEN);
		assert_eq!(ticket_account.owner, crate::id());
		assert_eq!(ticket_account.lamports, rent.minimum_balance(Ticket::LEN));
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_unregister(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let (ticket, _) = get_user_ticket_address(&payer.pubkey(), &whitelist);

		let ix_true = crate::instructions::register(&whitelist, &payer.pubkey(), &ticket).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix_true], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_start_registration(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix = crate::instructions::start_registration(&whitelist, &payer.pubkey()).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}

	#[test_case(spl_token::id() ; "Token Program")]
	#[test_case(spl_token_2022::id() ; "Token-2022 Program")]
	#[tokio::test]
	async fn test_start_token_sale(token_program_id: Pubkey) {
		let (mut banks_client, payer, recent_blockhash) = setup_test_environment().await;
		let (whitelist, _vault, _mint, _treasury) = create_default_whitelist(
			&mut banks_client,
			&payer,
			&recent_blockhash,
			&token_program_id,
		)
		.await;

		let ix = crate::instructions::start_registration(&whitelist, &payer.pubkey()).unwrap();

		let mut transaction = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
		transaction.sign(&[payer], recent_blockhash);
		banks_client.process_transaction(transaction).await.unwrap();
	}
}

use {
	anyhow::{anyhow, Result},
	borsh::BorshDeserialize,
	chrono::NaiveDateTime,
	clap::{command, Args, Parser, Subcommand},
	solana_cli_config,
	solana_client::rpc_client::RpcClient,
	solana_program::{instruction::Instruction, pubkey::Pubkey},
	solana_sdk::{
		commitment_config::CommitmentConfig,
		signature::{read_keypair_file, Signer},
		transaction::Transaction,
	},
	spl_token_2022::{
		extension::StateWithExtensions,
		state::{Account, Mint},
	},
	stuk_wl::{get_user_ticket_address, get_whitelist_address, instructions},
};

#[derive(Parser, Debug)]
struct Cli {
	#[arg(short, long)]
	config: Option<String>,
	#[arg(short, long)]
	rpc: Option<String>,
	#[arg(short, long)]
	payer: Option<String>,
	#[command(subcommand)]
	cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Initialise a whitelist
	///
	/// This command will initialise a whitelist, the <MINT>, <TREASURY>, <PRICE> and <BUY_LIMIT>
	/// fields are all required, other fields can be provided using --<FIELD_NAME> <VALUE> or
	/// later amended using <AMEND>
	Init(Init),

	/// Add/Remove users from the whitelist
	#[command(subcommand)]
	User(UserManagement),

	/// Buy tokens
	Buy(TokenFields),

	/// Deposit tokens into the vault
	Deposit(TokenFields),

	/// Withdraw tokens from the vault - authority only
	Withdraw(TokenFields),

	/// Amend whitelist size or registration/token sale times/duration
	#[command(subcommand)]
	Amend(Detail),

	/// Commence registration/token sale
	#[command(subcommand)]
	Start(Start),

	AllowRegister {
		/// Mint of the token sale
		mint: Pubkey,

		/// true: allow registration. false: freeze/deny registration
		allow: String,
	},

	/// Register to the whitelist
	Register {
		/// Mint of the token sale
		mint: Pubkey,
	},

	/// Unregister from the whitelist and claim rent
	Unregister {
		/// Mint of the token sale
		mint: Pubkey,
	},

	/// Burn ticket and reclaims tokens + lamports to treasury
	#[command(subcommand)]
	Burn(Method),

	/// Terminate the whitelist and send tokens to the recipient
	Close {
		/// Mint of the token sale
		mint: Pubkey,

		/// Address to send tokens/SOL to, if `None` then defaults to authority wallet
		recipient: Option<Pubkey>,
	},

	/// Get info about the whitelist or a specific ticket
	#[command(subcommand)]
	Info(Info),
}

#[derive(Subcommand, Debug)]
enum UserManagement {
	/// Add a user to the whitelist
	Add(UserManagementCommonFields),

	/// Remove a user from the whitelist and claim rent
	Remove(UserManagementCommonFields),
}

#[derive(Args, Debug)]
struct UserManagementCommonFields {
	/// Public key of the mint of the token associated with the whitelist
	mint: Pubkey,

	/// Public key of the user
	user: Pubkey,
}

#[derive(Subcommand, Debug)]
enum Method {
	/// Withdraw from a single ticket instance
	Single(TicketFields),

	/// Withdraw from all tickets associated with the whitelist
	Bulk {
		/// Mint of the token sale
		mint: Pubkey,
	},
}

#[derive(Subcommand, Debug)]
enum Detail {
	/// Amend registration/sale times/duration
	Times {
		// Mint of the token sale
		mint: Pubkey,

		/// When registration starts. Format: YYYY-MM-DD HH:MM:SS (UTC)
		registration_start_time: Option<String>,

		/// When registration ends. Format: YYYY-MM-DD HH:MM:SS (UTC)
		registration_end_time: Option<String>,

		/// When token sale starts. Format: YYYY-MM-DD HH:MM:SS (UTC)
		sale_start_time: Option<String>,

		/// When token sale stops. Format: YYYY-MM-DD HH:MM:SS (UTC)
		sale_end_time: Option<String>,
	},

	/// Amend whitelist size
	Size {
		/// Mint of the token sale
		mint: Pubkey,

		/// Desired whitelist size. `0` == no limit
		size: u64,
	},
}

#[derive(Subcommand, Debug)]
enum Start {
	/// Commences registration
	Registration { mint: Pubkey },

	/// Commences the token sale
	Sale { mint: Pubkey },
}

#[derive(Subcommand, Debug)]
enum Info {
	/// Get whitelist info
	Whitelist { mint: Pubkey },

	/// Get user info
	User {
		/// Mint of the token sale
		mint: Pubkey,
		/// User ticket address
		user: Pubkey,
	},
}

#[derive(Args, Debug)]
struct TokenFields {
	/// Mint of the token associated with the whitelist
	mint: Pubkey,

	/// Amount of tokens you wish to transfer
	amount: u64,

	/// The wallet address that will receive the tokens
	#[clap(long)]
	recipient: Option<Pubkey>,
}
#[derive(Args, Clone, Debug)]
struct TicketFields {
	/// Mint of the token
	mint: Pubkey,

	/// A user's public key
	user: Pubkey,
}

#[derive(Args, Debug)]
struct Init {
	/// Mint of the token that is to be sold
	mint: Pubkey,

	/// Address that will receive the proceeds of the sale
	treasury: Pubkey,

	/// Price of the token in SOL
	price: u64,

	/// Number of tokens a whitelist member can purchase
	buy_limit: u64,

	/// The number of subscribers allowed in the whitelist
	whitelist_size: u64,

	/// Allow users to register
	///
	/// This flag has two purposes, it can freeze an ongoing registration, or permit
	/// only the authority to add members to the whitelist
	#[clap(long)]
	allow_registration: bool,

	/// When registration starts. Format: YYYY-MM-DD HH:MM:SS
	#[clap(long)]
	registration_start_time: Option<String>,

	/// When registration ends. Format: YYYY-MM-DD HH:MM:SS
	#[clap(long)]
	registration_end_time: Option<String>,

	/// When token sale starts. Format: YYYY-MM-DD HH:MM:SS
	#[clap(long)]
	sale_start_time: Option<String>,

	/// When token sale ends. Format: YYYY-MM-DD HH:MM:SS
	#[clap(long)]
	sale_end_time: Option<String>,
}

fn main() -> Result<()> {
	let args = Cli::parse();

	let solana_config_file = if let Some(ref config) = *solana_cli_config::CONFIG_FILE {
		solana_cli_config::Config::load(config).unwrap_or_default()
	} else {
		solana_cli_config::Config::default()
	};

	let wallet_keypair = if let Some(payer) = args.payer {
		match read_keypair_file(&payer) {
			Ok(keypair) => keypair,
			Err(e) => {
				eprintln!(
					"Unable to read provided keypair file, attempting to set to default: {}",
					e
				);
				read_keypair_file(&solana_config_file.keypair_path)
					.map_err(|err| anyhow!("Unable to read keypair file: {}", err))?
			}
		}
	} else {
		read_keypair_file(&solana_config_file.keypair_path)
			.map_err(|err| anyhow!("Unable to read keypair file: {}", err))?
	};

	let wallet_pubkey = wallet_keypair.pubkey();

	let client = RpcClient::new_with_commitment(
		solana_config_file.json_rpc_url.to_string(),
		CommitmentConfig::confirmed(),
	);

	let instruction: Instruction = match args.cmd {
		Commands::Init(fields) => {
			let whitelist = get_whitelist_address(&fields.mint).0;

			// Retrieve the correct token program from the mint's owner
			let mint_account = client.get_account(&fields.mint)?;
			let token_program = mint_account.owner;

			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist,
				&fields.mint,
				&token_program,
			);

			let registration_start_timestamp = match fields.registration_start_time {
				Some(ref time) => string_to_timestamp(time.to_string())?,
				None => 0,
			};

			let registration_duration = match fields.registration_end_time {
				Some(ref time) => {
					let ts = string_to_timestamp(time.to_string()).expect("error parsing time");

					if registration_start_timestamp > 0 && registration_start_timestamp < ts {
						ts - registration_start_timestamp
					} else {
						return Err(anyhow!(
							"Cannot compute duration, start time is after provided end time"
						));
					}
				}
				None => 0,
			};

			let sale_start_timestamp = match fields.sale_start_time {
				Some(ref time) => string_to_timestamp(time.to_string())?,
				None => 0,
			};

			let sale_duration = match fields.sale_end_time {
				Some(ref time) => {
					let ts = string_to_timestamp(time.to_string()).expect("error parsing time");

					if sale_start_timestamp > 0 && sale_start_timestamp < ts {
						ts - sale_start_timestamp
					} else {
						return Err(anyhow!(
							"Cannot compute duration, start time is after provided end time"
						));
					}
				}
				None => 0,
			};

			println!("Whitelist Account: {}", whitelist);
			println!("Vault Account: {}", vault);
			println!("Treasury: {}", wallet_pubkey);
			println!("Mint: {}", fields.mint);

			instructions::init_whitelist(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&fields.mint,
				&fields.treasury,
				fields.price,
				fields.buy_limit,
				fields.whitelist_size,
				fields.allow_registration,
				registration_start_timestamp,
				registration_duration,
				sale_start_timestamp,
				sale_duration,
				&token_program,
			)
			.map_err(|err| {
				anyhow!(
					"Unable to create `InitialiseWhitelist` instruction: {}",
					err
				)
			})?
		}
		Commands::User(subcommand) => match subcommand {
			UserManagement::Add(fields) => {
				let whitelist = get_whitelist_address(&fields.mint).0;
				let user_ticket = get_user_ticket_address(&fields.user, &whitelist).0;

				println!("User Whitelist Account: {}", user_ticket);

				instructions::add_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_ticket,
				)
				.map_err(|err| anyhow!("Unable to create `AddUser` instruction: {}", err))?
			}
			UserManagement::Remove(fields) => {
				let (whitelist, _) = get_whitelist_address(&fields.mint);
				let (user_ticket, _) = get_user_ticket_address(&fields.user, &whitelist);

				println!("Removing user from whitelist: {}", fields.user);
				println!("Whitelist Account: {}", user_ticket);

				instructions::remove_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_ticket,
				)
				.map_err(|err| anyhow!("Unable to create `RemoveUser` instruction: {}", err))?
			}
		},
		Commands::Buy(fields) => {
			let whitelist = get_whitelist_address(&fields.mint).0;
			let user_ticket = get_user_ticket_address(&wallet_pubkey, &whitelist).0;

			let mint_account = client.get_account(&fields.mint)?;
			let token_program = mint_account.owner;

			let ticket_token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&user_ticket,
					&fields.mint,
					&token_program,
				);

			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist,
				&fields.mint,
				&token_program,
			);

			let user_token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&wallet_pubkey,
					&fields.mint,
					&token_program,
				);

			instructions::buy_tokens(
				&whitelist,
				&vault,
				&fields.mint,
				&wallet_pubkey,
				&user_ticket,
				&ticket_token_account,
				&user_token_account,
				fields.amount,
				&token_program,
			)
			.map_err(|err| anyhow!("Unable to create `BuyTokens` instruction: {}", err))?
		}
		Commands::Deposit(fields) => {
			let whitelist = get_whitelist_address(&fields.mint).0;
			let mint_account = client.get_account(&fields.mint)?;
			let token_program = mint_account.owner;

			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist,
				&fields.mint,
				&token_program,
			);
			let user_token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&wallet_pubkey,
					&fields.mint,
					&token_program,
				);
			instructions::deposit_tokens(
				&whitelist,
				&vault,
				&wallet_pubkey,
				&user_token_account,
				&fields.mint,
				fields.amount,
				&token_program,
			)
			.map_err(|err| anyhow!("Unable to create `DepositTokens` instruction: {}", err))?
		}
		Commands::Withdraw(fields) => {
			let whitelist = get_whitelist_address(&fields.mint).0;
			let mint_account = client.get_account(&fields.mint)?;
			let token_program = mint_account.owner;

			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist,
				&fields.mint,
				&token_program,
			);
			let recipient = match fields.recipient {
				Some(r) => r,
				None => wallet_pubkey,
			};
			let token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&recipient,
					&fields.mint,
					&token_program,
				);
			instructions::withdraw_tokens(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&fields.mint,
				&token_account,
				fields.amount,
				&token_program,
			)
			.map_err(|err| anyhow!("Unable to create `WithdrawTokens` instruction: {}", err))?
		}
		Commands::Burn(method) => match method {
			Method::Single(fields) => {
				let whitelist = get_whitelist_address(&fields.mint).0;
				let user_ticket = get_user_ticket_address(&fields.user, &whitelist).0;

				println!("Removing user from whitelist: {}", fields.user);
				println!("Whitelist Account: {}", user_ticket);

				instructions::remove_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_ticket,
				)
				.map_err(|err| anyhow!("Unable to create `RemoveUser` instruction: {}", err))?
			}
			Method::Bulk { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				let whitelist_account_data = client.get_account_data(&whitelist)?;
				let wl_data = stuk_wl::state::Whitelist::try_from_slice(&whitelist_account_data)?;
				let mint_account = client.get_account(&mint)?;
				let token_program = mint_account.owner;

				let program_accounts = client.get_program_accounts(&stuk_wl::id())?;
				let mut whitelist_accounts = Vec::new();
				// May want to split the returned array into chunks for parallel
				// processing and the reconstruct when done
				for (pubkey, account) in program_accounts.iter() {
					let data = stuk_wl::state::Ticket::try_from_slice(&account.data)?;
					if data.whitelist == whitelist {
						whitelist_accounts.push((pubkey, account, data));
					}
				}
				let treasury_token_account =
					spl_associated_token_account::get_associated_token_address_with_program_id(
						&wl_data.treasury,
						&mint,
						&token_program,
					);

				// Depending on the size of this array we may want to split into
				// threads depending on number of cores on a machine to parallel
				// execute the withdrawals to reduce execution time for now let's
				// just do this single threadedly
				let mut failures = 0;
				let mut failed_accounts: Vec<&Pubkey> =
					Vec::with_capacity(whitelist_accounts.len());
				for (ticket, _ticket_account, _data) in whitelist_accounts {
					// want this to continue on failure
					let ticket_token_account =
						spl_associated_token_account::get_associated_token_address_with_program_id(
							&ticket,
							&mint,
							&token_program,
						);
					let instruction = match instructions::burn_ticket(
						&whitelist,
						&wallet_pubkey,
						&mint,
						&wl_data.treasury,
						&treasury_token_account,
						&ticket,
						&ticket_token_account,
						&token_program,
					) {
						Ok(ix) => ix,
						Err(e) => {
							println!(
								"Unable to create `BurnTicket` instruction for: {}, reason: {}",
								ticket, e
							);
							failures += 1;
							failed_accounts.push(ticket);
							continue;
						}
					};
					let mut transaction =
						Transaction::new_with_payer(&[instruction], Some(&wallet_pubkey));
					let latest_blockhash = match client.get_latest_blockhash() {
						Ok(bh) => bh,
						Err(e) => {
							println!(
								"Unable to get latest blockhash for: {}, reason: {}",
								ticket, e
							);
							failures += 1;
							failed_accounts.push(ticket);
							continue;
						}
					};
					transaction.sign(&[&wallet_keypair], latest_blockhash);
					let txid = match client.send_and_confirm_transaction_with_spinner(&transaction)
					{
						Ok(tx) => tx,
						Err(e) => {
							println!("Unable to send transaction for: {}, reason: {}", ticket, e);
							failures += 1;
							failed_accounts.push(ticket);
							continue;
						}
					};
					println!("Ticket burned: {}", ticket);
					println!("TXID: {}", txid);
				}
				println!("Complete");
				println!("Number of failures: {}", failures);
				println!("Failed accounts: {:?}", failed_accounts);
				std::process::exit(1);
			}
		},
		Commands::Amend(detail) => {
			match detail {
				Detail::Size { mint, size } => {
					let whitelist = get_whitelist_address(&mint).0;
					instructions::amend_whitelist_size(&whitelist, &wallet_pubkey, size).map_err(
						|err| anyhow!("Unable to create `AmendWhitelistSize` instruction: {}", err),
					)?
				}
				Detail::Times {
					mint,
					registration_start_time,
					registration_end_time,
					sale_start_time,
					sale_end_time,
				} => {
					let whitelist = get_whitelist_address(&mint).0;

					let whitelist_account = client.get_account_data(&whitelist)?;
					let wl_data = stuk_wl::state::Whitelist::try_from_slice(&whitelist_account)?;

					let registration_start_timestamp = match registration_start_time {
						Some(time) => Some(string_to_timestamp(time)?),
						None => None,
					};

					let registration_duration = match registration_end_time {
						Some(time) => {
							let ts = string_to_timestamp(time).expect("error parsing time");

							if registration_start_timestamp.is_some_and(|t| t < ts) {
								Some(ts - registration_start_timestamp.unwrap())
							} else {
								return Err(anyhow!("Cannot compute duration, start time is after provided end time"));
							};

							if wl_data.registration_timestamp > 0
								&& wl_data.registration_timestamp < ts
							{
								Some(ts - wl_data.registration_timestamp)
							} else {
								return Err(anyhow!("Cannot compute duration, start time is after provided end time"));
							}
						}
						None => None,
					};

					let sale_start_timestamp = match sale_start_time {
						Some(time) => Some(string_to_timestamp(time)?),
						None => None,
					};

					let sale_duration = match sale_end_time {
						Some(time) => {
							let ts = string_to_timestamp(time).expect("error parsing time");

							if sale_start_timestamp.is_some_and(|t| t < ts) {
								Some(ts - sale_start_timestamp.unwrap())
							} else {
								return Err(anyhow!("Cannot compute duration, start time is after provided end time"));
							};

							if wl_data.sale_timestamp > 0 && wl_data.sale_timestamp < ts {
								Some(ts - wl_data.sale_timestamp)
							} else {
								return Err(anyhow!("Cannot compute duration, start time is after provided end time"));
							}
						}
						None => None,
					};

					instructions::amend_times(
						&whitelist,
						&wallet_pubkey,
						registration_start_timestamp,
						registration_duration,
						sale_start_timestamp,
						sale_duration,
					)
					.map_err(|err| anyhow!("Unable to create `AmendTimes` instruction: {}", err))?
				}
			}
		}
		Commands::Start(start) => match start {
			Start::Registration { mint } => {
				let whitelist = get_whitelist_address(&mint).0;
				instructions::start_registration(&whitelist, &wallet_pubkey).map_err(|err| {
					anyhow!("Unable to create `StartRegistration` instruction: {}", err)
				})?
			}
			Start::Sale { mint } => {
				let whitelist = get_whitelist_address(&mint).0;
				instructions::start_token_sale(&whitelist, &wallet_pubkey).map_err(|err| {
					anyhow!("Unable to create `StartTokenSale` instruction: {}", err)
				})?
			}
		},
		Commands::AllowRegister { allow, mint } => {
			let whitelist = get_whitelist_address(&mint).0;
			let allow_bool = match allow.as_str() {
				"true" | "yes" | "y" => true,
				"false" | "no" | "n" => false,
				_ => return Err(anyhow!("Incorrect value provided")),
			};
			instructions::allow_registration(&whitelist, &wallet_pubkey, allow_bool).map_err(
				|err| anyhow!("Unable to create `AllowRegistration` instruction: {}", err),
			)?
		}
		Commands::Register { mint } => {
			let whitelist = get_whitelist_address(&mint).0;
			let user_ticket = get_user_ticket_address(&wallet_pubkey, &whitelist).0;
			let whitelist_data = client.get_account_data(&whitelist)?;
			let wl_data = stuk_wl::state::Whitelist::try_from_slice(&whitelist_data)?;

			if wl_data.whitelist_size > 0 && {
				let whitelist_accounts = client.get_program_accounts(&whitelist).unwrap();
				let mut accounts = Vec::new();
				for (pubkey, account) in whitelist_accounts {
					if account.data.len() == stuk_wl::state::Ticket::LEN {
						accounts.push(pubkey);
					}
				}
				(wl_data.whitelist_size as usize) < accounts.len()
			} {
				println!("Whitelist full");
				std::process::exit(2);
			}
			println!("Ticket: {}", user_ticket);

			instructions::register(&whitelist, &wallet_pubkey, &user_ticket)
				.map_err(|err| anyhow!("Unable to create `Register` instruction: {}", err))?
		}
		Commands::Unregister { mint } => {
			let whitelist = get_whitelist_address(&mint).0;
			let user_ticket = get_user_ticket_address(&wallet_pubkey, &whitelist).0;

			let mint_account = client.get_account(&mint)?;
			let token_program = mint_account.owner;

			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&whitelist,
				&mint,
				&token_program,
			);
			let ticket_token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&user_ticket,
					&mint,
					&token_program,
				);

			let data = client.get_account_data(&whitelist).unwrap().clone();
			let unpacked_data = stuk_wl::state::Whitelist::try_from_slice(&data[..])?;
			let authority = unpacked_data.authority;

			instructions::unregister(
				&whitelist,
				&authority,
				&vault,
				&mint,
				&wallet_pubkey,
				&user_ticket,
				&ticket_token_account,
				&token_program,
			)
			.map_err(|err| anyhow!("Unable to create `Unregister` instruction: {}", err))?
		}
		Commands::Close { mint, recipient } => {
			let whitelist = get_whitelist_address(&mint).0;
			let mint_account = client.get_account(&mint)?;
			let token_program = mint_account.owner;
			let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
				&mint,
				&whitelist,
				&token_program,
			);
			let recipient = match recipient {
				Some(r) => r,
				None => wallet_pubkey,
			};
			let token_account =
				spl_associated_token_account::get_associated_token_address_with_program_id(
					&mint,
					&recipient,
					&token_program,
				);

			instructions::terminate_whitelist(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&mint,
				&recipient,
				&token_account,
				&token_program,
			)
			.map_err(|err| anyhow!("Unable to create `TerminateWhitelist` instruction: {}", err))?
		}
		Commands::Info(info) => match info {
			Info::Whitelist { mint } => {
				let whitelist = get_whitelist_address(&mint).0;

				let mint_decimals = {
					let mint_account = client.get_account_data(&mint)?;
					let mint_data = spl_token_2022::extension::StateWithExtensions::<Mint>::unpack(
						&mint_account,
					)?;
					mint_data.base.decimals
				};

				let data = client.get_account_data(&whitelist).unwrap().clone();
				let d = stuk_wl::state::Whitelist::try_from_slice(&data)?;

				let buy_limit = spl_token_2022::amount_to_ui_amount(d.buy_limit, mint_decimals);
				let deposited = spl_token_2022::amount_to_ui_amount(d.deposited, mint_decimals);

				println!("Whitelist address: {}", whitelist);
				println!("Authority address: {}", d.authority);
				println!("Vault address: {}", d.vault);
				println!("Mint address: {}", d.mint);
				println!("Price per token: {}", d.token_price);
				println!("Limit per ticket: {}", buy_limit);
				println!("Deposited amount: {}", deposited);
				println!("Registration?: {}", d.allow_registration);
				println!("Registration start time: {:?}", d.registration_timestamp);
				println!("Registration duration: {:?}", d.registration_duration);
				println!("Sale start time: {:?}", d.sale_timestamp);
				println!("Sale duration: {:?}", d.sale_duration);

				std::process::exit(1);
			}
			Info::User { mint, user } => {
				let mint_decimals = {
					let mint_account = client.get_account_data(&mint)?;
					let mint_data = spl_token_2022::extension::StateWithExtensions::<Mint>::unpack(
						&mint_account,
					)?;
					mint_data.base.decimals
				};
				let whitelist = get_whitelist_address(&mint).0;
				let ticket = get_user_ticket_address(&user, &whitelist).0;
				let ticket_ata =
					spl_associated_token_account::get_associated_token_address(&ticket, &mint);

				let data = client.get_account_data(&ticket).unwrap().clone();
				let d = stuk_wl::state::Ticket::try_from_slice(&data)?;

				let allowance = spl_token_2022::amount_to_ui_amount(d.allowance, mint_decimals);
				let amount_bought =
					spl_token_2022::amount_to_ui_amount(d.amount_bought, mint_decimals);
				println!("Ticket address: {}", ticket);
				println!("Ticket ata address: {}", ticket_ata);
				println!("Ticket owner: {}", d.owner);
				println!("Ticket payer: {}", d.payer);
				println!("Ticket allowance: {}", allowance);
				println!("Amount purchased: {}", amount_bought);

				std::process::exit(1);
			}
		},
	};

	let mut transaction = Transaction::new_with_payer(&[instruction], Some(&wallet_pubkey));
	let latest_blockhash = client
		.get_latest_blockhash()
		.map_err(|err| anyhow!("Unable to get latest blockhash: {}", err))?;
	transaction.sign(&[&wallet_keypair], latest_blockhash);
	let txid = client
		.send_and_confirm_transaction_with_spinner(&transaction)
		.map_err(|err| anyhow!("Unable to send transaction: {}", err))?;
	println!("TXID: {}", txid);
	Ok(())
}

fn string_to_timestamp(date_string: String) -> Result<i64, chrono::ParseError> {
	let datetime = NaiveDateTime::parse_from_str(date_string.as_str(), "%Y-%m-%s %H:%M:%S")?;
	Ok(datetime.and_utc().timestamp())
}

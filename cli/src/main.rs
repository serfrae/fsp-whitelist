use {
	anyhow::{anyhow, Result},
	borsh::BorshDeserialize,
	chrono::NaiveDateTime,
	clap::{command, Args, Parser, Subcommand},
	solana_cli_config,
	solana_client::rpc_client::RpcClient,
	solana_program::{instruction::Instruction, pubkey::Pubkey, system_instruction},
	solana_sdk::{
		account::Account,
		commitment_config::CommitmentConfig,
		signature::{read_keypair_file, Signer},
		transaction::Transaction,
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
	Init(Init),
	#[command(subcommand)]
	User(UserManagement),
	#[command(subcommand)]
	Token(Token),
	#[command(subcommand)]
	Amend(Detail),
	#[command(subcommand)]
	Start(Start),
	#[command(subcommand)]
	Register(Registration),
	Close {
		mint: Pubkey,
		recipient: Option<Pubkey>,
	},
	#[command(subcommand)]
	Info(Info),
}

#[derive(Subcommand, Debug)]
enum UserManagement {
	Add(UserManagementCommonFields),
	Remove(UserManagementCommonFields),
}

#[derive(Args, Debug)]
struct UserManagementCommonFields {
	mint: Pubkey,
	user: Pubkey,
}

#[derive(Subcommand, Debug)]
enum Token {
	Buy(TokenFields),
	Deposit(TokenFields),
	#[command(subcommand)]
	Withdraw(TokenType),
}

#[derive(Subcommand, Debug)]
enum TokenType {
	#[command(subcommand)]
	Token(Source),
	#[command(subcommand)]
	Sol(Source),
}

#[derive(Subcommand, Debug)]
enum Source {
	Vault(TokenFields),
	#[command(subcommand)]
	Ticket(Method),
}

#[derive(Subcommand, Debug)]
enum Method {
	Single(TicketFields),
	Bulk { mint: Pubkey },
}

#[derive(Subcommand, Debug)]
enum Detail {
	Times {
		mint: Pubkey,
		registration_start_time: Option<String>,
		registration_end_time: Option<String>,
		sale_start_time: Option<String>,
		sale_end_time: Option<String>,
	},
	Size {
		mint: Pubkey,
		size: Option<u64>,
	},
}

#[derive(Subcommand, Debug)]
enum Start {
	Registration { mint: Pubkey },
	Sale { mint: Pubkey },
}

#[derive(Subcommand, Debug)]
enum Registration {
	Allow { allow: bool, mint: Pubkey },
	Register { mint: Pubkey },
	Unregister { mint: Pubkey },
}

#[derive(Subcommand, Debug)]
enum Info {
	Whitelist { mint: Pubkey },
	User { mint: Pubkey, user: Pubkey },
}

#[derive(Args, Debug)]
struct TokenFields {
	mint: Pubkey,
	recipient: Option<Pubkey>,
	amount: u64,
}
#[derive(Args, Clone, Debug)]
struct TicketFields {
	mint: Pubkey,
	user: Pubkey,
}

#[derive(Args, Debug)]
struct Init {
	mint: Pubkey,
	treasury: Pubkey,
	price: u64,
	buy_limit: u64,
	whitelist_size: Option<u64>,
	allow_registration: bool,
	registration_start_time: Option<String>,
	registration_end_time: Option<String>,
	sale_start_time: Option<String>,
	sale_end_time: Option<String>,
}

fn main() -> Result<()> {
	let args = Cli::parse();

	let solana_config_file = if let Some(ref config) = *solana_cli_config::CONFIG_FILE {
		solana_cli_config::Config::load(config).unwrap_or_default()
	} else {
		solana_cli_config::Config::default()
	};

	let wallet_keypair = read_keypair_file(&solana_config_file.keypair_path)
		.map_err(|err| anyhow!("Unable to read keypair file: {}", err))?;
	let wallet_pubkey = wallet_keypair.pubkey();

	let client = RpcClient::new_with_commitment(
		solana_config_file.json_rpc_url.to_string(),
		CommitmentConfig::confirmed(),
	);

	let instruction: Instruction = match args.cmd {
		Commands::Init(fields) => {
			let (whitelist, _) = get_whitelist_address(&fields.mint);
			let vault = spl_associated_token_account::get_associated_token_address(
				&whitelist,
				&fields.mint,
			);
			let registration_start_timestamp = match fields.registration_start_time {
				Some(time) => Some(string_to_timestamp(time)?),
				None => None,
			};
			let registration_end_timestamp = match fields.registration_end_time {
				Some(time) => Some(string_to_timestamp(time)?),
				None => None,
			};
			let sale_start_timestamp = match fields.sale_start_time {
				Some(time) => Some(string_to_timestamp(time)?),
				None => None,
			};
			let sale_end_timestamp = match fields.sale_end_time {
				Some(time) => Some(string_to_timestamp(time)?),
				None => None,
			};

			println!("Whitelist Account: {}", whitelist);
			println!("Vault Account: {}", vault);

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
				registration_end_timestamp,
				sale_start_timestamp,
				sale_end_timestamp,
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
				let (whitelist, _) = get_whitelist_address(&fields.mint);
				let (user_ticket, _) = get_user_ticket_address(&fields.user, &whitelist);

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
		Commands::Token(subcmd) => match subcmd {
			Token::Buy(fields) => {
				let (whitelist, _) = get_whitelist_address(&fields.mint);
				let (user_ticket, _) = get_user_ticket_address(&wallet_pubkey, &whitelist);

				let ticket_token_account =
					spl_associated_token_account::get_associated_token_address(
						&user_ticket,
						&fields.mint,
					);

				let vault = spl_associated_token_account::get_associated_token_address(
					&whitelist,
					&fields.mint,
				);

				let user_token_account = spl_associated_token_account::get_associated_token_address(
					&wallet_pubkey,
					&fields.mint,
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
				)
				.map_err(|err| anyhow!("Unable to create `BuyTokens` instruction: {}", err))?
			}
			Token::Deposit(fields) => {
				let (whitelist, _) = get_whitelist_address(&fields.mint);
				let vault = spl_associated_token_account::get_associated_token_address(
					&whitelist,
					&fields.mint,
				);
				let user_token_account = spl_associated_token_account::get_associated_token_address(
					&wallet_pubkey,
					&fields.mint,
				);
				instructions::deposit_tokens(
					&whitelist,
					&vault,
					&wallet_pubkey,
					&user_token_account,
					&fields.mint,
					fields.amount,
				)
				.map_err(|err| anyhow!("Unable to create `DepositTokens` instruction: {}", err))?
			}
			Token::Withdraw(token_type) => match token_type {
				TokenType::Token(source) => match source {
					Source::Vault(fields) => {
						let (whitelist, _) = get_whitelist_address(&fields.mint);

						let vault = spl_associated_token_account::get_associated_token_address(
							&whitelist,
							&fields.mint,
						);
						let recipient = match fields.recipient {
							Some(r) => r,
							None => wallet_pubkey,
						};
						let token_account =
							spl_associated_token_account::get_associated_token_address(
								&recipient,
								&fields.mint,
							);
						instructions::withdraw_tokens(
							&whitelist,
							&wallet_pubkey,
							&vault,
							&fields.mint,
							&token_account,
							fields.amount,
						)
						.map_err(|err| {
							anyhow!("Unable to create `WithdrawTokens` instruction: {}", err)
						})?
					}
					Source::Ticket(method) => match method {
						Method::Single(fields) => {
							let (whitelist, _) = get_whitelist_address(&fields.mint);
							let (ticket, _) = get_user_ticket_address(&fields.user, &whitelist);
							unimplemented!();
						}
						Method::Bulk { mint } => unimplemented!(),
					},
				},
				TokenType::Sol(source) => match source {
					Source::Vault(fields) => {
						let (whitelist, _) = get_whitelist_address(&fields.mint);
						let recipient = match fields.recipient {
							Some(r) => r,
							None => wallet_pubkey,
						};
						instructions::withdraw_sol(
							&whitelist,
							&wallet_pubkey,
							&recipient,
							fields.amount,
						)
						.map_err(|err| {
							anyhow!("Unable to create `WithdrawSol` instruction: {}", err)
						})?
					}
					Source::Ticket(method) => match method {
						Method::Single(fields) => unimplemented!(),
						Method::Bulk { mint } => {
							let (whitelist, _) = get_whitelist_address(&mint);
							let whitelist_account_data = client.get_account_data(&whitelist)?;
							let wl_data =
								stuk_wl::state::Whitelist::try_from_slice(&whitelist_account_data)?;

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
							// Depending on the size of this array we may want to split into
							// threads depending on number of cores on a machine to parallel
							// execute the withdrawals to reduce execution time for now let's
							// just do this single threadedly
							for (pubkey, account, data) in whitelist_accounts {
								unimplemented!();
							}
							std::process::exit(1);
						}
					},
				},
			},
		},
		Commands::Amend(detail) => match detail {
			Detail::Size { mint, size } => {
				let (whitelist, _) = get_whitelist_address(&mint);
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
				let (whitelist, _) = get_whitelist_address(&mint);

				let registration_start_timestamp = match registration_start_time {
					Some(time) => Some(string_to_timestamp(time)?),
					None => None,
				};
				let registration_end_timestamp = match registration_end_time {
					Some(time) => Some(string_to_timestamp(time)?),
					None => None,
				};
				let sale_start_timestamp = match sale_start_time {
					Some(time) => Some(string_to_timestamp(time)?),
					None => None,
				};
				let sale_end_timestamp = match sale_end_time {
					Some(time) => Some(string_to_timestamp(time)?),
					None => None,
				};

				instructions::amend_times(
					&whitelist,
					&wallet_pubkey,
					registration_start_timestamp,
					registration_end_timestamp,
					sale_start_timestamp,
					sale_end_timestamp,
				)
				.map_err(|err| anyhow!("Unable to create `AmendTimes` instruction: {}", err))?
			}
		},
		Commands::Start(start) => match start {
			Start::Registration { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				instructions::start_registration(&whitelist, &wallet_pubkey).map_err(|err| {
					anyhow!("Unable to create `StartRegistration` instruction: {}", err)
				})?
			}
			Start::Sale { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				instructions::start_token_sale(&whitelist, &wallet_pubkey).map_err(|err| {
					anyhow!("Unable to create `StartTokenSale` instruction: {}", err)
				})?
			}
		},
		Commands::Register(reg) => match reg {
			Registration::Allow { allow, mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				instructions::allow_registration(&whitelist, &wallet_pubkey, allow).map_err(
					|err| anyhow!("Unable to create `AllowRegistration` instruction: {}", err),
				)?
			}
			Registration::Register { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				let (user_ticket, _) = get_user_ticket_address(&wallet_pubkey, &whitelist);

				instructions::register(&whitelist, &wallet_pubkey, &user_ticket)
					.map_err(|err| anyhow!("Unable to create `Register` instruction: {}", err))?
			}
			Registration::Unregister { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				let (user_ticket, _) = get_user_ticket_address(&wallet_pubkey, &whitelist);

				let vault =
					spl_associated_token_account::get_associated_token_address(&whitelist, &mint);
				let ticket_token_account =
					spl_associated_token_account::get_associated_token_address(&user_ticket, &mint);

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
				)
				.map_err(|err| anyhow!("Unable to create `Unregister` instruction: {}", err))?
			}
		},
		Commands::Close { mint, recipient } => {
			let (whitelist, _) = get_whitelist_address(&mint);
			let vault =
				spl_associated_token_account::get_associated_token_address(&mint, &whitelist);
			let recipient = match recipient {
				Some(r) => r,
				None => wallet_pubkey,
			};
			let token_account =
				spl_associated_token_account::get_associated_token_address(&mint, &recipient);

			instructions::terminate_whitelist(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&mint,
				&recipient,
				&token_account,
			)
			.map_err(|err| anyhow!("Unable to create `TerminateWhitelist` instruction: {}", err))?
		}
		Commands::Info(info) => match info {
			Info::Whitelist { mint } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				let data = client.get_account_data(&whitelist).unwrap().clone();
				let d = stuk_wl::state::Whitelist::try_from_slice(&data)?;
				println!("Whitelist address: {}", whitelist);
				println!("Authority address: {}", d.authority);
				println!("Vault address: {}", d.vault);
				println!("Mint address: {}", d.mint);
				println!("Price per token: {}", d.token_price);
				println!("Limit per ticket: {}", d.buy_limit);
				println!("Deposited amount: {}", d.deposited);
				println!("Registration?: {}", d.allow_registration);
				println!(
					"Registration start time: {:?}",
					d.registration_start_timestamp
				);
				println!("Registration duration: {:?}", d.registration_duration);
				println!("Sale start time: {:?}", d.sale_start_timestamp);
				println!("Sale duration: {:?}", d.sale_duration);

				std::process::exit(1);
			}
			Info::User { mint, user } => {
				let (whitelist, _) = get_whitelist_address(&mint);
				let (ticket, _) = get_user_ticket_address(&user, &whitelist);
				let data = client.get_account_data(&ticket).unwrap().clone();
				let d = stuk_wl::state::Ticket::try_from_slice(&data)?;
				println!("Ticket owner: {}", d.owner);
				println!("Ticket payer: {}", d.payer);
				println!("Ticket allowance: {}", d.allowance);
				println!("Amount purchased: {}", d.amount_bought);

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

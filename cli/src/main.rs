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
	Register(Registration),
	Close {
		mint: Pubkey,
		recipient: Option<Pubkey>,
	},
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
	Token(TokenFields),
	Sol(TokenFields),
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
enum Registration {
	Allow { allow: bool, mint: Pubkey },
	Register { mint: Pubkey },
	Unregister { mint: Pubkey },
}

#[derive(Args, Debug)]
struct TokenFields {
	whitelist: Pubkey,
	mint: Option<Pubkey>,
	recipient: Option<Pubkey>,
	amount: u64,
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
				let mint = match fields.mint {
					Some(mint) => mint,
					None => return Err(anyhow!("Please provide the token mint pubkey")),
				};
				let (user_ticket, _) =
					get_user_ticket_address(&wallet_pubkey, &fields.whitelist);
				let vault = spl_associated_token_account::get_associated_token_address(
					&mint,
					&fields.whitelist,
				);
				let user_token_account = spl_associated_token_account::get_associated_token_address(
					&mint,
					&wallet_pubkey,
				);
				instructions::buy_tokens(
					&fields.whitelist,
					&vault,
					&mint,
					&wallet_pubkey,
					&user_ticket,
					&user_token_account,
					fields.amount,
				)
				.map_err(|err| anyhow!("Unable to create `BuyTokens` instruction: {}", err))?
			}
			Token::Deposit(fields) => {
				let mint = match fields.mint {
					Some(mint) => mint,
					None => return Err(anyhow!("Please provide the token mint pubkey")),
				};
				let vault = spl_associated_token_account::get_associated_token_address(
					&mint,
					&fields.whitelist,
				);
				let user_token_account = spl_associated_token_account::get_associated_token_address(
					&mint,
					&wallet_pubkey,
				);
				instructions::deposit_tokens(
					&fields.whitelist,
					&vault,
					&wallet_pubkey,
					&user_token_account,
					&mint,
					fields.amount,
				)
				.map_err(|err| anyhow!("Unable to create `DepositTokens` instruction: {}", err))?
			}
			Token::Withdraw(token_type) => match token_type {
				TokenType::Token(fields) => {
					let mint = match fields.mint {
						Some(mint) => mint,
						None => return Err(anyhow!("Please provide the token mint pubkey")),
					};
					let vault = spl_associated_token_account::get_associated_token_address(
						&mint,
						&fields.whitelist,
					);
					let recipient = match fields.recipient {
						Some(r) => r,
						None => wallet_pubkey,
					};
					let token_account = spl_associated_token_account::get_associated_token_address(
						&mint, &recipient,
					);
					instructions::withdraw_tokens(
						&fields.whitelist,
						&wallet_pubkey,
						&vault,
						&mint,
						&token_account,
						fields.amount,
					)
					.map_err(|err| {
						anyhow!("Unable to create `WithdrawTokens` instruction: {}", err)
					})?
				}
				TokenType::Sol(fields) => {
					let recipient = match fields.recipient {
						Some(r) => r,
						None => wallet_pubkey,
					};
					instructions::withdraw_sol(
						&fields.whitelist,
						&wallet_pubkey,
						&recipient,
						fields.amount,
					)
					.map_err(|err| anyhow!("Unable to create `WithdrawSol` instruction: {}", err))?
				}
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

				let data = client.get_account_data(&whitelist).unwrap().clone();
				let unpacked_data = stuk_wl::state::WhitelistState::try_from_slice(&data[..])?;
				let authority = unpacked_data.authority;

				instructions::unregister(
					&whitelist,
					&authority,
					&wallet_pubkey,
					&user_ticket,
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
	};

	let mut transaction = Transaction::new_with_payer(&[instruction], Some(&wallet_pubkey));
	let latest_blockhash = client
		.get_latest_blockhash()
		.map_err(|err| anyhow!("Unable to get latest blockhash: {}", err))?;
	transaction.sign(&[&wallet_keypair], latest_blockhash);
	client
		.send_and_confirm_transaction_with_spinner(&transaction)
		.map_err(|err| anyhow!("Unable to send transaction: {}", err))?;
	Ok(())
}

fn string_to_timestamp(date_string: String) -> Result<i64, chrono::ParseError> {
	let datetime = NaiveDateTime::parse_from_str(date_string.as_str(), "%Y-%m-%s %H:%M:%S")?;
	Ok(datetime.and_utc().timestamp())
}

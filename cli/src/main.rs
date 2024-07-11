use {
	anyhow::{anyhow, Result},
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
	stuk_wl::{get_user_whitelist_address, get_whitelist_address, instructions},
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
	Close {
		mint: Pubkey,
	},
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

#[derive(Args, Debug)]
struct TokenFields {
	whitelist: Pubkey,
	mint: Option<Pubkey>,
	amount: u64,
}

#[derive(Subcommand, Debug)]
enum UserManagement {
	Add(UserCommonFields),
	Remove(UserCommonFields),
	Terminate(UserCommonFields),
}

#[derive(Args, Debug)]
struct UserCommonFields {
	mint: Pubkey,
	user: Pubkey,
}

#[derive(Args, Debug)]
struct Init {
	mint: Pubkey,
	price: u64,
	whitelist_size: u64,
	purchase_limit: u64,
	sale_start_time: String,
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
			let (whitelist, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
			let vault = spl_associated_token_account::get_associated_token_address(
				&whitelist,
				&fields.mint,
			);
			let sale_start_time = string_to_timestamp(fields.sale_start_time)?;

			println!("Whitelist Account: {}", whitelist);
			println!("Vault Account: {}", vault);

			instructions::init_whitelist(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&fields.mint,
				fields.price,
				fields.whitelist_size,
				fields.purchase_limit,
				sale_start_time,
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
				let (whitelist, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
				let (user_whitelist, _) = get_user_whitelist_address(&fields.user, &whitelist);

				println!("User Whitelist Account: {}", user_whitelist);

				instructions::add_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_whitelist,
				)
				.map_err(|err| anyhow!("Unable to create `AddUser` instruction: {}", err))?
			}
			UserManagement::Remove(fields) => {
				let (whitelist, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
				let (user_whitelist, _) = get_user_whitelist_address(&fields.user, &whitelist);

				println!("Removing user from whitelist: {}", fields.user);
				println!("Whitelist Account: {}", user_whitelist);

				instructions::remove_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_whitelist,
				)
				.map_err(|err| anyhow!("Unable to create `RemoveUser` instruction: {}", err))?
			}
			UserManagement::Terminate(fields) => {
				let (whitelist, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
				let (user_whitelist, _) = get_user_whitelist_address(&fields.user, &whitelist);

				instructions::terminate_user(
					&whitelist,
					&wallet_pubkey,
					&fields.mint,
					&fields.user,
					&user_whitelist,
				)
				.map_err(|err| anyhow!("Unable to create `TerminateUser` instruction: {}", err))?
			}
		},
		Commands::Token(subcmd) => match subcmd {
			Token::Buy(fields) => {
				let mint = match fields.mint {
					Some(mint) => mint,
					None => return Err(anyhow!("Please provide the token mint pubkey")),
				};
				let (user_whitelist, _) =
					get_user_whitelist_address(&wallet_pubkey, &fields.whitelist);
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
					&user_whitelist,
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
					let token_account = spl_associated_token_account::get_associated_token_address(
						&mint,
						&wallet_pubkey,
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
					instructions::withdraw_sol(&fields.whitelist, &wallet_pubkey, fields.amount)
						.map_err(|err| {
							anyhow!("Unable to create `WithdrawSol` instruction: {}", err)
						})?
				}
			},
		},
		Commands::Close { mint } => {
			let (whitelist, _) = get_whitelist_address(&mint, &wallet_pubkey);
			let vault =
				spl_associated_token_account::get_associated_token_address(&mint, &whitelist);
			let token_account =
				spl_associated_token_account::get_associated_token_address(&mint, &wallet_pubkey);

			instructions::terminate_whitelist(
				&whitelist,
				&wallet_pubkey,
				&vault,
				&mint,
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
	let datetime = NaiveDateTime::parse_from_string(date_string, "%Y-%m-%s %H:%M:%S")?;
	Ok(datetime.timestamp)
}

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
	stuk_wl::{get_whitelist_address, instructions},
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
	Buy,
	Deposit,
	Withdraw,
	Close,
}

#[derive(Subcommand, Debug)]
enum UserManagement {
	Add(UserCommonFields),
	Remove(UserCommonFields),
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

	let client = RpcClient::new_with_commmitment(
		solana_config_file.json_rpc_url.to_string(),
		CommitmentConfig::confirmed(),
	);

	let instruction: Instruction = match args.cmd {
		Commands::Init(fields) => {
			let (whitelist_addr, _) = get_whitelist_address(&wallet_pubkey, &mint);
			let vault_addr = spl_associated_token_account::get_associated_token_address(
				whitelist_addr,
				&fields.mint,
			);
			let sale_start_time = string_to_timestamp(sale_start_time)?;

			println!("Whitelist Account: {}", whitelist_addr);
			println!("Vault Account: {}", vault_addr);

			instructions::init_whitelist(
				whitelist_addr,
				&wallet_pubkey,
				&vault_addr,
				&mint,
				price,
				whitelist_size,
				purchase_limit,
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
				let (whitelist_addr, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
				let (user_wl_addr, _) = get_user_whitelist_address(&fields.user, whitelist_addr);

				println!("User Whitelist Account: {}", user_wl_addr);

				instructions::add_user(
					whitelist_addr,
					&wallet_pubkey,
					&fields.mint,
					user,
					user_wl_addr,
				)
				.map_err(|err| anyhow!("Unable to create `AddUser` instruction: {}", err))?
			}
			UserManagement::Remove(fields) => {
				let (whitelist_addr, _) = get_whitelist_address(&wallet_pubkey, &fields.mint);
				let (user_wl_addr, _) = get_user_whitelist_address(&fields.user, whitelist_addr);

				println!("User Whitelist Account: {}", user_wl_addr);

				instructions::remove_user(
					whitelist_addr,
					&wallet_pubkey,
					&fields.mint,
					user,
					user_wl_addr,
				)
				.map_err(|err| anyhow!("Unable to create `RemoveUser` instruction: {}", err))?
			}
		},
		Commands::Buy => unimplemented!(),
		Commands::Deposit => unimplemented!(),
		Commands::Withdraw => unimplemented!(),
		Commands::Close => unimplemented!(),
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

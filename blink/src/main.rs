use {
	anyhow:: Result,
	blink::server::Server,
	clap::{command, Parser},
    solana_sdk::pubkey::Pubkey,
};

#[derive(Parser, Debug)]
#[command(
	name = "Superteam UK Whitelist Blink",
	about = "A solana action/blink for the Superteam UK Whitelist-Gated Token Sale"
)]
struct Cli {
	/// Address of the token for sale
	mint: Pubkey,
	/// RPC url values: t/testnet, d/devnet, m/mainnet, l/local, or a custom RPC
	#[arg(short, long)]
	url: Option<String>,
	/// Path to a solana config file - must be a full path
	#[arg(short, long)]
	config: Option<String>,
	/// The exposed port, default: :8080
	#[arg(short, long)]
	port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Cli::parse();

	let mint = args.mint;
	let solana_config_file = match args.config {
		Some(path) => solana_cli_config::Config::load(&path).unwrap_or_default(),
		None => {
			if let Some(ref config) = *solana_cli_config::CONFIG_FILE {
				solana_cli_config::Config::load(config).unwrap_or_default()
			} else {
				solana_cli_config::Config::default()
			}
		}
	};

	let url = match args.url {
		Some(id) => match id.as_str() {
			"t" | "testnet" => "https://api.testnet.solana.com".to_string(),
			"d" | "devnet" => "https://api.devnet.solana.com".to_string(),
			"m" | "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
			"l" | "local" => "http://localhost:8899".to_string(),
			_ => id,
		},
		None => solana_config_file.json_rpc_url,
	};

	let port = args.port.unwrap_or(8080);
	let server = Server::new(mint, url, port).await;
	server.run().await?;

    Ok(())
}

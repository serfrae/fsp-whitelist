use {
	anyhow::{anyhow, Result},
	axum::{
		extract::{Json, Query, State},
		http::{
			header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE},
			Method, StatusCode,
		},
		response::IntoResponse,
		routing::{get, post},
		Router,
	},
	base64::{engine::general_purpose::STANDARD, Engine},
	bincode::serialize,
	clap::Parser,
	serde::{Deserialize, Serialize},
	serde_json::{json, Value},
	solana_client::rpc_client::RpcClient,
	solana_sdk::{
		commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
		transaction::Transaction,
	},
	std::str::FromStr,
	std::sync::Arc,
	stuk_wl::instructions,
	tokio::net::TcpListener,
	tower_http::cors::{Any, CorsLayer},
};

struct AppState {
	mint: Pubkey,
	rpc_client: RpcClient,
}

#[derive(Parser, Debug)]
struct Cli {
	mint: Pubkey,
	url: Option<String>,
	config: Option<String>,
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

	let rpc_client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

	let state = Arc::new(AppState { mint, rpc_client });

	let cors = CorsLayer::new()
		.allow_methods([Method::GET, Method::POST, Method::OPTIONS])
		.allow_headers([
			CONTENT_TYPE,
			AUTHORIZATION,
			CONTENT_ENCODING,
			ACCEPT_ENCODING,
		])
		.allow_origin(Any);

	let app = Router::new()
		.route("/actions.json", get(get_request_actions_json))
		.route("/api/actions/buy-token", get(buy_get_request_handler))
		.route("/api/actions/buy-token", post(buy_post_request_handler))
		.route("/api/actions/register", get(reg_get_request_handler))
		.route("/api/actions/register", post(reg_post_request_handler))
		.layer(cors)
		.with_state(state);

	let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
	axum::serve(listener, app)
		.await
		.map_err(|e| anyhow!("Could not start webserver: {}", e))
}

async fn get_request_actions_json() -> impl IntoResponse {
	Json(json!({
		"rules": [
			{
				"pathPattern": "/*",
				"apiPath": "/api/actions/*",
			},
			{
				"pathPattern": "/api/actions/**",
				"apiPath": "/api/actions/**",
			},
		],
	}))
}

#[derive(Serialize)]
struct ActionGetResponse {
	title: String,
	icon: String,
	description: String,
	links: Links,
}

#[derive(Serialize)]
struct Links {
	actions: Vec<ActionLink>,
}

#[derive(Serialize)]
struct ActionLink {
	label: String,
	href: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	parameters: Option<Vec<Parameter>>,
}

#[derive(Serialize)]
struct Parameter {
	name: String,
	label: String,
	required: bool,
}

async fn buy_get_request_handler() -> impl IntoResponse {
	let base_href = "/api/actions/buy-token?";
	let response = ActionGetResponse {
		title: "Whitelist - Buy token".into(),
		icon: "".into(),
		description: "Allow purchase of tokens if user is whitelisted".into(),
		links: Links {
			actions: vec![
				ActionLink {
					label: "Buy 1 Token".into(),
					href: format!("{}amount=1", base_href),
					parameters: None,
				},
				ActionLink {
					label: "Buy 10 Tokens".into(),
					href: format!("{}amount=10", base_href),
					parameters: None,
				},
				ActionLink {
					label: "Buy 100 Tokens".into(),
					href: format!("{}amount=100", base_href),
					parameters: None,
				},
			],
		},
	};
	(StatusCode::OK, Json(response))
}

async fn reg_get_request_handler() -> impl IntoResponse {
	let base_href = "/api/actions/register";
	let response = ActionGetResponse {
		title: "Whitelist Register".into(),
		icon: "".into(),
		description: "Register for token presale".into(),
		links: Links {
			actions: vec![ActionLink {
				label: "Register".into(),
				href: base_href.to_string(),
				parameters: None,
			}],
		},
	};
	(StatusCode::OK, Json(response))
}

#[derive(Deserialize)]
struct QueryParams {
	amount: f64,
}

#[derive(Deserialize)]
struct PostRequest {
	account: String,
}

#[derive(Serialize)]
struct PostResponse {
	transaction: String,
	message: String,
}

async fn buy_post_request_handler(
	State(state): State<Arc<AppState>>,
	Query(params): Query<QueryParams>,
	Json(payload): Json<PostRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
	let account = Pubkey::from_str(&payload.account).map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(json!({"error": "Invalid 'account' provided"})),
		)
	})?;

	let latest_blockhash = state.rpc_client.get_latest_blockhash().map_err(|err| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": format!("Failed to get latest blockhash: {}", err)})),
		)
	})?;

	let (whitelist, _) = stuk_wl::get_whitelist_address(&state.mint);
	let mint_account = state.rpc_client.get_account(&state.mint).map_err(|err| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": format!("Failed to get mint account: {}", err)})),
		)
	})?;
	let token_program = mint_account.owner;
	let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
		&whitelist,
		&state.mint,
		&token_program,
	);
	let (ticket, _) = stuk_wl::get_user_ticket_address(&account, &whitelist);
	let ticket_token_account =
		spl_associated_token_account::get_associated_token_address_with_program_id(
			&ticket,
			&state.mint,
			&token_program,
		);
	let user_token_account =
		spl_associated_token_account::get_associated_token_address_with_program_id(
			&account,
			&state.mint,
			&token_program,
		);

	let instruction = instructions::buy_tokens(
		&whitelist,
		&vault,
		&state.mint,
		&account,
		&ticket,
		&ticket_token_account,
		&user_token_account,
		params.amount as u64,
		&token_program,
	)
	.map_err(|err| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": format!("Could not create `BuyToken` instruction: {}", err)})),
		)
	})?;
	let mut transaction = Transaction::new_with_payer(&[instruction], Some(&account));
	transaction.message.recent_blockhash = latest_blockhash;

	let serialized_transaction = serialize(&transaction).map_err(|_| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": "Failed to serialize transaction"})),
		)
	})?;

	Ok(Json(PostResponse {
		transaction: STANDARD.encode(serialized_transaction),
		message: format!("Buying {} tokens", params.amount),
	}))
}

async fn reg_post_request_handler(
	State(state): State<Arc<AppState>>,
	Query(params): Query<QueryParams>,
	Json(payload): Json<PostRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
	let account = Pubkey::from_str(&payload.account).map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(json!({"error": "Invalid 'account' provided"})),
		)
	})?;
	let to_pubkey = Keypair::new().pubkey();
	let rpc_client = RpcClient::new_with_commitment(
		"https://api.devnet.solana.com".to_string(),
		CommitmentConfig::confirmed(),
	);

	let latest_blockhash = rpc_client.get_latest_blockhash().map_err(|err| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": format!("Failed to get latest blockhash: {}", err)})),
		)
	})?;

	let (whitelist, _) = stuk_wl::get_whitelist_address(&state.mint);
	let (ticket, _) = stuk_wl::get_user_ticket_address(&account, &whitelist);

	let instruction = instructions::register(&whitelist, &account, &ticket).map_err(|err| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": format!("Could not create `Register` instruction: {}", err)})),
		)
	})?;
	let mut transaction = Transaction::new_with_payer(&[instruction], Some(&account));
	transaction.message.recent_blockhash = latest_blockhash;

	let serialized_transaction = serialize(&transaction).map_err(|_| {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(json!({"error": "Failed to serialize transaction"})),
		)
	})?;

	Ok(Json(PostResponse {
		transaction: STANDARD.encode(serialized_transaction),
		message: format!("Send {} SOL to {}", params.amount, to_pubkey),
	}))
}

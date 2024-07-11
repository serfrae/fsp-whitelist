use {
	axum::{
		extract::{Json, Query},
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
	serde::{Deserialize, Serialize},
	serde_json::{json, Value},
	solana_client::rpc_client::RpcClient,
	solana_sdk::{
		commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
		transaction::Transaction,
	},
	std::str::FromStr,
	stuk_wl::instructions,
	tokio::net::TcpListener,
	tower_http::cors::{Any, CorsLayer},
};

#[tokio::main]
async fn main() {
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
		.route("/api/actions/buy-token", get(get_request_handler))
		.route("/api/actions/buy-token", post(post_request_handler))
		.layer(cors);

	let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
	axum::serve(listener, app).await.unwrap();
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

async fn get_request_handler() -> impl IntoResponse {
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

async fn post_request_handler(
	Query(params): Query<QueryParams>,
	Json(payload): Json<PostRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
	let account = Pubkey::from_str(&payload.account).map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(json!({"errpr": "Invalid 'account' provided"})),
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

	let instruction = instructions::buy_tokens(
		&Pubkey::new_unique(),
		&Pubkey::new_unique(),
		&Pubkey::new_unique(),
		&Pubkey::new_unique(),
		&Pubkey::new_unique(),
		&Pubkey::new_unique(),
		params.amount as u64,
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
		message: format!("Send {} SOL to {}", params.amount, to_pubkey),
	}))
}

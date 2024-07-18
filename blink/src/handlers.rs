use {
	axum::{
		extract::{Json, Query, State},
		http::StatusCode,
		response::IntoResponse,
	},
	base64::{engine::general_purpose::STANDARD, Engine},
	bincode::serialize,
    crate::{monitor::CounterMessage, server::AppState},
	serde::{Deserialize, Serialize},
	serde_json::{json, Value},
    solana_sdk::{pubkey::Pubkey, transaction::Transaction},
    std::{str::FromStr, sync::Arc},
    stuk_wl::instructions,
};

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

#[derive(Deserialize)]
pub(crate) struct QueryParams {
	amount: f64,
}

#[derive(Deserialize)]
pub(crate) struct PostRequest {
	account: String,
}

#[derive(Serialize)]
struct PostResponse {
	transaction: String,
	message: String,
}

pub(crate) async fn get_request_actions_json(State(state): State<Arc<AppState>>) -> impl IntoResponse {
	tokio::spawn(async move {
		let _ = state.counter_tx.send(CounterMessage::Get).await;
	});
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

pub(crate) async fn reg_get_request_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

	tokio::spawn(async move {
		let _ = state.counter_tx.send(CounterMessage::Get).await;
	});
	(StatusCode::OK, Json(response))
}

pub(crate) async fn buy_get_request_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

	tokio::spawn(async move {
		let _ = state.counter_tx.send(CounterMessage::Get).await;
	});

	(StatusCode::OK, Json(response))
}

pub(crate) async fn buy_post_request_handler(
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
	let (ticket, _) = stuk_wl::get_user_ticket_address(&account, &whitelist);

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

	tokio::spawn(async move {
		let _ = state.counter_tx.send(CounterMessage::Post).await;
	});

	Ok(Json(PostResponse {
		transaction: STANDARD.encode(serialized_transaction),
		message: format!("Buying {} tokens", params.amount),
	}))
}

pub(crate) async fn reg_post_request_handler(
	State(state): State<Arc<AppState>>,
	Query(_params): Query<QueryParams>,
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

	tokio::spawn(async move {
		let _ = state.counter_tx.send(CounterMessage::Post).await;
	});

	Ok(Json(PostResponse {
		transaction: STANDARD.encode(serialized_transaction),
		message: format!("Registered for whitelist"),
	}))
}

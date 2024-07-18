use {
	crate::monitor::*,
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
	serde::{Deserialize, Serialize},
	serde_json::{json, Value},
	solana_client::rpc_client::RpcClient,
	solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, transaction::Transaction},
	std::{str::FromStr, sync::Arc},
	stuk_wl::instructions,
	tokio::{net::TcpListener, sync::mpsc},
	tower_http::cors::{Any, CorsLayer},
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

struct AppState {
	mint: Pubkey,
	rpc_client: RpcClient,
	control_tx: mpsc::Sender<ControlMessage>,
	counter_tx: mpsc::Sender<CounterMessage>,
}

impl AppState {
	pub fn new(
		mint: Pubkey,
		url: String,
		control_tx: mpsc::Sender<ControlMessage>,
		counter_tx: mpsc::Sender<CounterMessage>,
	) -> Self {
		let rpc_client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());
		AppState {
			mint,
			rpc_client,
			control_tx,
			counter_tx,
		}
	}
}

pub struct Server {
	state: Arc<AppState>,
	app: Router,
	listener: TcpListener,
	monitor: Monitor,
}

impl Server {
	pub async fn new(mint: Pubkey, url: String, port: u16) -> Self {
		let (control_tx, control_rx) = mpsc::channel(32);
		let (counter_tx, counter_rx) = mpsc::channel(1024);

		let cors = CorsLayer::new()
			.allow_methods([Method::GET, Method::POST, Method::OPTIONS])
			.allow_headers([
				CONTENT_TYPE,
				AUTHORIZATION,
				CONTENT_ENCODING,
				ACCEPT_ENCODING,
			])
			.allow_origin(Any);

		let state = Arc::new(AppState::new(mint, url, control_tx, counter_tx));

		let app = Router::new()
			.route("/actions.json", get(Self::get_request_actions_json))
			.route("/api/actions/buy-token", get(Self::buy_get_request_handler))
			.route(
				"/api/actions/buy-token",
				post(Self::buy_post_request_handler),
			)
			.route("/api/actions/register", get(Self::reg_get_request_handler))
			.route(
				"/api/actions/register",
				post(Self::reg_post_request_handler),
			)
			.layer(cors)
			.with_state(state.clone());

		let monitor = Monitor::new(control_rx, counter_rx);

		let addr = format!("0.0.0.0:{}", port);
		let listener = TcpListener::bind(&addr).await.unwrap();

		Server {
			state,
			app,
			listener,
			monitor,
		}
	}

	async fn get_request_actions_json(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

	async fn reg_get_request_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

	async fn buy_get_request_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

	async fn reg_post_request_handler(
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

	pub async fn run(mut self) -> Result<()> {
		tokio::spawn(async move { self.monitor.run().await });
		self.state.control_tx.send(ControlMessage::Start).await?;

		axum::serve(self.listener, self.app)
			.await
			.map_err(|e| anyhow!("Could not start server: {}", e))
	}
}

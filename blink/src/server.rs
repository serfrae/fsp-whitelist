use {
	crate::{
		handlers::*,
		monitor::{CounterMessage, Monitor},
	},
	anyhow::{anyhow, Result},
	axum::{
		http::{
			header::{ACCEPT_ENCODING, AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE},
			Method,
		},
		routing::{get, post},
		Router,
	},
	solana_client::rpc_client::RpcClient,
	solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey},
	std::sync::Arc,
	tokio::{net::TcpListener, sync::mpsc},
	tower_http::cors::{Any, CorsLayer},
};

pub(crate) struct AppState {
	pub(crate) mint: Pubkey,
	pub(crate) rpc_client: RpcClient,
	pub(crate) counter_tx: mpsc::Sender<CounterMessage>,
}

impl AppState {
	pub fn new(mint: Pubkey, url: String, counter_tx: mpsc::Sender<CounterMessage>) -> Self {
		let rpc_client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());
		AppState {
			mint,
			rpc_client,
			counter_tx,
		}
	}
}

pub struct Server {
	app: Router,
	listener: TcpListener,
	monitor: Monitor,
}

impl Server {
	pub async fn new(mint: Pubkey, url: String, port: u16) -> Self {
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

		let state = Arc::new(AppState::new(mint, url, counter_tx));

		let app = Router::new()
			.route("/actions.json", get(get_request_actions_json))
			.route("/api/actions/buy-token", get(buy_get_request_handler))
			.route(
				"/api/actions/buy-token",
				post(buy_post_request_handler),
			)
			.route("/api/actions/register", get(reg_get_request_handler))
			.route(
				"/api/actions/register",
				post(reg_post_request_handler),
			)
			.layer(cors)
			.with_state(state);

		let monitor = Monitor::new(counter_rx);

		let addr = format!("0.0.0.0:{}", port);
		let listener = TcpListener::bind(&addr).await.unwrap();

		Server {
			app,
			listener,
			monitor,
		}
	}

	pub async fn run(mut self) -> Result<()> {
		tokio::spawn(async move { self.monitor.run().await });

		axum::serve(self.listener, self.app)
			.await
			.map_err(|e| anyhow!("Could not start server: {}", e))
	}
}

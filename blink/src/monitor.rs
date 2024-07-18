use {
	indicatif::{ProgressBar, ProgressStyle},
	std::time::Instant,
	tokio::{
		sync::mpsc,
		time::{interval, Duration, Interval},
	},
};

pub struct Monitor {
	spinner: ProgressBar,
	start_time: Instant,
	update_interval: Interval,
	get_counter: u64,
	post_counter: u64,
	counter_rx: mpsc::Receiver<CounterMessage>,
}

pub enum CounterMessage {
	Get,
	Post,
}

impl Monitor {
	pub fn new(
		counter_rx: mpsc::Receiver<CounterMessage>,
	) -> Self {
		let spinner = ProgressBar::new_spinner();
		spinner.set_style(
			ProgressStyle::default_spinner()
				.template("{spinner:.green} {msg}")
				.unwrap()
				.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
		);
		Monitor {
			spinner,
			start_time: Instant::now(),
			update_interval: interval(Duration::from_millis(80)),
			get_counter: 0,
			post_counter: 0,
			counter_rx,
		}
	}

	fn update_spinner(&mut self) {
		self.spinner.set_message(self.get_display_string());
	}

	fn get_elapsed_time(&self) -> String {
		let elapsed = self.start_time.elapsed();
		let secs = elapsed.as_secs();
		let mins = secs / 60;
		let hrs = mins / 60;
		format!("{}:{:02}:{:02}", hrs, mins % 60, secs % 60)
	}

	fn get_display_string(&self) -> String {
		let get_text = format!("\x1b[1m{}\x1b[0m requests:", "GET");
		let post_text = format!("\x1b[1m{}\x1b[0m requests:", "POST");
		format!(
			"Server running... | {} | {} {} | {} {}",
			self.get_elapsed_time(),
			get_text,
			self.get_counter,
			post_text,
			self.post_counter
		)
	}

	pub async fn run(&mut self) {
        self.spinner.enable_steady_tick(Duration::from_millis(80));
		loop {
			tokio::select! {
				Some(message) = self.counter_rx.recv() => {
						match message {
							CounterMessage::Get => self.get_counter += 1,
							CounterMessage::Post => self.post_counter += 1,
							}
					self.update_spinner();
				},
				_ = self.update_interval.tick() => {
					self.update_spinner();
				}
			}
		}
	}
}

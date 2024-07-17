use solana_program_test::{ProgramTest, *};
use solana_sdk::pubkey::Pubkey;

pub struct TestAccounts<'a> {
	whitelist: Option<&'a Pubkey>,
	whitelist_2022: Option<&'a Pubkey>,
	mint: Option<&'a Pubkey>,
	mint_2022: Option<&'a Pubkey>,
	vault: Option<&'a Pubkey>,
	vault_2022: Option<&'a Pubkey>,
	_wallet: Option<&'a Pubkey>,
	wallet_token_account: Option<&'a Pubkey>,
	wallet_token_account_2022: Option<&'a Pubkey>,
	ticket_account: Option<&'a Pubkey>,
	ticket_account_2022: Option<&'a Pubkey>,
	ticket_token_account: Option<&'a Pubkey>,
	ticket_token_account_2022: Option<&'a Pubkey>,
}

pub fn program_test(accounts: TestAccounts) -> ProgramTest {
	let mut pc = ProgramTest::new(
		"stuk_wl",
		stuk_wl::id(),
		processor!(stuk_wl::processor::Processor::process),
	);

	pc.prefer_bpf(false);
	pc.add_program(
		"spl_token_2022",
		spl_token_2022::id(),
		processor!(spl_token_2022::processor::Processor::process),
	);

	pc.add_program(
		"spl_token",
		spl_token::id(),
		processor!(spl_token::processor::Processor::process),
	);

	pc.add_program(
		"sok_associated_token_account",
		spl_associated_token_account::id(),
		processor!(spl_associated_token_account::processor::process_instruction),
	);

	if let Some(whitelist) = accounts.whitelist {
		pc.add_account_with_file_data(*whitelist, 203928, stuk_wl::id(), "whitelist.bin")
	}

	if let Some(whitelist_2022) = accounts.whitelist_2022 {
		pc.add_account_with_file_data(*whitelist_2022, 207408, stuk_wl::id(), "whitelist_2022.bin")
	}

	if let Some(mint) = accounts.mint {
		pc.add_account_with_file_data(*mint, 1461600, spl_token::id(), "mint.bin");
	}

	if let Some(mint_2022) = accounts.mint_2022 {
		pc.add_account_with_file_data(*mint_2022, 1461600, spl_token_2022::id(), "mint_2022.bin")
	}

	if let Some(vault) = accounts.vault {
		pc.add_account_with_file_data(*vault, 203928, spl_token::id(), "vault.bin")
	}

	if let Some(vault_2022) = accounts.vault_2022 {
		pc.add_account_with_file_data(*vault_2022, 207408, spl_token_2022::id(), "vault_2022.bin")
	}

	if let Some(wallet_token_account) = accounts.wallet_token_account {
		pc.add_account_with_file_data(
			*wallet_token_account,
			203928,
			spl_token::id(),
			"wallet_token_account.bin",
		)
	}

	if let Some(wallet_token_account_2022) = accounts.wallet_token_account_2022 {
		pc.add_account_with_file_data(
			*wallet_token_account_2022,
			207408,
			spl_token_2022::id(),
			"wallet_token_account_2022.bin",
		)
	}

	if let Some(ticket) = accounts.ticket_account {
		pc.add_account_with_file_data(*ticket, 167736, stuk_wl::id(), "ticket_account.bin")
	}

	if let Some(ticket_2022) = accounts.ticket_account_2022 {
		pc.add_account_with_file_data(
			*ticket_2022,
			167736,
			stuk_wl::id(),
			"ticket_account_2022.bin",
		)
	}

	if let Some(ticket_token_account) = accounts.ticket_token_account {
		pc.add_account_with_file_data(
			*ticket_token_account,
			203928,
			spl_token::id(),
			"ticket_token_account.bin",
		)
	}

	if let Some(ticket_token_account_2022) = accounts.ticket_token_account_2022 {
		pc.add_account_with_file_data(
			*ticket_token_account_2022,
			207408,
			spl_token_2022::id(),
			"ticket_token_account_2022.bin",
		)
	}

	pc
}

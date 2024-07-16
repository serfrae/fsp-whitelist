use solana_program_test::{ProgramTest, *};
use solana_sdk::pubkey::Pubkey;

pub struct TestAccounts<'a> {
	whitelist: Option<&'a Pubkey>,
	mint: Option<&'a Pubkey>,
	treasury: Option<&'a Pubkey>,
	wallet: Option<&'a Pubkey>,
	wallet_token_account: Option<&'a Pubkey>,
	ticket: Option<&'a Pubkey>,
	ticket_token_account: Option<&'a Pubkey>,
	token_program_id: Pubkey,
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

	if accounts.mint.is_some() {
		pc.add_account_with_file_data(
			*accounts.mint.unwrap(),
			1461600,
            spl_token_2022::id(),
			"mint.bin",
		);
	}

    if accounts.wallet_token_account.is_some() {
        pc.add_account_with_file_data(
        *accounts.wallet_token_account.unwrap(),

    }

	pc
}

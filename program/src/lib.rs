pub mod entrypoint;
pub mod error;
pub mod instructions;
pub mod processor;
pub mod state;

use solana_program::{declare_id, pubkey::Pubkey};

const SEED: &[u8; 12] = b"___whitelist";
declare_id!("3jyFQazJomtErMzsHrhNzj18aTJYiq3Xdr3H9J51CUzp");
pub fn get_whitelist_address(mint: &Pubkey) -> (Pubkey, u8) {
	Pubkey::find_program_address(&[SEED, mint.as_ref()], &crate::id())
}

pub fn get_user_ticket_address(user: &Pubkey, whitelist: &Pubkey) -> (Pubkey, u8) {
	Pubkey::find_program_address(&[SEED, user.as_ref(), whitelist.as_ref()], &crate::id())
}

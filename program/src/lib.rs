pub mod entrypoint;
pub mod error;
pub mod instructions;
pub mod processor;
pub mod state;

use solana_program::{declare_id, pubkey::Pubkey};

const SEED: &[u8; 12] = b"___whitelist";
solana_program::declare_id!("GJUfUomLVdepDKDcWfR7nNYTSuWKXCUphfGNZj81uTgu");
pub fn get_whitelist_address(authority: &Pubkey, mint: &Pubkey) -> (&Pubkey, u8) {
	Pubkey::find_program_address(&[SEED, authority.as_ref(), mint.as_ref()], &crate::id())
}

pub fn get_user_whitelist_address(user: &Pubkey, whitelist: &Pubkey) -> (&Pubkey, u8) {
	Pubkey::find_program_address(&[SEED, user.as_ref(), whitelist.as_ref()], &crate::id())
}

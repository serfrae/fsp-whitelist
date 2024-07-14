use {
	num_derive::FromPrimitive,
	num_traits::FromPrimitive as FromPrimitiveTrait,
	solana_program::{
		decode_error::DecodeError,
		msg,
		program_error::{PrintProgramError, ProgramError},
	},
	thiserror::Error,
};

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum WhitelistError {
	#[error("Invalid Instruction")]
	InvalidInstruction,
	#[error("Invalid Whitelist Address")]
	InvalidWhitelistAddress,
	#[error("Signer error")]
	SignerError,
	#[error("Account mismatch")]
	AccountMismatch,
    #[error("Incorrect token program")]
    IncorrectTokenProgram,
	#[error("Whitelist Already Initialized")]
	WhitelistAlreadyInitialized,
	#[error("Whitelist Not Initialized")]
	WhitelistNotInitialized,
	#[error("Incorrect Account Address")]
	IncorrectUserAccount,
	#[error("Incorrect Whitelist Address")]
	IncorrectWhitelistAddress,
	#[error("Incorrect Vault Address")]
	IncorrectVaultAddress,
	#[error("Incorrect Underlying Mint Address")]
	IncorrectMintAddress,
	#[error("Incorrect payer")]
	IncorrectPayer,
	#[error("Sale Has Not Started")]
	SaleNotStarted,
	#[error("Sale has ended")]
	SaleEnded,
	#[error("Registration has not started")]
	RegistrationNotStarted,
	#[error("Registration has finished")]
	RegistrationFinished,
	#[error("Cannot unregister")]
	CannotUnregister,
	#[error("Illegal Mint Owner")]
	IllegalMintOwner,
	#[error("Unauthorised Access")]
	Unauthorised,
	#[error("Insufficient Funds")]
	InsufficientFunds,
	#[error("Vault Is Not Empty")]
	VaultNotEmpty,
	#[error(
		"The `sale_end_timestamp` or `registration_end_timestamp` is before the current timestamp"
	)]
	InvalidTimestamp,
	#[error("Invalid registration start time")]
	InvalidRegistrationStartTime,
	#[error("Invalid Sale Start Time")]
	InvalidSaleStartTime,
	#[error("Sale begins before registration")]
	SaleBeforeRegistration,
	#[error("Registration has started")]
	RegistrationStarted,
	#[error("Token sale has commenced")]
	SaleStarted,
	#[error("Sale is still ongoing")]
	SaleOngoing,
	#[error("Buy limit exceeded")]
	BuyLimitExceeded,
	#[error("Overflow")]
	Overflow,
}

impl From<WhitelistError> for ProgramError {
	fn from(e: WhitelistError) -> Self {
		ProgramError::Custom(e as u32)
	}
}

impl<T> DecodeError<T> for WhitelistError {
	fn type_of() -> &'static str {
		"Whitelist error"
	}
}

impl PrintProgramError for WhitelistError {
	fn print<E>(&self)
	where
		E: 'static + std::error::Error + DecodeError<E> + FromPrimitiveTrait + PrintProgramError,
	{
		msg!(&self.to_string())
	}
}

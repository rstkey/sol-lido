use crate::{error::AnchorError, token::BLamports, ANCHOR_RESERVE_ACCOUNT};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use lido::{
    error::LidoError,
    token::{Rational, StLamports},
    util::serialize_b58,
};
use serde::Serialize;
use solana_program::{
    account_info::AccountInfo, clock::Epoch, entrypoint::ProgramResult, msg, program_pack::Pack,
    pubkey::Pubkey,
};

#[repr(C)]
#[derive(
    Clone, Debug, Default, BorshDeserialize, BorshSerialize, BorshSchema, Eq, PartialEq, Serialize,
)]
pub struct Anchor {
    /// The SPL Token mint address for bSOL.
    #[serde(serialize_with = "serialize_b58")]
    pub bsol_mint: Pubkey,

    /// Reserve authority for `reserve_account`.
    #[serde(serialize_with = "serialize_b58")]
    pub reserve_authority: Pubkey,

    /// The associated LIDO state address.
    #[serde(serialize_with = "serialize_b58")]
    pub lido: Pubkey,

    /// Bump seeds for signing messages on behalf of the authority.
    pub mint_authority_bump_seed: u8,
    pub reserve_authority_bump_seed: u8,
    pub reserve_account_bump_seed: u8,
}

impl Anchor {
    pub fn save(&self, account: &AccountInfo) -> ProgramResult {
        // NOTE: If you ended up here because the tests are failing because the
        // runtime complained that an account's size was modified by a program
        // that wasn't its owner, double check that the name passed to
        // ProgramTest matches the name of the crate.
        BorshSerialize::serialize(self, &mut *account.data.borrow_mut())?;
        Ok(())
    }

    pub fn check_is_b_sol_account(&self, token_account_info: &AccountInfo) -> ProgramResult {
        if token_account_info.owner != &spl_token::id() {
            msg!(
                "Expected SPL token account to be owned by {}, but it's owned by {} instead.",
                spl_token::id(),
                token_account_info.owner
            );
            return Err(AnchorError::InvalidBSolAccountOwner.into());
        }
        let token_account =
            match spl_token::state::Account::unpack_from_slice(&token_account_info.data.borrow()) {
                Ok(account) => account,
                Err(..) => {
                    msg!(
                        "Expected an SPL token account at {}.",
                        token_account_info.key
                    );
                    return Err(AnchorError::InvalidBSolAccount.into());
                }
            };

        if token_account.mint != self.bsol_mint {
            msg!(
                "Expected mint of {} to be our bSOL mint ({}), but found {}.",
                token_account_info.key,
                self.bsol_mint,
                token_account.mint,
            );
            return Err(AnchorError::InvalidBSolMint.into());
        }
        Ok(())
    }

    pub fn check_reserve_account(
        &self,
        program_id: &Pubkey,
        anchor_state: &Pubkey,
        provided_reserve: &Pubkey,
    ) -> ProgramResult {
        let reserve_account = Pubkey::create_program_address(
            &[
                &anchor_state.to_bytes(),
                ANCHOR_RESERVE_ACCOUNT,
                &[self.reserve_account_bump_seed],
            ],
            program_id,
        )?;

        if &reserve_account != provided_reserve {
            msg!(
                "Invalid reserve account, expected {}, but found {}.",
                reserve_account,
                provided_reserve,
            );
            return Err(AnchorError::InvalidReserveAccount.into());
        }
        Ok(())
    }
}

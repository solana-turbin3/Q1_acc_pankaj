use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};

use crate::state::Escrow;

pub fn process_refund_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [maker, mint_a, escrow_account, vault, maker_ata, _system_program @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify maker is signer
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Deserialize escrow state and verify maker
    let escrow_state = Escrow::from_account_info(escrow_account)?;
    if escrow_state.maker().as_ref() != maker.address().as_ref() {
        return Err(ProgramError::IllegalOwner);
    }
    if escrow_state.mint_a().as_ref() != mint_a.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    let bump = escrow_state.bump;

    // Build PDA signer seeds
    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed);

    // Get vault balance in scoped block to drop Ref borrow before CPI
    let vault_balance = {
        let vault_state = pinocchio_token::state::TokenAccount::from_account_view(vault)?;
        vault_state.amount()
    };

    // Transfer all tokens from vault back to maker's ATA
    if vault_balance > 0 {
        pinocchio_token::instructions::Transfer {
            from: vault,
            to: maker_ata,
            authority: escrow_account,
            amount: vault_balance,
        }
        .invoke_signed(&[signer.clone()])?;
    }

    // Close vault token account, send rent to maker
    pinocchio_token::instructions::CloseAccount {
        account: vault,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer.clone()])?;

    // Close escrow account: move lamports to maker, then close
    let escrow_lamports = escrow_account.lamports();
    maker.set_lamports(maker.lamports() + escrow_lamports);
    escrow_account.set_lamports(0);
    escrow_account.close()?;

    Ok(())
}

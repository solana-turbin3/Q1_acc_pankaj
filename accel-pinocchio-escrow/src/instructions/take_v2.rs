use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};

use crate::state::EscrowV2;

pub fn process_take_instruction_v2(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [taker, maker, mint_a, mint_b, escrow_account, vault, taker_ata_a, taker_ata_b, maker_ata_b, _token_program, _system_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify taker is signer
    if !taker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Deserialize escrow state using EscrowV2
    let escrow_state = EscrowV2::from_account_info(escrow_account)?;
    if escrow_state.maker().as_ref() != maker.address().as_ref() {
        return Err(ProgramError::IllegalOwner);
    }
    if escrow_state.mint_a().as_ref() != mint_a.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    if escrow_state.mint_b().as_ref() != mint_b.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount_to_receive = escrow_state.amount_to_receive();
    let amount_to_give = escrow_state.amount_to_give();
    let bump = escrow_state.bump;

    // Build PDA signer seeds for the escrow
    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&seed);

    // Step 1: Transfer amount_to_receive of mint_b from taker → maker
    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
    }
    .invoke()?;

    // Step 2: Transfer amount_to_give of mint_a from vault → taker
    pinocchio_token::instructions::Transfer {
        from: vault,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[signer.clone()])?;

    // Step 3: Close vault token account, send rent to maker
    pinocchio_token::instructions::CloseAccount {
        account: vault,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer.clone()])?;

    // Step 4: Close escrow account
    let escrow_lamports = escrow_account.lamports();
    maker.set_lamports(maker.lamports() + escrow_lamports);
    escrow_account.set_lamports(0);
    escrow_account.close()?;

    Ok(())
}

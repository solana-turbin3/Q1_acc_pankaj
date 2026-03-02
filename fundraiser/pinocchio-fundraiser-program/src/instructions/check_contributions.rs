use pinocchio::{
    cpi::{Seed, Signer},
    entrypoint::InstructionContext,
    error::ProgramError,
    ProgramResult,
};

use crate::{
    error::{err, ERR_FUNDRAISER_NOT_ENDED, ERR_TARGET_NOT_MET},
    raw_cpi,
    state::Fundraiser,
    SECONDS_TO_DAYS,
};

/// Accounts:
/// 0. maker             (signer, mut)
/// 1. mint_to_raise
/// 2. fundraiser        (PDA, mut)
/// 3. vault             (mut)
/// 4. maker_ata         (mut) — must be pre-created by client
/// 5. token_program
#[inline(always)]
pub fn process_check_contributions(ctx: &mut InstructionContext, _data: &[u8]) -> ProgramResult {
    let maker = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _mint_to_raise = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let fundraiser = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let vault = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let maker_ata = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _token_program = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let clock = crate::take_account(unsafe { ctx.next_account_unchecked() })?;

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let current_time = unsafe {
        let clock_data = clock.try_borrow()?;
        *(clock_data.as_ptr().add(32) as *const i64)
    };

    let fund_state = unsafe { Fundraiser::from_account_unchecked(&fundraiser)? };

    if maker.address().as_array() != fund_state.maker().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    if unsafe { vault.owner() } != &pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // validating fundraiser pda address (prevent fund draining from arbitrary accounts)
    let expected_fundraiser_seeds: [&[u8]; 2] = [b"fundraiser", fund_state.maker().as_ref()];
    let expected_fundraiser = pinocchio_pubkey::derive_address(
        &expected_fundraiser_seeds,
        Some(fund_state.bump),
        crate::ID.as_array(),
    );
    if fundraiser.address().as_array() != expected_fundraiser.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    let elapsed_days = ((current_time - fund_state.time_started()) / SECONDS_TO_DAYS) as u8;
    if elapsed_days < fund_state.duration {
        return Err(err(ERR_FUNDRAISER_NOT_ENDED));
    }

    // Read vault balance via raw pointer
    // let vault_balance = {
    //     let vault_data = vault.try_borrow()?;
    //     u64::from_le_bytes(unsafe { *(vault_data.as_ptr().add(64) as *const [u8; 8]) })
    // };

    let vault_balance = {
        let vault_data = vault.try_borrow()?; // Or unwrap_unchecked() if you are sure no double borrow exists
        let vault_mint = unsafe { *(vault_data.as_ptr() as *const [u8; 32]) };
        if vault_mint != *fund_state.mint_to_raise() {
            return Err(ProgramError::InvalidAccountData);
        }
        unsafe { (vault_data.as_ptr().add(64) as *const u64).read_unaligned() }
    };

    if vault_balance < fund_state.amount_to_raise() {
        return Err(err(ERR_TARGET_NOT_MET));
    }

    // Build PDA signer
    let bump_bytes = [fund_state.bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seeds);

    // Raw CPI transfer — bypasses borrow checks
    raw_cpi::raw_transfer_signed(&vault, &maker_ata, &fundraiser, vault_balance, &[signer])?;

    // Close fundraiser PDA
    let fundraiser_lamports = fundraiser.lamports();
    maker.set_lamports(maker.lamports() + fundraiser_lamports);
    fundraiser.set_lamports(0);
    fundraiser.close()?;

    Ok(())
}

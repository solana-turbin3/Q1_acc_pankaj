use pinocchio::{
    cpi::{Seed, Signer},
    entrypoint::InstructionContext,
    error::ProgramError,
    ProgramResult,
};

use crate::{
    error::*,
    raw_cpi,
    state::{Contributor, Fundraiser},
    SECONDS_TO_DAYS,
};

/// Accounts:
/// 0. contributor       (signer, mut)
/// 1. maker
/// 2. mint_to_raise
/// 3. fundraiser        (PDA, mut)
/// 4. contributor_account (PDA, mut)
/// 5. contributor_ata   (mut)
/// 6. vault             (mut)
/// 7. token_program
///
/// Data: [contributor_bump: u8] = 1 byte
#[inline(always)]
pub fn process_refund(ctx: &mut InstructionContext, data: &[u8]) -> ProgramResult {
    let contributor_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let maker_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _mint_to_raise_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let fundraiser_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let contributor_account_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let contributor_ata_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let vault_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _token_program = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let clock = crate::take_account(unsafe { ctx.next_account_unchecked() })?;

    let contributor = &contributor_acc;
    let maker = &maker_acc;
    let _mint_to_raise = &_mint_to_raise_acc;
    let fundraiser = &fundraiser_acc;
    let contributor_account = &contributor_account_acc;
    let contributor_ata = &contributor_ata_acc;
    let vault = &vault_acc;

    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let contributor_bump = data[0];

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let current_time = unsafe {
        let clock_data = clock.try_borrow()?;
        *(clock_data.as_ptr().add(32) as *const i64)
    };

    let fund_state = unsafe { Fundraiser::from_account_unchecked(fundraiser)? };

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

    // validating contributor PDA address (using instruction bump)
    let contributor_seeds: [&[u8]; 3] = [
        b"contributor",
        fundraiser.address().as_ref(),
        contributor.address().as_ref(),
    ];
    let expected_contributor = pinocchio_pubkey::derive_address(
        &contributor_seeds,
        Some(contributor_bump),
        crate::ID.as_array(),
    );

    if contributor_account.address().as_array() != expected_contributor.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    let elapsed_days = ((current_time - fund_state.time_started()) / SECONDS_TO_DAYS) as u8;
    if elapsed_days < fund_state.duration {
        return Err(err(ERR_FUNDRAISER_NOT_ENDED));
    }

    let vault_balance = {
        let vault_data = vault.try_borrow()?;
        let vault_mint = unsafe { *(vault_data.as_ptr() as *const [u8; 32]) };
        if vault_mint != *fund_state.mint_to_raise() {
            return Err(ProgramError::InvalidAccountData);
        }
        u64::from_le_bytes(unsafe { *(vault_data.as_ptr().add(64) as *const [u8; 8]) })
    };

    if vault_balance >= fund_state.amount_to_raise() {
        return Err(err(ERR_TARGET_MET));
    }

    let cont_state = unsafe { Contributor::from_account_unchecked(contributor_account)? };
    let refund_amount = cont_state.amount();

    // Build PDA signer
    let bump_bytes = [fund_state.bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seeds);

    // Raw CPI transfer — bypasses borrow checks
    raw_cpi::raw_transfer_signed(
        &vault,
        &contributor_ata,
        &fundraiser,
        refund_amount,
        &[signer],
    )?;

    fund_state.set_current_amount(fund_state.current_amount() - refund_amount);

    // Close contributor PDA
    let cont_lamports = contributor_account.lamports();
    contributor.set_lamports(contributor.lamports() + cont_lamports);
    contributor_account.set_lamports(0);
    contributor_account.close()?;

    Ok(())
}

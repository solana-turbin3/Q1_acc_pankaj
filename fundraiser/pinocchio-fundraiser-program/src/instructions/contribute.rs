use pinocchio::{entrypoint::InstructionContext, error::ProgramError, ProgramResult};

use crate::{
    error::*,
    raw_cpi,
    state::{Contributor, Fundraiser},
    SECONDS_TO_DAYS,
};

/// Accounts:
/// 0. contributor (signer)
/// 1. mint_to_raise
/// 2. fundraiser (PDA, mut)
/// 3. contributor_account (PDA, mut) — must be pre-created via CreateContributor
/// 4. contributor_ata (mut)
/// 5. vault (mut)
/// 6. token_program
///
/// Data: [amount: u64 (8), timestamp: i64 (8)] = 16 bytes

#[inline(always)]

pub fn process_contribute(ctx: &mut InstructionContext, data: &[u8]) -> ProgramResult {
    let contributor_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _mint_to_raise_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let fundraiser_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let contributor_account_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let contributor_ata_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let vault_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _token_program = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let clock = crate::take_account(unsafe { ctx.next_account_unchecked() })?;

    let contributor = &contributor_acc;
    let _mint_to_raise = &_mint_to_raise_acc;
    let fundraiser = &fundraiser_acc;
    let contributor_account = &contributor_account_acc;
    let contributor_ata = &contributor_ata_acc;
    let vault = &vault_acc;

    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let amount = unsafe { (data.as_ptr() as *const u64).read_unaligned() };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let current_time = unsafe {
        let clock_data = clock.try_borrow()?;
        *(clock_data.as_ptr().add(32) as *const i64)
    };

    let fund_state = unsafe { Fundraiser::from_account_unchecked(fundraiser)? };

    if unsafe { vault.owner() } != &pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // PDA VALIDATION
    // validating fundraiser pda address (prevent fund draining from arbitrary accounts)
    // using derive_address to bypass syscall overhead

    let expected_fundraiser_seeds: [&[u8]; 2] = [b"fundraiser", fund_state.maker().as_ref()];

    let expected_fundraiser = pinocchio_pubkey::derive_address(
        &expected_fundraiser_seeds,
        Some(fund_state.bump),
        crate::ID.as_array(),
    );

    if fundraiser.address().as_array() != expected_fundraiser.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    // validating vault ATA address by checking token state directly (fastest, no PDA syscall)

    let vault_data = vault.try_borrow()?;
    let vault_mint = unsafe { *(vault_data.as_ptr() as *const [u8; 32]) };
    let vault_owner = unsafe { *(vault_data.as_ptr().add(32) as *const [u8; 32]) };

    if vault_mint != *fund_state.mint_to_raise() || vault_owner != *fundraiser.address().as_array()
    {
        return Err(ProgramError::InvalidAccountData);
    }

    let max_contribution = (fund_state.amount_to_raise() * 10) / 100;

    if amount > max_contribution {
        return Err(err(ERR_CONTRIBUTION_TOO_BIG));
    }

    // Checking if we will exceed the total to raise

    let new_current_amount = fund_state
        .current_amount()
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    if new_current_amount > fund_state.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }

    let elapsed_days = ((current_time - fund_state.time_started()) / SECONDS_TO_DAYS) as u8;

    if elapsed_days >= fund_state.duration {
        return Err(err(ERR_FUNDRAISER_ENDED));
    }

    let cont_state = unsafe { Contributor::from_account_unchecked(contributor_account)? };

    let new_cont_amount = cont_state
        .amount()
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    if new_cont_amount > max_contribution {
        return Err(err(ERR_MAX_CONTRIBUTIONS_REACHED));
    }

    // Raw CPI transfer — bypasses pinocchio_token borrow checks
    raw_cpi::raw_transfer(contributor_ata, vault, contributor, amount)?;
    fund_state.set_current_amount(new_current_amount);
    cont_state.set_amount(new_cont_amount);

    Ok(())
}

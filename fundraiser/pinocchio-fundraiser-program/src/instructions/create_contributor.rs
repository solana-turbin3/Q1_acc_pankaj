use pinocchio::{
    cpi::{Seed, Signer},
    entrypoint::InstructionContext,
    error::ProgramError,
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::state::Contributor;

/// Accounts:
/// 0. contributor       (signer, mut)
/// 1. fundraiser        (PDA)
/// 2. contributor_account (PDA, mut)
/// 3. system_program
///
/// Data: [bump: u8] = 1 byte
///
/// Creates the contributor PDA account. Called once per contributor.
/// Client bundles this with the first contribute ix in the same transaction.
#[inline(always)]
pub fn process_create_contributor(ctx: &mut InstructionContext, data: &[u8]) -> ProgramResult {
    let contributor_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let fundraiser_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let contributor_account_acc = crate::take_account(unsafe { ctx.next_account_unchecked() })?;

    let contributor = &contributor_acc;
    let fundraiser = &fundraiser_acc;
    let contributor_account = &contributor_account_acc;

    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = data[0];

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser.address().as_array()),
        Seed::from(contributor.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seeds);

    CreateAccount {
        from: contributor,
        to: contributor_account,
        lamports: crate::CONTRIBUTOR_RENT,
        space: Contributor::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[signer])?;

    // Initialize contributor state
    unsafe {
        let mut data = contributor_account.try_borrow_mut()?;
        let state = &mut *(data.as_mut_ptr() as *mut Contributor);
        state.disc = Contributor::DISC;
        state.set_amount(0);
    }

    Ok(())
}

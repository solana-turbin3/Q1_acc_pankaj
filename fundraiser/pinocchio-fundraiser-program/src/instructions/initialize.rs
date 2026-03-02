use pinocchio::{
    cpi::{Seed, Signer},
    entrypoint::InstructionContext,
    error::ProgramError,
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::state::Fundraiser;

/// Accounts:
/// 0. maker        (signer, mut)
/// 1. mint_to_raise
/// 2. fundraiser   (PDA, mut)
/// 3. vault        (ATA, pre-created by client)
/// 4. system_program
///
/// Data: [bump: u8, amount: u64, duration: u8, timestamp: i64] = 18 bytes
///
/// Client MUST create the vault ATA before calling this instruction:
///   vault = getAssociatedTokenAddress(fundraiser_pda, mint)
///   createAssociatedTokenAccount(payer, mint, fundraiser_pda)
#[inline(always)]
pub fn process_initialize(ctx: &mut InstructionContext, data: &[u8]) -> ProgramResult {
    let maker = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let mint_to_raise = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let fundraiser = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _vault = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let _system_program = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    let clock = crate::take_account(unsafe { ctx.next_account_unchecked() })?;
    // }

    // let maker = unsafe { accounts.get_unchecked(0).as_ref().unwrap_unchecked() };
    // let mint_to_raise = unsafe { accounts.get_unchecked(1).as_ref().unwrap_unchecked() };
    // let fundraiser = unsafe { accounts.get_unchecked(2).as_ref().unwrap_unchecked() };

    // Parse instruction data: bump(1) + amount(8) + duration(1) + timestamp(8) = 18 bytes

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let ptr = data.as_ptr();
    let (bump, amount, duration) = unsafe {
        (
            *ptr,                                        // Offset 0: bump
            (ptr.add(1) as *const u64).read_unaligned(), // Offset 1: amount
            *ptr.add(9),                                 // Offset 9: duration
        )
    };

    // Use on-chain clock for secure timestamp verification
    let clock_ts = unsafe {
        let clock_data = clock.try_borrow()?;
        *(clock_data.as_ptr().add(32) as *const i64)
    };

    // Build fundraiser PDA signer
    let bump_bytes = [bump];
    let fund_signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let fund_signer = Signer::from(&fund_signer_seeds);

    // Create fundraiser PDA account — hardcoded rent
    CreateAccount {
        from: &maker,
        to: &fundraiser,
        lamports: crate::FUNDRAISER_RENT,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[fund_signer])?;

    // Write fundraiser state (zero-copy)
    unsafe {
        let mut data = fundraiser.try_borrow_mut()?;
        let state = &mut *(data.as_mut_ptr() as *mut Fundraiser);
        state.disc = Fundraiser::DISC;
        state.set_maker(maker.address().as_array());
        state.set_mint_to_raise(mint_to_raise.address().as_array());
        state.set_amount_to_raise(amount);
        state.set_current_amount(0);
        state.set_time_started(clock_ts);
        state.duration = duration;
        state.bump = bump;
    }

    // Vault is already created by client — nothing else to do!
    Ok(())
}

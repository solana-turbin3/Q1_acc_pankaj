#![allow(unexpected_cfgs)]

use pinocchio::{
    address::declare_id, default_panic_handler, entrypoint::lazy::MaybeAccount,
    entrypoint::InstructionContext, error::ProgramError, lazy_program_entrypoint, no_allocator,
    AccountView, ProgramResult,
};

mod error;
mod instructions;
pub mod raw_cpi;
mod state;

#[cfg(test)]
mod tests;

use instructions::FundraiserInstruction;

// No heap allocator
no_allocator!();

// Lazy entrypoint — defers account parsing, saves ~400-800 CU
lazy_program_entrypoint!(process_instruction);

// Minimal panic handler
default_panic_handler!();

declare_id!("FUNDnS1gP6jvBN7bKLPFn37Brgx9q3V89JfLoXQSUBHE");

// Constants
pub const SECONDS_TO_DAYS: i64 = 86400;

// Precomputed rent-exempt minimums — avoids Rent::get() sysvar syscall
// Formula: 3480 * 2 * (128 + data_len)
pub const FUNDRAISER_RENT: u64 = 1_524_240; // 91 bytes
pub const CONTRIBUTOR_RENT: u64 = 953_520; // 9 bytes

/// Extract AccountView from MaybeAccount.
#[inline(always)]
pub fn take_account(maybe: MaybeAccount) -> Result<AccountView, ProgramError> {
    match maybe {
        MaybeAccount::Account(acc) => Ok(acc),
        MaybeAccount::Duplicated(_) => Err(ProgramError::InvalidArgument),
    }
}

#[inline(always)]
pub fn process_instruction(mut context: InstructionContext) -> ProgramResult {
    let mut context_clone = unsafe { core::ptr::read(&context as *const InstructionContext) };
    for _ in 0..context_clone.remaining() {
        let _ = unsafe { context_clone.next_account_unchecked() };
    }

    let instruction_data = unsafe { context_clone.instruction_data_unchecked() };
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let discriminator_val = *discriminator;
    let data_slice = unsafe { core::slice::from_raw_parts(data.as_ptr(), data.len()) };

    match FundraiserInstruction::try_from(&discriminator_val)? {
        FundraiserInstruction::Initialize => {
            instructions::process_initialize(&mut context, data_slice)
        }
        FundraiserInstruction::Contribute => {
            instructions::process_contribute(&mut context, data_slice)
        }
        FundraiserInstruction::CheckContributions => {
            instructions::process_check_contributions(&mut context, data_slice)
        }
        FundraiserInstruction::Refund => instructions::process_refund(&mut context, data_slice),
        FundraiserInstruction::CreateContributor => {
            instructions::process_create_contributor(&mut context, data_slice)
        }
    }
}

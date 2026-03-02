///! Token CPI helpers — ultra-optimized raw CPI for SPL Token / p-token.
///!
///! p-token is a drop-in replacement that runs at the SAME program ID as
///! SPL Token (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA). No separate
///! program address is needed — the Solana runtime uses p-token's optimized
///! implementation internally.
///!
///! Uses invoke_signed_unchecked to skip per-account borrow validation,
///! going directly to the sol_invoke_signed_c syscall for maximum CU savings.
use core::mem::MaybeUninit;
use core::slice::from_raw_parts;

use pinocchio::{
    address::Address,
    cpi::Signer,
    instruction::{
        cpi::{invoke_signed_unchecked, CpiAccount},
        InstructionAccount, InstructionView,
    },
    AccountView, ProgramResult,
};

/// SPL Token / p-token program ID (same address — p-token is a drop-in replacement)
/// TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
const TOKEN_ID: [u8; 32] = [
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
];

const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::<u8>::uninit();

/// Token Transfer CPI (no PDA signer) — skips all borrow validation.
///
/// Wire format: [discriminator(1) = 3 | amount(8 LE)] = 9 bytes
#[inline(always)]
pub fn raw_transfer(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    raw_transfer_signed(from, to, authority, amount, &[])
}

/// Token Transfer CPI with PDA signer — skips all borrow validation.
///
/// On validators running p-token, this CPI benefits from p-token's
/// optimized execution (~19x fewer CU on the token program side).
#[inline(always)]
pub fn raw_transfer_signed(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    signers: &[Signer],
) -> ProgramResult {
    let instruction_accounts: [InstructionAccount; 3] = [
        InstructionAccount::writable(from.address()),
        InstructionAccount::writable(to.address()),
        InstructionAccount::readonly_signer(authority.address()),
    ];

    // [discriminator(1) | amount(8)] = 9 bytes
    let mut data = [UNINIT_BYTE; 9];
    unsafe {
        (data.as_mut_ptr() as *mut u8).write(3u8); // Transfer discriminator = 3
        ((data.as_mut_ptr() as *mut u8).add(1) as *mut u64).write_unaligned(amount);
    }

    let instruction = InstructionView {
        program_id: unsafe { &*(&TOKEN_ID as *const [u8; 32] as *const Address) },
        accounts: &instruction_accounts,
        data: unsafe { from_raw_parts(data.as_ptr() as _, 9) },
    };

    let cpi_accounts: [CpiAccount; 3] = [
        CpiAccount::from(from),
        CpiAccount::from(to),
        CpiAccount::from(authority),
    ];

    unsafe {
        invoke_signed_unchecked(&instruction, &cpi_accounts, signers);
    }

    Ok(())
}

use pinocchio::error::ProgramError;

// Custom error codes — raw u32, no strings on-chain for zero CU overhead
pub const ERR_TARGET_NOT_MET: u32 = 0x100;
pub const ERR_TARGET_MET: u32 = 0x101;
pub const ERR_CONTRIBUTION_TOO_BIG: u32 = 0x102;
pub const ERR_MAX_CONTRIBUTIONS_REACHED: u32 = 0x104;
pub const ERR_FUNDRAISER_NOT_ENDED: u32 = 0x105;
pub const ERR_FUNDRAISER_ENDED: u32 = 0x106;

#[inline(always)]
pub fn err(code: u32) -> ProgramError {
    ProgramError::Custom(code)
}

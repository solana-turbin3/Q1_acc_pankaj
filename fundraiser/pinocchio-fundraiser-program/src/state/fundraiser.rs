use pinocchio::{error::ProgramError, AccountView};

/// Fundraiser state — zero-copy, #[repr(C)], 1-byte discriminator
///
/// Layout:
/// | disc (1) | maker (32) | mint (32) | amount_to_raise (8) | current_amount (8) | time_started (8) | duration (1) | bump (1) |
/// Total: 91 bytes
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fundraiser {
    pub disc: u8,
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
}

impl Fundraiser {
    pub const LEN: usize = 1 + 32 + 32 + 8 + 8 + 8 + 1 + 1;
    pub const DISC: u8 = 0xF0; // fundraiser discriminator

    /// Zero-copy cast from account data. Caller must verify owner + length.
    #[inline(always)]
    pub unsafe fn from_account_unchecked(account: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account.try_borrow_mut()?;
        if data[0] != Self::DISC {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut *(data.as_mut_ptr() as *mut Self))
    }

    #[inline(always)]
    pub fn maker(&self) -> &[u8; 32] {
        &self.maker
    }

    #[inline(always)]
    pub fn mint_to_raise(&self) -> &[u8; 32] {
        &self.mint_to_raise
    }

    #[inline(always)]
    pub fn amount_to_raise(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_raise)
    }

    #[inline(always)]
    pub fn current_amount(&self) -> u64 {
        u64::from_le_bytes(self.current_amount)
    }

    #[inline(always)]
    pub fn time_started(&self) -> i64 {
        i64::from_le_bytes(self.time_started)
    }

    #[inline(always)]
    pub fn set_maker(&mut self, val: &[u8; 32]) {
        self.maker.copy_from_slice(val);
    }

    #[inline(always)]
    pub fn set_mint_to_raise(&mut self, val: &[u8; 32]) {
        self.mint_to_raise.copy_from_slice(val);
    }

    #[inline(always)]
    pub fn set_amount_to_raise(&mut self, val: u64) {
        self.amount_to_raise = val.to_le_bytes();
    }

    #[inline(always)]
    pub fn set_current_amount(&mut self, val: u64) {
        self.current_amount = val.to_le_bytes();
    }

    #[inline(always)]
    pub fn set_time_started(&mut self, val: i64) {
        self.time_started = val.to_le_bytes();
    }
}

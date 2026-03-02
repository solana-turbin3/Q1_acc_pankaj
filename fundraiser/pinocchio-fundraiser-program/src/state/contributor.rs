use pinocchio::{error::ProgramError, AccountView};

/// Contributor state — zero-copy, #[repr(C)], 1-byte discriminator
///
/// Layout:
/// | disc (1) | amount (8) |
/// Total: 9 bytes
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Contributor {
    pub disc: u8,
    pub amount: [u8; 8],
}

impl Contributor {
    pub const LEN: usize = 1 + 8;
    pub const DISC: u8 = 0xC0; // contributor discriminator

    /// Zero-copy cast from account data. Caller must verify owner + length.
    #[inline(always)]
    pub unsafe fn from_account_unchecked(account: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account.try_borrow_mut()?;
        if data[0] != Self::DISC {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut *(data.as_mut_ptr() as *mut Self))
    }

    /// Safe accessor that checks disc + length
    // pub fn from_account(account: &AccountView) -> Result<&mut Self, ProgramError> {
    //     let data = account.try_borrow()?;
    //     if data.len() != Self::LEN {
    //         return Err(ProgramError::InvalidAccountData);
    //     }
    //     if data[0] != Self::DISC {
    //         return Err(ProgramError::InvalidAccountData);
    //     }
    //     drop(data);
    //     unsafe { Self::from_account_unchecked(account) }
    // }

    #[inline(always)]
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    #[inline(always)]
    pub fn set_amount(&mut self, val: u64) {
        self.amount = val.to_le_bytes();
    }
}

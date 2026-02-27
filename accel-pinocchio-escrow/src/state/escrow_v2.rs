use pinocchio::{error::ProgramError, AccountView};
use wincode::{SchemaRead, SchemaWrite};

/// Escrow state using Wincode zero-copy deserialization.
/// All fields are [u8; N] so the struct is zero-copy eligible (no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, SchemaWrite, SchemaRead)]
pub struct EscrowV2 {
    maker: [u8; 32],
    mint_a: [u8; 32],
    mint_b: [u8; 32],
    amount_to_receive: [u8; 8],
    amount_to_give: [u8; 8],
    pub bump: u8,
}

/// Instruction data for MakeV2, parsed via Wincode.
#[derive(Clone, Copy, Debug, SchemaRead)]
pub struct MakeParams {
    pub bump: u8,
    pub amount_to_receive: u64,
    pub amount_to_give: u64,
}

impl EscrowV2 {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 1;

    /// Deserialize from account data using Wincode zero-copy.
    pub fn from_account_info(account_info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // Wincode zero-copy: cast bytes directly to &mut Self
        // Safe because EscrowV2 is #[repr(C)] and all fields are [u8; N]
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn maker(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.maker)
    }

    pub fn set_maker(&mut self, maker: &pinocchio::Address) {
        self.maker.copy_from_slice(maker.as_ref());
    }

    pub fn mint_a(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.mint_a)
    }

    pub fn set_mint_a(&mut self, mint_a: &pinocchio::Address) {
        self.mint_a.copy_from_slice(mint_a.as_ref());
    }

    pub fn mint_b(&self) -> pinocchio::Address {
        pinocchio::Address::from(self.mint_b)
    }

    pub fn set_mint_b(&mut self, mint_b: &pinocchio::Address) {
        self.mint_b.copy_from_slice(mint_b.as_ref());
    }

    pub fn amount_to_receive(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_receive)
    }

    pub fn set_amount_to_receive(&mut self, amount: u64) {
        self.amount_to_receive = amount.to_le_bytes();
    }

    pub fn amount_to_give(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_give)
    }

    pub fn set_amount_to_give(&mut self, amount: u64) {
        self.amount_to_give = amount.to_le_bytes();
    }
}

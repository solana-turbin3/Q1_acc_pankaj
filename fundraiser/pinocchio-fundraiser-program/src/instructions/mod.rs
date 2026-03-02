pub mod check_contributions;
pub mod contribute;
pub mod create_contributor;
pub mod initialize;
pub mod refund;

pub use check_contributions::*;
pub use contribute::*;
pub use create_contributor::*;
pub use initialize::*;
pub use refund::*;

use pinocchio::error::ProgramError;

/// 1-byte instruction discriminator
pub enum FundraiserInstruction {
    Initialize = 0,
    Contribute = 1,
    CheckContributions = 2,
    Refund = 3,
    CreateContributor = 4,
}

impl TryFrom<&u8> for FundraiserInstruction {
    type Error = ProgramError;

    #[inline(always)]
    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstruction::Initialize),
            1 => Ok(FundraiserInstruction::Contribute),
            2 => Ok(FundraiserInstruction::CheckContributions),
            3 => Ok(FundraiserInstruction::Refund),
            4 => Ok(FundraiserInstruction::CreateContributor),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

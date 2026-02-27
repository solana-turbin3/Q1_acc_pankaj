pub mod make;
pub mod make_v2;
pub mod refund;
pub mod refund_v2;
pub mod take;
pub mod take_v2;

pub use make::*;
pub use make_v2::*;
use pinocchio::error::ProgramError;
pub use refund::*;
pub use refund_v2::*;
pub use take::*;
pub use take_v2::*;

pub enum EscrowInstrctions {
    Make = 0,
    Take = 1,
    Refund = 2,
    MakeV2 = 3,
    TakeV2 = 4,
    RefundV2 = 5,
}

impl TryFrom<&u8> for EscrowInstrctions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EscrowInstrctions::Make),
            1 => Ok(EscrowInstrctions::Take),
            2 => Ok(EscrowInstrctions::Refund),
            3 => Ok(EscrowInstrctions::MakeV2),
            4 => Ok(EscrowInstrctions::TakeV2),
            5 => Ok(EscrowInstrctions::RefundV2),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

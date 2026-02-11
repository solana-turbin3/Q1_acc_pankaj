use crate::state::UserAccount;
use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::consts;

#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    /// CHECK: The VRF identity PDA that signs the callback
    #[account(signer, address = consts::VRF_PROGRAM_IDENTITY)]
    pub vrf: AccountInfo<'info>,

    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> ConsumeRandomness<'info> {
    pub fn consume_randomness(&mut self, random_value: u64) -> Result<()> {
        msg!("Consuming randomness: {}", random_value);
        self.user_account.data = random_value;
        Ok(())
    }
}

use crate::state::UserAccount;
use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// CHECK: The oracle queue
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_EPHEMERAL_QUEUE)]
    // DEFAULT_EPHEMERAL_QUEUE (special queue optimized for Magicblock Ephemeral Rollups)
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomness<'info> {
    pub fn request_randomness(&mut self, _bump: u8) -> Result<()> {
        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.user.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: crate::ID, // which program to call
            callback_discriminator: crate::instruction::ConsumeRandomness::DISCRIMINATOR.to_vec(), // CALL this specific function
            caller_seed: [0; 32],
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            ..Default::default()
        });

        // Sending instruction to the VRF Program on-chain through Oracle-queue
        self.invoke_signed_vrf(&self.user.to_account_info(), &ix)?;

        Ok(())
    }
}

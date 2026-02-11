use crate::state::UserAccount;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use ephemeral_vrf_sdk::consts;
use ephemeral_vrf_sdk::instructions::create_request_randomness_ix;
use ephemeral_vrf_sdk::instructions::RequestRandomnessParams;

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

    /// CHECK: The ephemeral queue account.
    /// Must be passed by caller (e.g. consts::DEFAULT_EPHEMERAL_QUEUE)
    #[account(mut)]
    pub queue: AccountInfo<'info>,

    /// CHECK: The VRF program ID
    #[account(address = consts::VRF_PROGRAM_ID)]
    pub vrf: AccountInfo<'info>,

    /// CHECK: The identity PDA. Derived from this program's ID.
    /// Used by VRF to verify identity/callback.
    // We assume the PDA exists or is validly derived.
    #[account(seeds = [consts::IDENTITY], bump)]
    pub identity: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Slot hashes sysvar
    #[account(address = solana_program::sysvar::slot_hashes::ID)]
    pub slot_hashes: AccountInfo<'info>,
}

impl<'info> RequestRandomness<'info> {
    pub fn request_randomness(&mut self, bump: u8) -> Result<()> {
        let params = RequestRandomnessParams {
            payer: self.user.key(),
            oracle_queue: self.queue.key(),
            callback_program_id: crate::ID,
            callback_discriminator: crate::instruction::ConsumeRandomness::DISCRIMINATOR.to_vec(),
            accounts_metas: Some(vec![ephemeral_vrf_sdk::types::SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            caller_seed: [0; 32], // Optional seed, can use something unique if needed
            callback_args: None,
        };

        let ix = create_request_randomness_ix(params);

        solana_program::program::invoke_signed(
            &ix,
            &[
                self.user.to_account_info(),
                self.identity.to_account_info(),
                self.queue.to_account_info(),
                // self.system_program.to_account_info(), // System program likely not needed by VRF instruction itself, check SDK
                // Logic check: invoke_signed arguments must match ix.accounts.
                // create_request_randomness_ix uses: payer, identity, queue, system_program, slot_hashes
                self.system_program.to_account_info(),
                self.slot_hashes.to_account_info(),
            ],
            &[&[consts::IDENTITY, &[bump]]],
        )?;

        Ok(())
    }
}

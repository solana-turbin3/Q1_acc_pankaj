use anchor_lang::prelude::*;

use crate::state::{VaultConfig, WhitelistEntry};

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(
        init,
        payer = admin,
        space = WhitelistEntry::LEN,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    #[account(
        mut,
        constraint = admin.key() == vault_config.admin @ ErrorCode::ConstraintOwner,
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"vault-config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    #[account(
        mut,
        constraint = admin.key() == vault_config.admin @ ErrorCode::ConstraintOwner,
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"vault-config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, _user: Pubkey, bump: u8) -> Result<()> {
        self.whitelist_entry.bump = bump;
        msg!("User added to whitelist");
        Ok(())
    }
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self, _user: Pubkey) -> Result<()> {
        msg!("User removed from whitelist");
        Ok(())
    }
}

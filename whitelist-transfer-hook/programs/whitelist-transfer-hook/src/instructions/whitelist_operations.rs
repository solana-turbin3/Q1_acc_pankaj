use anchor_lang::prelude::*;

use crate::state::whitelist_entry::WhitelistEntry;

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(
        init,
        payer = admin,
        space = WhitelistEntry::LEN,
        seeds = [b"whitelist", user.key().as_ref()],
        bump
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", user.key().as_ref()],
        bump
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, _user: Pubkey, bump: u8) -> Result<()> {
        self.whitelist_entry.bump = bump;
        Ok(())
    }
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self, _user: Pubkey) -> Result<()> {
        Ok(())
    }
}

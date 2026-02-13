use anchor_lang::prelude::*;

#[account]
pub struct VaultConfig {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

impl VaultConfig {
    pub const LEN: usize = 8 + 32 + 32 + 1;
}

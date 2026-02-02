use anchor_lang::prelude::*;

#[account]
pub struct WhitelistEntry {
    pub bump: u8,
}

impl WhitelistEntry {
    pub const LEN: usize = 8 + 1;
}

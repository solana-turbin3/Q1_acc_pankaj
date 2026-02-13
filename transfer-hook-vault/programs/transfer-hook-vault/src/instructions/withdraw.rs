use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{burn, mint_to, Burn, Mint, MintTo, Token2022, TokenAccount},
};

use crate::state::{VaultConfig, WhitelistEntry};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [b"whitelist", user.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    #[account(
        mut,
        seeds = [b"vault-config"],
        bump = vault_config.bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        constraint = mint.key() == vault_config.mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// The vault's token account (ATA owned by vault_config PDA)
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_config,
        associated_token::token_program = token_program,
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// The user's token account
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let seeds = &[b"vault-config".as_ref(), &[self.vault_config.bump]];
        let signer_seeds = &[&seeds[..]];

        // 1. Burn tokens from the vault ATA
        burn(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Burn {
                    mint: self.mint.to_account_info(),
                    from: self.vault_ata.to_account_info(),
                    authority: self.vault_config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        // 2. Mint new tokens directly to user's ATA
        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.mint.to_account_info(),
                    to: self.user_ata.to_account_info(),
                    authority: self.vault_config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        msg!("Withdrew {} tokens from vault to user", amount);
        Ok(())
    }
}

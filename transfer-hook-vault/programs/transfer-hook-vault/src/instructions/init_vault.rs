use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::token_interface::{
    spl_token_2022::{
        extension::{
            transfer_hook::instruction::initialize as initialize_transfer_hook, ExtensionType,
        },
        instruction::initialize_mint2,
    },
    Token2022,
};

use crate::state::VaultConfig;

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = VaultConfig::LEN,
        seeds = [b"vault-config"],
        bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(mut)]
    pub mint: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitVault<'info> {
    pub fn init_vault(&mut self, bumps: &InitVaultBumps) -> Result<()> {
        // 1. Calculate space for mint with TransferHook + MintCloseAuthority + PermanentDelegate extensions
        let extensions = &[
            ExtensionType::TransferHook,
            ExtensionType::MintCloseAuthority,
            ExtensionType::PermanentDelegate,
        ];
        let space = ExtensionType::try_calculate_account_len::<
            anchor_spl::token_2022::spl_token_2022::state::Mint,
        >(extensions)?;
        let lamports = self.rent.minimum_balance(space);

        // 2. Create the mint account
        create_account(
            CpiContext::new(
                self.system_program.to_account_info(),
                CreateAccount {
                    from: self.admin.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            lamports,
            space as u64,
            self.token_program.key,
        )?;

        // 3. Initialize TransferHook extension (must be done before initialize_mint2)
        let ix_transfer_hook = initialize_transfer_hook(
            self.token_program.key,
            &self.mint.key(),
            Some(self.admin.key()), // Transfer hook update authority
            Some(crate::ID),        // Transfer hook program ID
        )?;
        anchor_lang::solana_program::program::invoke(
            &ix_transfer_hook,
            &[self.mint.to_account_info()],
        )?;

        // 4. Initialize MintCloseAuthority extension
        let ix_close_auth = anchor_spl::token_interface::spl_token_2022::instruction::initialize_mint_close_authority(
            self.token_program.key,
            &self.mint.key(),
            Some(&self.vault_config.key()),
        )?;
        anchor_lang::solana_program::program::invoke(
            &ix_close_auth,
            &[self.mint.to_account_info()],
        )?;

        // 5. Initialize PermanentDelegate extension
        let ix_perm_delegate = anchor_spl::token_interface::spl_token_2022::instruction::initialize_permanent_delegate(
            self.token_program.key,
            &self.mint.key(),
            &self.admin.key(), // Delegate authority = Admin for full control
        )?;
        anchor_lang::solana_program::program::invoke(
            &ix_perm_delegate,
            &[self.mint.to_account_info()],
        )?;

        // 6. Initialize the mint itself
        let ix_mint = initialize_mint2(
            self.token_program.key,
            &self.mint.key(),
            &self.vault_config.key(), // Mint authority = vault_config PDA
            Some(&self.vault_config.key()), // Freeze authority
            9,                        // decimals
        )?;
        anchor_lang::solana_program::program::invoke(&ix_mint, &[self.mint.to_account_info()])?;

        // 7. Initialize VaultConfig PDA
        self.vault_config.admin = self.admin.key();
        self.vault_config.mint = self.mint.key();
        self.vault_config.bump = bumps.vault_config;

        msg!("Vault initialized with TransferHook + MintCloseAuthority + PermanentDelegate extensions");
        Ok(())
    }
}

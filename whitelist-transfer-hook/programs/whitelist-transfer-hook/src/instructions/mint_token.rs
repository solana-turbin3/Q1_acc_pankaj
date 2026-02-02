use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        spl_token_2022::{
            extension::{
                transfer_hook::instruction::initialize as intialize_transfer_hook, ExtensionType,
            },
            instruction::initialize_mint2,
        },
        Token2022,
    },
};

#[derive(Accounts)]
pub struct TokenFactory<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub mint: Signer<'info>,
    /// CHECK: ExtraAccountMetaList Account, will be checked by the transfer hook
    #[account(mut)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> TokenFactory<'info> {
    pub fn init_mint(&mut self, _bumps: &TokenFactoryBumps) -> Result<()> {
        let payer = &self.user;
        let mint = &self.mint;
        let token_program = &self.token_program;
        let system_program = &self.system_program;
        let rent = &self.rent;
        let decimals = 9;

        // 1. Create account for the mint
        let space = ExtensionType::try_calculate_account_len::<
            anchor_spl::token_2022::spl_token_2022::state::Mint,
        >(&[ExtensionType::TransferHook])?;

        let lamports = rent.minimum_balance(space);

        create_account(
            CpiContext::new(
                system_program.to_account_info(),
                CreateAccount {
                    from: payer.to_account_info(),
                    to: mint.to_account_info(),
                },
            ),
            lamports,
            space as u64,
            token_program.key,
        )?;

        // 2. Initialize the Transfer Hook extension
        let auth = Some(self.user.key());
        let program_id = Some(crate::ID);

        // This is a direct instruction call helper, better use invoke or invoke_signed?
        // spl_token_2022::extension::transfer_hook::instruction::initialize
        // creates an Instruction.
        let ix = intialize_transfer_hook(token_program.key, &mint.key(), auth, program_id)?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                mint.to_account_info(),
                // authority check?
            ],
        )?;

        // 3. Initialize the Mint
        let ix = initialize_mint2(
            token_program.key,
            &mint.key(),
            &self.user.key(),       // Mint Authority
            Some(&self.user.key()), // Freeze Authority
            decimals,
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                mint.to_account_info(),
                rent.to_account_info(), // Rent sysvar might be needed for old token, but initialize_mint2 doesn't always need it?
                                        // initialize_mint2 is correct.
            ],
        )?;

        msg!("Mint initialized successfully with Transfer Hook extension.");

        Ok(())
    }
}

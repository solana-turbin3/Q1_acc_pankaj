#[cfg(test)]
mod tests {
    use {
        anchor_lang::{InstructionData, ToAccountMetas},
        litesvm::LiteSVM,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        spl_token_2022::extension::StateWithExtensionsOwned,
        spl_token_2022::state::Account as TokenAccount,
        std::path::PathBuf,
    };

    // Token-2022 program id
    const TOKEN_2022_PROGRAM_ID: Pubkey =
        solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
        solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

    static PROGRAM_ID: Pubkey = crate::ID;

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();

        svm.airdrop(&admin.pubkey(), 100 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL");

        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/transfer_hook_vault.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        svm.add_program(PROGRAM_ID, &program_data);

        (svm, admin)
    }

    fn get_vault_config_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault-config"], &PROGRAM_ID)
    }

    fn get_whitelist_entry_pda(user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"whitelist", user.as_ref()], &PROGRAM_ID)
    }

    fn get_extra_account_meta_list_pda(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"extra-account-metas", mint.as_ref()], &PROGRAM_ID)
    }

    fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[
                owner.as_ref(),
                TOKEN_2022_PROGRAM_ID.as_ref(),
                mint.as_ref(),
            ],
            &ASSOCIATED_TOKEN_PROGRAM_ID,
        )
        .0
    }

    /// Initialize vault + mint, returns mint keypair
    fn init_vault(svm: &mut LiteSVM, admin: &Keypair) -> Keypair {
        let mint = Keypair::new();
        let (vault_config_pda, _) = get_vault_config_pda();

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::InitVault {
                admin: admin.pubkey(),
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
                token_program: TOKEN_2022_PROGRAM_ID,
                rent: solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111"),
            }
            .to_account_metas(None),
            data: crate::instruction::InitVault {}.data(),
        };

        let message = Message::new(&[ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[admin, &mint], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("init_vault failed");

        mint
    }

    fn whitelist_user(svm: &mut LiteSVM, admin: &Keypair, user: &Pubkey) {
        let (vault_config_pda, _) = get_vault_config_pda();
        let (whitelist_entry, _) = get_whitelist_entry_pda(user);

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::AddToWhitelist {
                whitelist_entry,
                admin: admin.pubkey(),
                vault_config: vault_config_pda,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::AddToWhitelist { user: *user }.data(),
        };

        let message = Message::new(&[ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[admin], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("add_to_whitelist failed");
    }

    fn init_extra_account_meta(svm: &mut LiteSVM, admin: &Keypair, mint: &Pubkey) {
        let (extra_meta_pda, _) = get_extra_account_meta_list_pda(mint);

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::InitializeExtraAccountMetaList {
                payer: admin.pubkey(),
                extra_account_meta_list: extra_meta_pda,
                mint: *mint,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::InitializeExtraAccountMeta {}.data(),
        };

        let message = Message::new(&[ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[admin], message, svm.latest_blockhash());
        svm.send_transaction(tx)
            .expect("initialize_extra_account_meta failed");
    }

    #[test]
    fn test_init_vault() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);

        // Verify vault config exists
        let (vault_config_pda, _) = get_vault_config_pda();
        let account = svm
            .get_account(&vault_config_pda)
            .expect("VaultConfig not found");
        assert!(account.data.len() > 0, "VaultConfig should have data");

        // Verify mint exists
        let mint_account = svm.get_account(&mint.pubkey()).expect("Mint not found");
        assert!(mint_account.data.len() > 0, "Mint should have data");
        assert_eq!(
            mint_account.owner, TOKEN_2022_PROGRAM_ID,
            "Mint should be owned by Token-2022"
        );

        println!("✅ test_init_vault passed");
        println!("   VaultConfig: {}", vault_config_pda);
        println!("   Mint: {}", mint.pubkey());
    }

    #[test]
    fn test_add_to_whitelist() {
        let (mut svm, admin) = setup();
        let _mint = init_vault(&mut svm, &admin);

        let user = Keypair::new();
        whitelist_user(&mut svm, &admin, &user.pubkey());

        // Verify whitelist entry exists
        let (whitelist_entry, _) = get_whitelist_entry_pda(&user.pubkey());
        let account = svm
            .get_account(&whitelist_entry)
            .expect("WhitelistEntry not found");
        assert!(account.data.len() > 0, "WhitelistEntry should have data");

        println!("✅ test_add_to_whitelist passed");
        println!("   Whitelisted user: {}", user.pubkey());
    }

    #[test]
    fn test_remove_from_whitelist() {
        let (mut svm, admin) = setup();
        let _mint = init_vault(&mut svm, &admin);

        let user = Keypair::new();
        whitelist_user(&mut svm, &admin, &user.pubkey());

        // Now remove
        let (vault_config_pda, _) = get_vault_config_pda();
        let (whitelist_entry, _) = get_whitelist_entry_pda(&user.pubkey());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::RemoveFromWhitelist {
                whitelist_entry,
                admin: admin.pubkey(),
                vault_config: vault_config_pda,
            }
            .to_account_metas(None),
            data: crate::instruction::RemoveFromWhitelist {
                user: user.pubkey(),
            }
            .data(),
        };

        let message = Message::new(&[ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[&admin], message, svm.latest_blockhash());
        svm.send_transaction(tx)
            .expect("remove_from_whitelist failed");

        // Verify whitelist entry no longer exists (account closed)
        let account = svm.get_account(&whitelist_entry);
        assert!(
            account.is_none() || account.unwrap().data.is_empty(),
            "WhitelistEntry should be closed"
        );

        println!("✅ test_remove_from_whitelist passed");
    }

    #[test]
    fn test_init_extra_account_meta() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);

        init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

        // Verify extra account meta list exists
        let (extra_meta_pda, _) = get_extra_account_meta_list_pda(&mint.pubkey());
        let account = svm
            .get_account(&extra_meta_pda)
            .expect("ExtraAccountMetaList not found");
        assert!(
            account.data.len() > 0,
            "ExtraAccountMetaList should have data"
        );

        println!("✅ test_init_extra_account_meta passed");
        println!("   ExtraAccountMetaList: {}", extra_meta_pda);
    }

    #[test]
    fn test_deposit() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);

        // Whitelist the admin user so they can deposit
        whitelist_user(&mut svm, &admin, &admin.pubkey());

        // Init extra account meta
        init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

        let (vault_config_pda, _) = get_vault_config_pda();
        let (whitelist_entry, _) = get_whitelist_entry_pda(&admin.pubkey());
        let vault_ata = get_associated_token_address(&vault_config_pda, &mint.pubkey());

        let deposit_amount: u64 = 100 * 10u64.pow(9); // 100 tokens

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: admin.pubkey(),
                whitelist_entry,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        let message = Message::new(&[ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[&admin], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("deposit failed");

        // Verify vault ATA has tokens (use StateWithExtensionsOwned for Token-2022)
        let vault_account = svm.get_account(&vault_ata).expect("Vault ATA not found");
        let vault_token_state =
            StateWithExtensionsOwned::<TokenAccount>::unpack(vault_account.data).unwrap();
        assert_eq!(vault_token_state.base.amount, deposit_amount);

        println!("✅ test_deposit passed");
        println!("   Vault balance: {}", vault_token_state.base.amount);
    }

    #[test]
    fn test_withdraw() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);

        // Whitelist the admin
        whitelist_user(&mut svm, &admin, &admin.pubkey());

        // Init extra account meta
        let (vault_config_pda, _) = get_vault_config_pda();
        init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

        // Deposit first
        let (whitelist_entry, _) = get_whitelist_entry_pda(&admin.pubkey());
        let vault_ata = get_associated_token_address(&vault_config_pda, &mint.pubkey());

        let deposit_amount: u64 = 100 * 10u64.pow(9);
        let deposit_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: admin.pubkey(),
                whitelist_entry,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        let message = Message::new(&[deposit_ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[&admin], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("deposit failed");

        // Now withdraw (uses burn + mint_to, no transfer_checked, no reentrancy)
        let user_ata = get_associated_token_address(&admin.pubkey(), &mint.pubkey());

        let withdraw_amount: u64 = 50 * 10u64.pow(9);
        let withdraw_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Withdraw {
                user: admin.pubkey(),
                whitelist_entry,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                user_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Withdraw {
                amount: withdraw_amount,
            }
            .data(),
        };

        let message = Message::new(&[withdraw_ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[&admin], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("withdraw failed");

        // Verify balances (use StateWithExtensionsOwned for Token-2022)
        let vault_account = svm.get_account(&vault_ata).expect("Vault ATA not found");
        let vault_token_state =
            StateWithExtensionsOwned::<TokenAccount>::unpack(vault_account.data).unwrap();
        assert_eq!(
            vault_token_state.base.amount,
            deposit_amount - withdraw_amount
        );

        let user_account = svm.get_account(&user_ata).expect("User ATA not found");
        let user_token_state =
            StateWithExtensionsOwned::<TokenAccount>::unpack(user_account.data).unwrap();
        assert_eq!(user_token_state.base.amount, withdraw_amount);

        println!("✅ test_withdraw passed");
        println!("   Vault remaining: {}", vault_token_state.base.amount);
        println!("   User received: {}", user_token_state.base.amount);
    }

    #[test]
    fn test_non_whitelisted_deposit_fails() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);
        init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

        // Create a non-whitelisted user
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let (vault_config_pda, _) = get_vault_config_pda();
        let (whitelist_entry, _) = get_whitelist_entry_pda(&user.pubkey());
        let vault_ata = get_associated_token_address(&vault_config_pda, &mint.pubkey());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: user.pubkey(),
                whitelist_entry,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Deposit { amount: 100 }.data(),
        };

        let message = Message::new(&[ix], Some(&user.pubkey()));
        let tx = Transaction::new(&[&user], message, svm.latest_blockhash());
        let result = svm.send_transaction(tx);

        assert!(
            result.is_err(),
            "Deposit should fail for non-whitelisted user"
        );

        println!("✅ test_non_whitelisted_deposit_fails passed");
    }

    #[test]
    fn test_non_whitelisted_withdraw_fails() {
        let (mut svm, admin) = setup();
        let mint = init_vault(&mut svm, &admin);

        // Whitelist admin and deposit
        whitelist_user(&mut svm, &admin, &admin.pubkey());
        init_extra_account_meta(&mut svm, &admin, &mint.pubkey());

        let (vault_config_pda, _) = get_vault_config_pda();
        let (whitelist_entry_admin, _) = get_whitelist_entry_pda(&admin.pubkey());
        let vault_ata = get_associated_token_address(&vault_config_pda, &mint.pubkey());

        let deposit_amount: u64 = 100 * 10u64.pow(9);
        let deposit_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: admin.pubkey(),
                whitelist_entry: whitelist_entry_admin,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        let message = Message::new(&[deposit_ix], Some(&admin.pubkey()));
        let tx = Transaction::new(&[&admin], message, svm.latest_blockhash());
        svm.send_transaction(tx).expect("deposit failed");

        // Create a non-whitelisted user and try to withdraw
        let attacker = Keypair::new();
        svm.airdrop(&attacker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let (whitelist_entry_attacker, _) = get_whitelist_entry_pda(&attacker.pubkey());
        let attacker_ata = get_associated_token_address(&attacker.pubkey(), &mint.pubkey());

        let withdraw_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Withdraw {
                user: attacker.pubkey(),
                whitelist_entry: whitelist_entry_attacker,
                vault_config: vault_config_pda,
                mint: mint.pubkey(),
                vault_ata,
                user_ata: attacker_ata,
                token_program: TOKEN_2022_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::Withdraw { amount: 50 }.data(),
        };

        let message = Message::new(&[withdraw_ix], Some(&attacker.pubkey()));
        let tx = Transaction::new(&[&attacker], message, svm.latest_blockhash());
        let result = svm.send_transaction(tx);

        assert!(
            result.is_err(),
            "Withdraw should fail for non-whitelisted user"
        );

        println!("✅ test_non_whitelisted_withdraw_fails passed");
    }
}

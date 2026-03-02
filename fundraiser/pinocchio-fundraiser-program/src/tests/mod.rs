#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use litesvm::LiteSVM;
    use litesvm_token::{
        spl_token::{self},
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    };

    use solana_clock::Clock;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    // In tests, raw_cpi.rs routes CPI to SPL Token via #[cfg(test)].
    // On-chain (cargo build-sbf), CPI targets p-token program ID.
    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/sbpf-solana-solana/release/pinocchio_fundraiser.so");

        let program_data = std::fs::read(&so_path).unwrap_or_else(|_| {
            panic!(
                "Failed to read program SO file at {:?}. Run `cargo build-sbf` first.",
                so_path
            )
        });

        svm.add_program(program_id(), &program_data)
            .expect("Failed to add program");

        //p-token setup
        let p_token_so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("/home/pankaj/turbine/acc-class/fundraiser/pinocchio-fundraiser/src/tests/fixtures/pinocchio_token_program.so");

        let p_token_data = std::fs::read(&p_token_so_path).unwrap_or_else(|_| {
            panic!(
                "p-token.so not found at {:?}. Please add the binary.",
                p_token_so_path
            )
        });

        svm.add_program(spl_token::ID, &p_token_data)
            .expect("Failed to overwrite SPL token with p-token");

        (svm, payer)
    }

    fn send_ix(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) -> u64 {
        let payer = signers[0];
        let message = Message::new(&[ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let signer_refs: Vec<&Keypair> = signers.to_vec();
        let transaction = Transaction::new(&signer_refs, message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        tx.compute_units_consumed
    }

    /// Send two instructions in a single transaction (e.g. CreateContributor + Contribute)
    fn send_2ix(
        svm: &mut LiteSVM,
        ix1: Instruction,
        ix2: Instruction,
        signers: &[&Keypair],
    ) -> (u64,) {
        let payer = signers[0];
        let message = Message::new(&[ix1, ix2], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let signer_refs: Vec<&Keypair> = signers.to_vec();
        let transaction = Transaction::new(&signer_refs, message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        (tx.compute_units_consumed,)
    }

    fn try_send_ix(
        svm: &mut LiteSVM,
        ix: Instruction,
        signers: &[&Keypair],
    ) -> Result<u64, litesvm::types::FailedTransactionMetadata> {
        let payer = signers[0];
        let message = Message::new(&[ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let signer_refs: Vec<&Keypair> = signers.to_vec();
        let transaction = Transaction::new(&signer_refs, message, recent_blockhash);
        svm.send_transaction(transaction)
            .map(|tx| tx.compute_units_consumed)
    }

    fn read_token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
        let acc = svm.get_account(ata).expect("ATA should exist");
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    }

    fn derive_fundraiser(maker: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"fundraiser".as_ref(), maker.as_ref()], &program_id())
    }

    fn derive_vault(fundraiser_pda: &Pubkey, mint: &Pubkey) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(fundraiser_pda, mint)
    }

    fn derive_contributor_pda(fundraiser: &Pubkey, contributor: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                b"contributor".as_ref(),
                fundraiser.as_ref(),
                contributor.as_ref(),
            ],
            &program_id(),
        )
    }

    // ─── Instruction Builders ────────────────────────────────────────────

    /// Data: [disc(1), bump(1), amount(8), duration(1), timestamp(8)] = 19 bytes
    fn build_initialize_ix(
        maker: &Pubkey,
        mint: &Pubkey,
        fundraiser_pda: &Pubkey,
        vault: &Pubkey,
        bump: u8,
        amount: u64,
        duration: u8,
        timestamp: i64,
    ) -> Instruction {
        let data = [
            vec![0u8],
            vec![bump],
            amount.to_le_bytes().to_vec(),
            vec![duration],
            timestamp.to_le_bytes().to_vec(),
        ]
        .concat();

        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*maker, true),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*fundraiser_pda, false),
                AccountMeta::new_readonly(*vault, false),
                AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::clock::ID, false),
            ],
            data,
        }
    }

    /// Data: [disc(1), bump(1)] = 2 bytes
    fn build_create_contributor_ix(
        contributor: &Pubkey,
        fundraiser_pda: &Pubkey,
        contributor_pda: &Pubkey,
        bump: u8,
    ) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*contributor, true),
                AccountMeta::new_readonly(*fundraiser_pda, false),
                AccountMeta::new(*contributor_pda, false),
                AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            ],
            data: vec![4u8, bump],
        }
    }

    /// Data: [disc(1), amount(8), timestamp(8)] = 17 bytes
    /// No system_program, no contributor_bump — contributor PDA must already exist
    fn build_contribute_ix(
        contributor: &Pubkey,
        mint: &Pubkey,
        fundraiser_pda: &Pubkey,
        contributor_pda: &Pubkey,
        contributor_ata: &Pubkey,
        vault: &Pubkey,
        amount: u64,
        timestamp: i64,
    ) -> Instruction {
        let data = [
            vec![1u8],
            amount.to_le_bytes().to_vec(),
            timestamp.to_le_bytes().to_vec(),
        ]
        .concat();

        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*contributor, true),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*fundraiser_pda, false),
                AccountMeta::new(*contributor_pda, false),
                AccountMeta::new(*contributor_ata, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::clock::ID, false),
            ],
            data,
        }
    }

    fn build_check_contributions_ix(
        maker: &Pubkey,
        mint: &Pubkey,
        fundraiser_pda: &Pubkey,
        vault: &Pubkey,
        maker_ata: &Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*maker, true),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*fundraiser_pda, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*maker_ata, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::clock::ID, false),
            ],
            data: vec![2u8],
        }
    }

    /// Data: [disc(1), contributor_bump(1)] = 2 bytes
    fn build_refund_ix(
        contributor: &Pubkey,
        maker: &Pubkey,
        mint: &Pubkey,
        fundraiser_pda: &Pubkey,
        contributor_pda: &Pubkey,
        contributor_ata: &Pubkey,
        vault: &Pubkey,
        contributor_bump: u8,
    ) -> Instruction {
        let data = vec![3u8, contributor_bump];

        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*contributor, true),
                AccountMeta::new_readonly(*maker, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new(*fundraiser_pda, false),
                AccountMeta::new(*contributor_pda, false),
                AccountMeta::new(*contributor_ata, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::clock::ID, false),
            ],
            data,
        }
    }

    // ─── Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_initialize() {
        let (mut svm, maker) = setup();
        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);

        // Client creates vault ATA
        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            30_000_000,
            0,
            current_timestamp(),
        );
        let cus = send_ix(&mut svm, ix, &[&maker]);
        println!("Initialize — CUs: {}", cus);

        let fundraiser_acc = svm
            .get_account(&fundraiser_pda)
            .expect("Fundraiser should exist");
        assert_eq!(fundraiser_acc.owner, program_id());
        assert_eq!(fundraiser_acc.data.len(), 91);
        println!("test_initialize PASSED");
    }

    #[test]
    fn test_create_contributor() {
        let (mut svm, maker) = setup();
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 5 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();
        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            30_000_000,
            10,
            current_timestamp(),
        );
        send_ix(&mut svm, init_ix, &[&maker]);

        let (contributor_pda, contrib_bump) =
            derive_contributor_pda(&fundraiser_pda, &contributor.pubkey());
        let create_ix = build_create_contributor_ix(
            &contributor.pubkey(),
            &fundraiser_pda,
            &contributor_pda,
            contrib_bump,
        );
        let cus = send_ix(&mut svm, create_ix, &[&contributor]);
        println!("CreateContributor — CUs: {}", cus);

        let pda_acc = svm
            .get_account(&contributor_pda)
            .expect("Contributor PDA should exist");
        assert_eq!(pda_acc.owner, program_id());
        println!("test_create_contributor PASSED");
    }

    #[test]
    fn test_contribute() {
        let (mut svm, maker) = setup();
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();
        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 10_000_000)
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);
        let ts = current_timestamp();

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            30_000_000,
            10,
            ts,
        );
        send_ix(&mut svm, init_ix, &[&maker]);

        // Create contributor PDA first, then contribute — same tx possible, separate here to measure CU
        let (contributor_pda, contrib_bump) =
            derive_contributor_pda(&fundraiser_pda, &contributor.pubkey());
        let create_ix = build_create_contributor_ix(
            &contributor.pubkey(),
            &fundraiser_pda,
            &contributor_pda,
            contrib_bump,
        );
        let create_cus = send_ix(&mut svm, create_ix, &[&contributor]);
        println!("CreateContributor — CUs: {}", create_cus);

        let contribute_ix = build_contribute_ix(
            &contributor.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            1_000_000,
            ts,
        );
        let cus = send_ix(&mut svm, contribute_ix, &[&contributor]);
        println!("Contribute #1 — CUs: {}", cus);
        assert_eq!(read_token_balance(&svm, &vault), 1_000_000);

        svm.expire_blockhash();

        // Repeat contribute — no CreateContributor needed
        let contribute_ix2 = build_contribute_ix(
            &contributor.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            1_000_000,
            ts,
        );
        let cus2 = send_ix(&mut svm, contribute_ix2, &[&contributor]);
        println!("Contribute #2 — CUs: {}", cus2);
        assert_eq!(read_token_balance(&svm, &vault), 2_000_000);
        println!("test_contribute PASSED");
    }

    #[test]
    fn test_contribute_too_big() {
        let (mut svm, maker) = setup();
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();
        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 100_000_000)
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);
        let ts = current_timestamp();

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            30_000_000,
            10,
            ts,
        );
        send_ix(&mut svm, init_ix, &[&maker]);

        let (contributor_pda, contrib_bump) =
            derive_contributor_pda(&fundraiser_pda, &contributor.pubkey());
        send_ix(
            &mut svm,
            build_create_contributor_ix(
                &contributor.pubkey(),
                &fundraiser_pda,
                &contributor_pda,
                contrib_bump,
            ),
            &[&contributor],
        );

        let contribute_ix = build_contribute_ix(
            &contributor.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            4_000_000,
            ts,
        );
        let result = try_send_ix(&mut svm, contribute_ix, &[&contributor]);
        assert!(result.is_err(), "Should fail: contribution too big");
        println!("test_contribute_too_big PASSED");
    }

    #[test]
    fn test_refund() {
        let (mut svm, maker) = setup();
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();
        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 10_000_000)
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);
        let ts = current_timestamp();

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            30_000_000,
            10,
            ts,
        );
        send_ix(&mut svm, init_ix, &[&maker]);

        let (contributor_pda, contrib_bump) =
            derive_contributor_pda(&fundraiser_pda, &contributor.pubkey());
        send_ix(
            &mut svm,
            build_create_contributor_ix(
                &contributor.pubkey(),
                &fundraiser_pda,
                &contributor_pda,
                contrib_bump,
            ),
            &[&contributor],
        );

        let contribute_ix = build_contribute_ix(
            &contributor.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            1_000_000,
            ts,
        );
        send_ix(&mut svm, contribute_ix, &[&contributor]);

        // Advance time by 11 days (950400 seconds)
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp += 950400;
        svm.set_sysvar(&clock);

        let balance_before = read_token_balance(&svm, &contributor_ata);
        let refund_ix = build_refund_ix(
            &contributor.pubkey(),
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            contrib_bump,
        );
        let cus = send_ix(&mut svm, refund_ix, &[&contributor]);
        println!("Refund — CUs: {}", cus);

        assert_eq!(
            read_token_balance(&svm, &contributor_ata),
            balance_before + 1_000_000
        );
        assert_eq!(read_token_balance(&svm, &vault), 0);
        println!("test_refund PASSED");
    }

    #[test]
    fn test_check_contributions_target_not_met() {
        let (mut svm, maker) = setup();
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();
        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();
        MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 50_000_000)
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);
        let ts = current_timestamp();

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            4_000_000,
            10,
            ts,
        );
        send_ix(&mut svm, init_ix, &[&maker]);

        let (contributor_pda, contrib_bump) =
            derive_contributor_pda(&fundraiser_pda, &contributor.pubkey());
        send_ix(
            &mut svm,
            build_create_contributor_ix(
                &contributor.pubkey(),
                &fundraiser_pda,
                &contributor_pda,
                contrib_bump,
            ),
            &[&contributor],
        );

        let contribute_ix = build_contribute_ix(
            &contributor.pubkey(),
            &mint,
            &fundraiser_pda,
            &contributor_pda,
            &contributor_ata,
            &vault,
            100_000,
            ts,
        );
        send_ix(&mut svm, contribute_ix, &[&contributor]);

        let maker_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let check_ix = build_check_contributions_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            &maker_ata,
        );

        // Target not met AND not ended: fails.
        // Even if we advance time, it should fail because target is not met.
        // Let's advance it to ensure target check is hit.
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp += 950400;
        svm.set_sysvar(&clock);

        let result = try_send_ix(&mut svm, check_ix, &[&maker]);
        assert!(result.is_err(), "Should fail: target not met");
        println!("test_check_contributions_target_not_met PASSED");
    }

    #[test]
    fn test_full_flow() {
        let (mut svm, maker) = setup();
        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let (fundraiser_pda, bump) = derive_fundraiser(&maker.pubkey());
        let vault = derive_vault(&fundraiser_pda, &mint);
        let ts = current_timestamp();
        let amount_to_raise: u64 = 1000;

        CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&fundraiser_pda)
            .send()
            .unwrap();

        let init_ix = build_initialize_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            bump,
            amount_to_raise,
            10,
            ts,
        );
        let init_cus = send_ix(&mut svm, init_ix, &[&maker]);
        println!("Initialize — CUs: {}", init_cus);

        for i in 0..10 {
            let contrib = Keypair::new();
            svm.airdrop(&contrib.pubkey(), 2 * LAMPORTS_PER_SOL)
                .unwrap();
            let contrib_ata = CreateAssociatedTokenAccount::new(&mut svm, &contrib, &mint)
                .owner(&contrib.pubkey())
                .send()
                .unwrap();
            MintTo::new(&mut svm, &maker, &mint, &contrib_ata, 1000)
                .send()
                .unwrap();

            let (contrib_pda, contrib_bump) =
                derive_contributor_pda(&fundraiser_pda, &contrib.pubkey());

            // Bundle CreateContributor + Contribute in same TX
            let create_ix = build_create_contributor_ix(
                &contrib.pubkey(),
                &fundraiser_pda,
                &contrib_pda,
                contrib_bump,
            );
            let contribute_ix = build_contribute_ix(
                &contrib.pubkey(),
                &mint,
                &fundraiser_pda,
                &contrib_pda,
                &contrib_ata,
                &vault,
                100,
                ts,
            );
            let (total_cus,) = send_2ix(&mut svm, create_ix, contribute_ix, &[&contrib]);
            println!(
                "Contributor #{} (create+contribute) — CUs: {}",
                i + 1,
                total_cus
            );
        }

        assert_eq!(read_token_balance(&svm, &vault), 1000);

        let maker_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let check_ix = build_check_contributions_ix(
            &maker.pubkey(),
            &mint,
            &fundraiser_pda,
            &vault,
            &maker_ata,
        );

        // Advance time to pass the duration check (10 days duration)
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp += 950400;
        svm.set_sysvar(&clock);

        let check_cus = send_ix(&mut svm, check_ix, &[&maker]);
        println!("CheckContributions — CUs: {}", check_cus);

        assert_eq!(read_token_balance(&svm, &maker_ata), 1000);

        let fundraiser_acc = svm.get_account(&fundraiser_pda);
        assert!(
            fundraiser_acc.is_none() || fundraiser_acc.unwrap().lamports == 0,
            "Fundraiser should be closed"
        );
        println!("test_full_flow PASSED");
    }
}

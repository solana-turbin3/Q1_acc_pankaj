#[cfg(test)]
mod benchmark {
    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{
        spl_token::{self},
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    };

    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const PROGRAM_ID: &str = "4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT";
    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let so_path = PathBuf::from("/home/pankaj/turbine/acc-class/accel-pinocchio-escrow/target/sbpf-solana-solana/release/escrow.so");

        let program_data = std::fs::read(&so_path).unwrap_or_else(|_| {
            panic!(
                "Failed to read program SO file at {:?}. Run `cargo-build-sbf` first.",
                so_path
            )
        });

        svm.add_program(program_id(), &program_data)
            .expect("Failed to add program");

        (svm, payer)
    }

    fn create_mints_and_ata(
        svm: &mut LiteSVM,
        maker: &Keypair,
        mint_amount: u64,
    ) -> (Pubkey, Pubkey, Pubkey) {
        let mint_a = CreateMint::new(svm, maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(svm, maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(svm, maker, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        MintTo::new(svm, maker, &mint_a, &maker_ata_a, mint_amount)
            .send()
            .unwrap();

        (mint_a, mint_b, maker_ata_a)
    }

    fn derive_escrow_and_vault(maker: &Pubkey, mint_a: &Pubkey) -> (Pubkey, u8, Pubkey) {
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), maker.as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        let vault = spl_associated_token_account::get_associated_token_address(&escrow.0, mint_a);
        (escrow.0, escrow.1, vault)
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

    fn make_escrow_with_discriminator(
        svm: &mut LiteSVM,
        maker: &Keypair,
        amount_to_receive: u64,
        amount_to_give: u64,
        discriminator: u8,
    ) -> (Pubkey, Pubkey, Pubkey, u8, Pubkey) {
        let (mint_a, mint_b, _maker_ata_a) =
            create_mints_and_ata(svm, maker, amount_to_give + 100_000_000);
        let (escrow_pda, bump, vault) = derive_escrow_and_vault(&maker.pubkey(), &mint_a);

        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        let make_data = [
            vec![discriminator],
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ]
        .concat();

        let make_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_data,
        };

        let _cus = send_ix(svm, make_ix, &[maker]);

        (mint_a, mint_b, escrow_pda, bump, vault)
    }

    fn build_take_ix(
        taker: &Pubkey,
        maker: &Pubkey,
        mint_a: &Pubkey,
        mint_b: &Pubkey,
        escrow_pda: &Pubkey,
        vault: &Pubkey,
        taker_ata_a: &Pubkey,
        taker_ata_b: &Pubkey,
        maker_ata_b: &Pubkey,
        discriminator: u8,
    ) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*taker, true),
                AccountMeta::new(*maker, false),
                AccountMeta::new(*mint_a, false),
                AccountMeta::new(*mint_b, false),
                AccountMeta::new(*escrow_pda, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*taker_ata_a, false),
                AccountMeta::new(*taker_ata_b, false),
                AccountMeta::new(*maker_ata_b, false),
                AccountMeta::new(TOKEN_PROGRAM_ID, false),
                AccountMeta::new(solana_sdk_ids::system_program::ID, false),
            ],
            data: vec![discriminator],
        }
    }

    fn build_refund_ix(
        maker: &Pubkey,
        mint_a: &Pubkey,
        escrow_pda: &Pubkey,
        vault: &Pubkey,
        maker_ata_a: &Pubkey,
        discriminator: u8,
    ) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*maker, true),
                AccountMeta::new(*mint_a, false),
                AccountMeta::new(*escrow_pda, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*maker_ata_a, false),
                AccountMeta::new(TOKEN_PROGRAM_ID, false),
                AccountMeta::new(solana_sdk_ids::system_program::ID, false),
            ],
            data: vec![discriminator],
        }
    }

    fn setup_taker(
        svm: &mut LiteSVM,
        maker: &Keypair,
        taker: &Keypair,
        mint_a: &Pubkey,
        mint_b: &Pubkey,
        amount_to_receive: u64,
    ) -> (Pubkey, Pubkey, Pubkey) {
        let taker_ata_b = CreateAssociatedTokenAccount::new(svm, taker, mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        MintTo::new(svm, maker, mint_b, &taker_ata_b, amount_to_receive)
            .send()
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(svm, taker, mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(svm, maker, mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        (taker_ata_a, taker_ata_b, maker_ata_b)
    }

    // ─── Benchmark Helpers ───────────────────────────────────────────────

    fn bench_make_only(
        svm: &mut LiteSVM,
        maker: &Keypair,
        amount_to_receive: u64,
        amount_to_give: u64,
        discriminator: u8,
    ) -> u64 {
        let (mint_a, mint_b, _maker_ata_a) =
            create_mints_and_ata(svm, maker, amount_to_give + 100_000_000);
        let (escrow_pda, bump, vault) = derive_escrow_and_vault(&maker.pubkey(), &mint_a);

        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        let make_data = [
            vec![discriminator],
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ]
        .concat();

        let make_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_data,
        };

        send_ix(svm, make_ix, &[maker])
    }

    fn bench_make_refund(
        svm: &mut LiteSVM,
        maker: &Keypair,
        amount_to_receive: u64,
        amount_to_give: u64,
        make_disc: u8,
        refund_disc: u8,
    ) -> u64 {
        let (mint_a, _mint_b, escrow_pda, _bump, vault) = make_escrow_with_discriminator(
            svm,
            maker,
            amount_to_receive,
            amount_to_give,
            make_disc,
        );

        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);

        let refund_ix = build_refund_ix(
            &maker.pubkey(),
            &mint_a,
            &escrow_pda,
            &vault,
            &maker_ata_a,
            refund_disc,
        );

        send_ix(svm, refund_ix, &[maker])
    }

    // ─── The Benchmark ──────────────────────────────────────────────────

    #[test]
    pub fn benchmark_v1_vs_v2() {
        const ITERATIONS: usize = 5;
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let mut v1_make_cus = Vec::with_capacity(ITERATIONS);
        let mut v2_make_cus = Vec::with_capacity(ITERATIONS);
        let mut v1_take_cus = Vec::with_capacity(ITERATIONS);
        let mut v2_take_cus = Vec::with_capacity(ITERATIONS);
        let mut v1_refund_cus = Vec::with_capacity(ITERATIONS);
        let mut v2_refund_cus = Vec::with_capacity(ITERATIONS);

        // ── Make Benchmark ──
        for _ in 0..ITERATIONS {
            let (mut svm, maker) = setup();
            v1_make_cus.push(bench_make_only(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                0,
            ));

            let (mut svm, maker) = setup();
            v2_make_cus.push(bench_make_only(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                3,
            ));
        }

        // ── Take Benchmark (Make → Take) ──
        for _ in 0..ITERATIONS {
            // V1
            let (mut svm, maker) = setup();
            let taker = Keypair::new();
            svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
            let (mint_a, mint_b, escrow_pda, _, vault) = make_escrow_with_discriminator(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                0,
            );
            let (taker_ata_a, taker_ata_b, maker_ata_b) = setup_taker(
                &mut svm,
                &maker,
                &taker,
                &mint_a,
                &mint_b,
                amount_to_receive,
            );
            let ix = build_take_ix(
                &taker.pubkey(),
                &maker.pubkey(),
                &mint_a,
                &mint_b,
                &escrow_pda,
                &vault,
                &taker_ata_a,
                &taker_ata_b,
                &maker_ata_b,
                1,
            );
            v1_take_cus.push(send_ix(&mut svm, ix, &[&taker]));

            // V2
            let (mut svm, maker) = setup();
            let taker = Keypair::new();
            svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
            let (mint_a, mint_b, escrow_pda, _, vault) = make_escrow_with_discriminator(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                3,
            );
            let (taker_ata_a, taker_ata_b, maker_ata_b) = setup_taker(
                &mut svm,
                &maker,
                &taker,
                &mint_a,
                &mint_b,
                amount_to_receive,
            );
            let ix = build_take_ix(
                &taker.pubkey(),
                &maker.pubkey(),
                &mint_a,
                &mint_b,
                &escrow_pda,
                &vault,
                &taker_ata_a,
                &taker_ata_b,
                &maker_ata_b,
                4,
            );
            v2_take_cus.push(send_ix(&mut svm, ix, &[&taker]));
        }

        // ── Refund Benchmark (Make → Refund) ──
        for _ in 0..ITERATIONS {
            let (mut svm, maker) = setup();
            v1_refund_cus.push(bench_make_refund(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                0,
                2,
            ));

            let (mut svm, maker) = setup();
            v2_refund_cus.push(bench_make_refund(
                &mut svm,
                &maker,
                amount_to_receive,
                amount_to_give,
                3,
                5,
            ));
        }

        // ── Print Results ──
        let avg = |v: &[u64]| -> u64 { v.iter().sum::<u64>() / v.len() as u64 };

        let v1_make_avg = avg(&v1_make_cus);
        let v2_make_avg = avg(&v2_make_cus);
        let v1_take_avg = avg(&v1_take_cus);
        let v2_take_avg = avg(&v2_take_cus);
        let v1_refund_avg = avg(&v1_refund_cus);
        let v2_refund_avg = avg(&v2_refund_cus);

        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║        BENCHMARK: V1 (Raw Pointer) vs V2 (Wincode)         ║");
        println!(
            "║                 ({} iterations each)                       ║",
            ITERATIONS
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Instruction │  V1 (CUs)  │  V2 (CUs)  │   Delta  │  Diff%  ║");
        println!("╠═════════════╪════════════╪════════════╪══════════╪═════════╣");
        println!(
            "║ Make        │ {:>10} │ {:>10} │ {:>+8} │ {:>+6.2}% ║",
            v1_make_avg,
            v2_make_avg,
            v2_make_avg as i64 - v1_make_avg as i64,
            (v2_make_avg as f64 - v1_make_avg as f64) / v1_make_avg as f64 * 100.0
        );
        println!(
            "║ Take        │ {:>10} │ {:>10} │ {:>+8} │ {:>+6.2}% ║",
            v1_take_avg,
            v2_take_avg,
            v2_take_avg as i64 - v1_take_avg as i64,
            (v2_take_avg as f64 - v1_take_avg as f64) / v1_take_avg as f64 * 100.0
        );
        println!(
            "║ Refund      │ {:>10} │ {:>10} │ {:>+8} │ {:>+6.2}% ║",
            v1_refund_avg,
            v2_refund_avg,
            v2_refund_avg as i64 - v1_refund_avg as i64,
            (v2_refund_avg as f64 - v1_refund_avg as f64) / v1_refund_avg as f64 * 100.0
        );
        println!("╠═════════════╪════════════╪════════════╪══════════╪═════════╣");

        let v1_total = v1_make_avg + v1_take_avg + v1_refund_avg;
        let v2_total = v2_make_avg + v2_take_avg + v2_refund_avg;
        println!(
            "║ TOTAL       │ {:>10} │ {:>10} │ {:>+8} │ {:>+6.2}% ║",
            v1_total,
            v2_total,
            v2_total as i64 - v1_total as i64,
            (v2_total as f64 - v1_total as f64) / v1_total as f64 * 100.0
        );
        println!("╚══════════════════════════════════════════════════════════════╝");

        println!("\nRaw data (CUs per iteration):");
        println!("  V1 Make:   {:?}", v1_make_cus);
        println!("  V2 Make:   {:?}", v2_make_cus);
        println!("  V1 Take:   {:?}", v1_take_cus);
        println!("  V2 Take:   {:?}", v2_take_cus);
        println!("  V1 Refund: {:?}", v1_refund_cus);
        println!("  V2 Refund: {:?}", v2_refund_cus);
    }
}

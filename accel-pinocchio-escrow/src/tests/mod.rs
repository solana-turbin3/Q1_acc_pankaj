#[cfg(test)]
mod tests {

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

    // ─── Shared Helpers ──────────────────────────────────────────────────

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

    /// Create two mints and the maker's ATA for mint_a, mint tokens.
    /// Returns (mint_a, mint_b, maker_ata_a).
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

    /// Derive escrow PDA and vault ATA.
    /// Returns (escrow_pda, bump, vault).
    fn derive_escrow_and_vault(maker: &Pubkey, mint_a: &Pubkey) -> (Pubkey, u8, Pubkey) {
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), maker.as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        let vault = spl_associated_token_account::get_associated_token_address(&escrow.0, mint_a);
        (escrow.0, escrow.1, vault)
    }

    /// Build and send a single instruction, return CUs consumed.
    fn send_ix(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) -> u64 {
        let payer = signers[0];
        let message = Message::new(&[ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let signer_refs: Vec<&Keypair> = signers.to_vec();
        let transaction = Transaction::new(&signer_refs, message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        tx.compute_units_consumed
    }

    /// Build Make/MakeV2 instruction data and accounts, execute and return results.
    /// `discriminator`: 0 = Make (V1), 3 = MakeV2
    /// Returns (mint_a, mint_b, escrow_pda, bump, vault).
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

        let cus = send_ix(svm, make_ix, &[maker]);
        println!(
            "Make (disc={}) transaction successful — CUs: {}",
            discriminator, cus
        );

        (mint_a, mint_b, escrow_pda, bump, vault)
    }

    /// Build Take instruction accounts and data.
    /// `discriminator`: 1 = Take (V1), 4 = TakeV2
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
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

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
                AccountMeta::new(token_program, false),
                AccountMeta::new(system_program, false),
            ],
            data: vec![discriminator],
        }
    }

    /// Build Refund instruction accounts and data.
    /// `discriminator`: 2 = Refund (V1), 5 = RefundV2
    fn build_refund_ix(
        maker: &Pubkey,
        mint_a: &Pubkey,
        escrow_pda: &Pubkey,
        vault: &Pubkey,
        maker_ata_a: &Pubkey,
        discriminator: u8,
    ) -> Instruction {
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*maker, true),
                AccountMeta::new(*mint_a, false),
                AccountMeta::new(*escrow_pda, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*maker_ata_a, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(system_program, false),
            ],
            data: vec![discriminator],
        }
    }

    /// Setup taker with ATAs for mint_a and mint_b, mint tokens to taker_ata_b.
    /// Returns (taker_ata_a, taker_ata_b, maker_ata_b).
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

    /// Read SPL token balance from account data (amount at offset 64).
    fn read_token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
        let acc = svm.get_account(ata).expect("ATA should exist");
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    }

    /// Assert escrow and vault are closed.
    fn assert_closed(svm: &LiteSVM, escrow_pda: &Pubkey, vault: &Pubkey) {
        assert!(svm.get_account(vault).is_none(), "Vault should be closed");
        let escrow_acc = svm.get_account(escrow_pda);
        assert!(
            escrow_acc.is_none() || escrow_acc.unwrap().lamports == 0,
            "Escrow should be closed"
        );
    }

    // ─── V1 Tests ────────────────────────────────────────────────────────

    #[test]
    pub fn test_make_instruction() {
        let (mut svm, maker) = setup();
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (_mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 0);

        let escrow_acc = svm.get_account(&escrow_pda).expect("Escrow should exist");
        assert_eq!(escrow_acc.owner, program_id());
        assert_eq!(read_token_balance(&svm, &vault), amount_to_give);

        println!("test_make_instruction passed");
    }

    #[test]
    pub fn test_take_instruction() {
        let (mut svm, maker) = setup();
        let taker = Keypair::new();
        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (mint_a, mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 0);

        let (taker_ata_a, taker_ata_b, maker_ata_b) = setup_taker(
            &mut svm,
            &maker,
            &taker,
            &mint_a,
            &mint_b,
            amount_to_receive,
        );

        let take_ix = build_take_ix(
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

        let cus = send_ix(&mut svm, take_ix, &[&taker]);
        println!("Take transaction successful — CUs: {}", cus);

        assert_eq!(read_token_balance(&svm, &taker_ata_a), amount_to_give);
        assert_eq!(read_token_balance(&svm, &maker_ata_b), amount_to_receive);
        assert_closed(&svm, &escrow_pda, &vault);

        println!("test_take_instruction passed");
    }

    #[test]
    pub fn test_refund_instruction() {
        let (mut svm, maker) = setup();
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 0);

        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);
        let balance_before = read_token_balance(&svm, &maker_ata_a);

        let refund_ix = build_refund_ix(
            &maker.pubkey(),
            &mint_a,
            &escrow_pda,
            &vault,
            &maker_ata_a,
            2,
        );

        let cus = send_ix(&mut svm, refund_ix, &[&maker]);
        println!("Refund transaction successful — CUs: {}", cus);

        assert_eq!(
            read_token_balance(&svm, &maker_ata_a),
            balance_before + amount_to_give
        );
        assert_closed(&svm, &escrow_pda, &vault);

        println!("test_refund_instruction passed");
    }

    // ─── V2 Tests (Wincode) ──────────────────────────────────────────────

    #[test]
    pub fn test_make_v2_instruction() {
        let (mut svm, maker) = setup();
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (_mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 3);

        let escrow_acc = svm.get_account(&escrow_pda).expect("Escrow should exist");
        assert_eq!(escrow_acc.owner, program_id());
        assert_eq!(read_token_balance(&svm, &vault), amount_to_give);

        println!("test_make_v2_instruction passed");
    }

    #[test]
    pub fn test_take_v2_instruction() {
        let (mut svm, maker) = setup();
        let taker = Keypair::new();
        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        // Use MakeV2 to create, TakeV2 to take
        let (mint_a, mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 3);

        let (taker_ata_a, taker_ata_b, maker_ata_b) = setup_taker(
            &mut svm,
            &maker,
            &taker,
            &mint_a,
            &mint_b,
            amount_to_receive,
        );

        let take_ix = build_take_ix(
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

        let cus = send_ix(&mut svm, take_ix, &[&taker]);
        println!("TakeV2 transaction successful — CUs: {}", cus);

        assert_eq!(read_token_balance(&svm, &taker_ata_a), amount_to_give);
        assert_eq!(read_token_balance(&svm, &maker_ata_b), amount_to_receive);
        assert_closed(&svm, &escrow_pda, &vault);

        println!("test_take_v2_instruction passed");
    }

    #[test]
    pub fn test_refund_v2_instruction() {
        let (mut svm, maker) = setup();
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        // Use MakeV2 to create, RefundV2 to cancel
        let (mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow_with_discriminator(&mut svm, &maker, amount_to_receive, amount_to_give, 3);

        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);
        let balance_before = read_token_balance(&svm, &maker_ata_a);

        let refund_ix = build_refund_ix(
            &maker.pubkey(),
            &mint_a,
            &escrow_pda,
            &vault,
            &maker_ata_a,
            5,
        );

        let cus = send_ix(&mut svm, refund_ix, &[&maker]);
        println!("RefundV2 transaction successful — CUs: {}", cus);

        assert_eq!(
            read_token_balance(&svm, &maker_ata_a),
            balance_before + amount_to_give
        );
        assert_closed(&svm, &escrow_pda, &vault);

        println!("test_refund_v2_instruction passed");
    }
}

mod benchmark;

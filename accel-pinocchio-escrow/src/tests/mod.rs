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

    /// Helper: create mints, ATAs, and execute the Make instruction.
    /// Returns (mint_a, mint_b, escrow PDA, escrow bump, vault address).
    fn make_escrow(
        svm: &mut LiteSVM,
        maker: &Keypair,
        amount_to_receive: u64,
        amount_to_give: u64,
    ) -> (Pubkey, Pubkey, Pubkey, u8, Pubkey) {
        let program_id = program_id();

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

        // Create the maker's ATA for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(svm, maker, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        // Derive escrow PDA
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), maker.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );

        // Derive vault (escrow's ATA for mint_a)
        let vault = spl_associated_token_account::get_associated_token_address(&escrow.0, &mint_a);

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Mint tokens to maker's ATA for mint_a
        MintTo::new(
            svm,
            maker,
            &mint_a,
            &maker_ata_a,
            amount_to_give + 100_000_000,
        )
        .send()
        .unwrap();

        let bump: u8 = escrow.1;

        // Build "Make" instruction data
        let make_data = [
            vec![0u8], // discriminator for Make
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ]
        .concat();

        let make_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: make_data,
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[maker], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        println!(
            "Make transaction successful — CUs: {}",
            tx.compute_units_consumed
        );

        (mint_a, mint_b, escrow.0, escrow.1, vault)
    }

    #[test]
    pub fn test_make_instruction() {
        let (mut svm, maker) = setup();

        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (_mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow(&mut svm, &maker, amount_to_receive, amount_to_give);

        println!("Escrow PDA: {}", escrow_pda);
        println!("Vault: {}", vault);

        // Verify escrow account exists and is owned by our program
        let escrow_acc = svm
            .get_account(&escrow_pda)
            .expect("Escrow account should exist");
        assert_eq!(escrow_acc.owner, program_id());

        // Verify vault has the deposited tokens
        let vault_acc = svm.get_account(&vault).expect("Vault account should exist");
        // SPL token account data: amount is at offset 64, 8 bytes LE
        let vault_amount = u64::from_le_bytes(vault_acc.data[64..72].try_into().unwrap());
        assert_eq!(vault_amount, amount_to_give);

        println!("test_make_instruction passed");
    }

    #[test]
    pub fn test_take_instruction() {
        let (mut svm, maker) = setup();
        let taker = Keypair::new();
        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let program_id = program_id();
        let amount_to_receive: u64 = 100_000_000; // 100 tokens of mint_b
        let amount_to_give: u64 = 500_000_000; // 500 tokens of mint_a

        let (mint_a, mint_b, escrow_pda, _bump, vault) =
            make_escrow(&mut svm, &maker, amount_to_receive, amount_to_give);

        // Create taker's ATA for mint_b and mint some tokens
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();
        // Mint mint_b tokens to taker (maker is authority of mint_b)
        MintTo::new(&mut svm, &maker, &mint_b, &taker_ata_b, amount_to_receive)
            .send()
            .unwrap();

        // Create taker's ATA for mint_a (to receive tokens from vault)
        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Create maker's ATA for mint_b (to receive tokens from taker)
        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Build "Take" instruction (discriminator = 1)
        let take_data = vec![1u8]; // discriminator for Take

        let take_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(maker.pubkey(), false),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(taker_ata_a, false),
                AccountMeta::new(taker_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(system_program, false),
            ],
            data: take_data,
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        println!(
            "Take transaction successful — CUs: {}",
            tx.compute_units_consumed
        );

        // Verify taker received mint_a tokens
        let taker_a_acc = svm
            .get_account(&taker_ata_a)
            .expect("Taker ATA A should exist");
        let taker_a_amount = u64::from_le_bytes(taker_a_acc.data[64..72].try_into().unwrap());
        assert_eq!(taker_a_amount, amount_to_give);

        // Verify maker received mint_b tokens
        let maker_b_acc = svm
            .get_account(&maker_ata_b)
            .expect("Maker ATA B should exist");
        let maker_b_amount = u64::from_le_bytes(maker_b_acc.data[64..72].try_into().unwrap());
        assert_eq!(maker_b_amount, amount_to_receive);

        // Verify vault is closed
        assert!(svm.get_account(&vault).is_none(), "Vault should be closed");

        // Verify escrow account is closed (zero lamports)
        let escrow_acc = svm.get_account(&escrow_pda);
        assert!(
            escrow_acc.is_none() || escrow_acc.unwrap().lamports == 0,
            "Escrow should be closed"
        );

        println!("test_take_instruction passed");
    }

    #[test]
    pub fn test_refund_instruction() {
        let (mut svm, maker) = setup();

        let program_id = program_id();
        let amount_to_receive: u64 = 100_000_000;
        let amount_to_give: u64 = 500_000_000;

        let (mint_a, _mint_b, escrow_pda, _bump, vault) =
            make_escrow(&mut svm, &maker, amount_to_receive, amount_to_give);

        // Maker's ATA for mint_a (already exists from make_escrow)
        let maker_ata_a =
            spl_associated_token_account::get_associated_token_address(&maker.pubkey(), &mint_a);

        // Check maker's balance before refund
        let maker_a_before = svm
            .get_account(&maker_ata_a)
            .expect("Maker ATA A should exist");
        let maker_a_before_amount =
            u64::from_le_bytes(maker_a_before.data[64..72].try_into().unwrap());
        println!(
            "Maker mint_a balance before refund: {}",
            maker_a_before_amount
        );

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Build "Refund" instruction (discriminator = 2)
        let refund_data = vec![2u8]; // discriminator for Refund/Cancel

        let refund_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(escrow_pda, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(system_program, false),
            ],
            data: refund_data,
        };

        let message = Message::new(&[refund_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        let tx = svm.send_transaction(transaction).unwrap();
        println!(
            "Refund transaction successful — CUs: {}",
            tx.compute_units_consumed
        );

        // Verify maker got tokens back
        let maker_a_after = svm
            .get_account(&maker_ata_a)
            .expect("Maker ATA A should exist");
        let maker_a_after_amount =
            u64::from_le_bytes(maker_a_after.data[64..72].try_into().unwrap());
        assert_eq!(maker_a_after_amount, maker_a_before_amount + amount_to_give);
        println!(
            "Maker mint_a balance after refund: {}",
            maker_a_after_amount
        );

        // Verify vault is closed
        assert!(svm.get_account(&vault).is_none(), "Vault should be closed");

        // Verify escrow account is closed
        let escrow_acc = svm.get_account(&escrow_pda);
        assert!(
            escrow_acc.is_none() || escrow_acc.unwrap().lamports == 0,
            "Escrow should be closed"
        );

        println!("test_refund_instruction passed");
    }
}

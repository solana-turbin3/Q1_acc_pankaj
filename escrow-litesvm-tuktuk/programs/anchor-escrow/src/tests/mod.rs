#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            prelude::{msg, Clock},
            solana_program::program_pack::Pack,
            AccountDeserialize, InstructionData, ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{self, spl_associated_token_account},
            token::spl_token,
        },
        litesvm::{types::TransactionMetadata, LiteSVM},
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
        },
        solana_account::Account,
        solana_address::Address,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_rpc_client::rpc_client::RpcClient,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::{path::PathBuf, str::FromStr},
    };

    static PROGRAM_ID: Pubkey = crate::ID;

    // Setup function to initialize LiteSVM and create a payer keypair
    // Also loads an account from devnet into the LiteSVM environment (for testing purposes)
    fn setup() -> (LiteSVM, Keypair) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();

        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 50 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        // Load program SO file
        let so_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/anchor_escrow.so");

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // LiteSVM does not have access to real Solana network data since it does not have network access,
        // so we use an RPC client to fetch account data from devnet
        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address =
            Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        // Set the fetched account in the LiteSVM environment
        // This allows us to simulate interactions with this account during testing
        program
            .set_account(
                payer.pubkey(),
                Account {
                    lamports: fetched_account.lamports,
                    data: fetched_account.data,
                    owner: Pubkey::from(fetched_account.owner.to_bytes()),
                    executable: fetched_account.executable,
                    rent_epoch: fetched_account.rent_epoch,
                },
            )
            .unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);

        // Return the LiteSVM instance and payer keypair
        (program, payer)
    }

    fn setup_escrow_make(
        program: &mut LiteSVM,
        payer: &Keypair,
    ) -> (
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        TransactionMetadata,
    ) {
        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();

        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
        // This done using litesvm-token's CreateMint utility which creates the mint in the LiteSVM environment
        let mint_a = CreateMint::new(program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        // This is done using litesvm-token's CreateAssociatedTokenAccount utility
        let maker_ata_a = CreateAssociatedTokenAccount::new(program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 10,
                seed: 123u64,
                receive: 10,
                expiry: 100, // 100 seconds expiry
            }
            .data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        (maker, mint_a, mint_b, maker_ata_a, escrow, vault, tx)
    }
    #[test]
    fn test_make() {
        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        let (maker, mint_a, mint_b, _maker_ata_a, escrow, vault, tx) =
            setup_escrow_make(&mut program, &payer);
        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data =
            crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);
    }

    #[test]
    fn test_refund() {
        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        let (maker, mint_a, _mint_b, maker_ata_a, escrow, vault, _) =
            setup_escrow_make(&mut program, &payer);

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;
        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker,
                mint_a,
                maker_ata_a,
                escrow,
                vault,
                token_program,
                system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        msg!("Refund transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // vault should be closed
        let vault_acc = program.get_account(&vault).unwrap();
        assert_eq!(vault_acc.data.len(), 0);
        assert_eq!(vault_acc.lamports, 0);
        assert_eq!(vault_acc.owner, system_program);
        // maker must receive back tokens
        let maker_acc = program.get_account(&maker_ata_a).unwrap();
        let maker_state = spl_token::state::Account::unpack(&maker_acc.data).unwrap();
        assert_eq!(maker_state.amount, 1_000_000_000);

        // escrow should be closed
        let escrow_acc = program.get_account(&escrow).unwrap();
        assert_eq!(escrow_acc.data.len(), 0);
        assert_eq!(escrow_acc.lamports, 0);
        assert_eq!(escrow_acc.owner, system_program);
    }

    #[test]
    fn test_take_before_deadline_fails() {
        let (mut program, payer) = setup();

        let (maker, mint_a, mint_b, _maker_ata_a, escrow, vault, _) =
            setup_escrow_make(&mut program, &payer);

        let taker = Keypair::new();

        program
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 10)
            .send()
            .unwrap();

        // Define program IDs for associated token program, token program, and system program
        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let msg = Message::new(&[take_ix], Some(&payer.pubkey()));

        let transaction = Transaction::new(&[&payer, &taker], msg, program.latest_blockhash());

        // This test is expected to fail because of the time lock
        let tx_result = program.send_transaction(transaction);
        assert!(
            tx_result.is_err(),
            "Take transaction should fail before deadline"
        );
    }

    #[test]
    fn test_take_after_deadline_succeeds() {
        let (mut program, payer) = setup();

        let (maker, mint_a, mint_b, _maker_ata_a, escrow, vault, _) =
            setup_escrow_make(&mut program, &payer);

        let taker = Keypair::new();

        program
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 10)
            .send()
            .unwrap();

        // Time Travel
        let mut clock: Clock = program.get_sysvar();
        let current_time = clock.unix_timestamp;
        let five_days = 5 * 24 * 60 * 60 + 1; // 5 days in seconds
        let future_time = current_time + five_days;

        clock.unix_timestamp = future_time;
        program.set_sysvar(&clock);

        msg!("Travelled into future: {} -> {}", current_time, future_time);

        // Define program IDs for associated token program, token program, and system program
        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program,
                token_program,
                system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let msg = Message::new(&[take_ix], Some(&payer.pubkey()));

        let transaction = Transaction::new(&[&payer, &taker], msg, program.latest_blockhash());

        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nTake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        let taker_a = program.get_account(&taker_ata_a).unwrap();
        let taker_a_state = spl_token::state::Account::unpack(&taker_a.data).unwrap();
        assert_eq!(taker_a_state.amount, 10);

        let maker_b = program.get_account(&maker_ata_b).unwrap();
        let maker_b_state = spl_token::state::Account::unpack(&maker_b.data).unwrap();
        assert_eq!(maker_b_state.amount, 10);

        let vault_acc = program.get_account(&vault).unwrap();
        assert_eq!(vault_acc.data.len(), 0);
        assert_eq!(vault_acc.lamports, 0);
        assert_eq!(vault_acc.owner, system_program);

        let escrow_acc = program.get_account(&escrow).unwrap();
        assert_eq!(escrow_acc.data.len(), 0);
        assert_eq!(escrow_acc.lamports, 0);
        assert_eq!(escrow_acc.owner, system_program);
    }

    #[test]
    fn test_schedule() {
        let (mut program, payer) = setup();

        // Load Tuktuk Program from Devnet (Pubkey only)
        let tuktuk_str = "tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA";
        let tuktuk_program_id = Pubkey::from_str(tuktuk_str).unwrap();

        let (maker, mint_a, _mint_b, _maker_ata_a, escrow, _vault, _) =
            setup_escrow_make(&mut program, &payer);

        let task_queue = Keypair::new().pubkey();
        let task_queue_authority = Keypair::new().pubkey();
        let queue_authority = Pubkey::find_program_address(&[b"queue_authority"], &crate::ID).0;
        let task = Keypair::new().pubkey();

        let task_id = 1u16;

        let schedule_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Schedule {
                user: payer.pubkey(),
                escrow,
                task_queue,
                task_queue_authority,
                task,
                queue_authority,
                system_program: SYSTEM_PROGRAM_ID,
                tuktuk_program: tuktuk_program_id,
            }
            .to_account_metas(None),
            data: crate::instruction::Schedule { task_id }.data(),
        };

        let message = Message::new(&[schedule_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());

        let tx_result = program.send_transaction(transaction);

        match tx_result {
            Ok(tx) => {
                msg!("Schedule successful");
                msg!("Tx Signature: {}", tx.signature);
            }
            Err(e) => {
                msg!("Schedule failed: {:?}", e);
            }
        }
    }
}

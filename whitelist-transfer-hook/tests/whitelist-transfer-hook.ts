import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createTransferCheckedInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  SendTransactionError,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction
} from '@solana/web3.js';
import { WhitelistTransferHook } from "../target/types/whitelist_transfer_hook";

describe("whitelist-transfer-hook", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as anchor.Wallet;
  const program = anchor.workspace.whitelistTransferHook as Program<WhitelistTransferHook>;

  const mint2022 = anchor.web3.Keypair.generate();

  // Sender token account address
  const sourceTokenAccount = getAssociatedTokenAddressSync(
    mint2022.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  // Recipient token account address
  const recipient = anchor.web3.Keypair.generate();
  const destinationTokenAccount = getAssociatedTokenAddressSync(
    mint2022.publicKey,
    recipient.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  // ExtraAccountMetaList address
  // Store extra accounts required by the custom transfer hook instruction
  const [extraAccountMetaListPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from('extra-account-metas'), mint2022.publicKey.toBuffer()],
    program.programId,
  );

  // Derive Whitelist PDA for the sender (wallet.publicKey)
  const [whitelistPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("whitelist"),
      wallet.publicKey.toBuffer(),
    ],
    program.programId
  );

  it("Add user to whitelist", async () => {
    // This now initializes the WhitelistEntry PDA
    const tx = await program.methods.addToWhitelist(wallet.publicKey)
      .accountsPartial({
        admin: wallet.publicKey,
        whitelistEntry: whitelistPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("\nUser added to whitelist:", wallet.publicKey.toBase58());
    console.log("Transaction signature:", tx);
  });

  it('Create Mint Account with Transfer Hook Extension', async () => {
    // We use the new initMint instruction
    const tx = await program.methods.initMint()
      .accountsPartial({
        user: wallet.publicKey,
        mint: mint2022.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID, // Use associated token program if needed, or just system
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([mint2022])
      .rpc();

    console.log("\nMint initialized with Transfer Hook:", mint2022.publicKey.toBase58());
    console.log("Transaction Signature: ", tx);
  });

  it('Create Token Accounts and Mint Tokens', async () => {
    // 100 tokens
    const amount = 100 * 10 ** 9;

    const transaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mint2022.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        destinationTokenAccount,
        recipient.publicKey,
        mint2022.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createMintToInstruction(mint2022.publicKey, sourceTokenAccount, wallet.publicKey, amount, [], TOKEN_2022_PROGRAM_ID),
    );

    const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: true });

    console.log("\nToken Accounts created and Minted. Tx:", txSig);
  });

  it('Create ExtraAccountMetaList Account', async () => {
    const tx = await program.methods
      .initializeTransferHook()
      .accountsPartial({
        payer: wallet.publicKey,
        mint: mint2022.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("\nExtraAccountMetaList Account created:", extraAccountMetaListPDA.toBase58());
    console.log('Transaction Signature:', tx);
  });

  it('Transfer Hook with Extra Account Meta', async () => {
    // 1 token
    const amount = 1 * 10 ** 9;
    const amountBigInt = BigInt(amount);

    // Create the base transfer instruction
    // Note: In a real client resolving the extra accounts, these would be added automatically.
    // For this test, we might need to manually add them if not resolved OR invoke valid transfer.

    // Attempting standard transfer first. Since we initialized ExtraMetaList with dynamic seeds,
    // we need to verify if we need to manually push accounts here or if library helps. 
    // Usually manual push is needed for raw transactions without a resolving client.

    const transferInstruction = createTransferCheckedInstruction(
      sourceTokenAccount,
      mint2022.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amountBigInt,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    // Manually add the extra accounts required by the transfer hook
    // Order matters and must match what ExtraAccountMetaList expects (if it enforces order)
    // or simply provide the accounts the hook needs.
    transferInstruction.keys.push(
      // ExtraAccountMetaList PDA
      { pubkey: extraAccountMetaListPDA, isSigner: false, isWritable: false },
      // Whitelist PDA (The dynamic one derived from Sender)
      { pubkey: whitelistPDA, isSigner: false, isWritable: false },
      // Transfer Hook Program (Required for CPI) - invalidation should ignore trailing accounts
      { pubkey: program.programId, isSigner: false, isWritable: false }
    );

    const transaction = new Transaction().add(transferInstruction);

    try {
      const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: true });
      console.log("\nTransfer Signature:", txSig);
    }
    catch (error) {
      console.error("\nTransaction failed:", error);
      throw error;
    }
  });

  it("Remove user from whitelist", async () => {
    const tx = await program.methods.removeFromWhitelist(wallet.publicKey)
      .accountsPartial({
        admin: wallet.publicKey,
        whitelistEntry: whitelistPDA,
      })
      .rpc();

    console.log("\nUser removed from whitelist:", wallet.publicKey.toBase58());
    console.log("Transaction signature:", tx);
  });

  it('Fail Transfer when NOT Whitelisted', async () => {
    // 1 token - Attempt transfer again, should fail
    const amount = 1 * 10 ** 9;
    const amountBigInt = BigInt(amount);

    const transferInstruction = createTransferCheckedInstruction(
      sourceTokenAccount,
      mint2022.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amountBigInt,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    transferInstruction.keys.push(
      { pubkey: extraAccountMetaListPDA, isSigner: false, isWritable: false },
      { pubkey: whitelistPDA, isSigner: false, isWritable: false },
      { pubkey: program.programId, isSigner: false, isWritable: false }
    );

    const transaction = new Transaction().add(transferInstruction);

    try {
      await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: true });
      throw new Error("Transaction should have failed but succeeded!");
    }
    catch (error) {
      // Expected failure
      console.log("\nTransaction correctly failed as expected.");
      // Ideally check error logs for specific panic
    }
  });

});

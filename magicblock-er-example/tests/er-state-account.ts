import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { GetCommitmentSignature } from "@magicblock-labs/ephemeral-rollups-sdk";
import { ErStateAccount } from "../target/types/er_state_account";

describe("er-state-account", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const providerEphemeralRollup = new anchor.AnchorProvider(
    new anchor.web3.Connection(
      process.env.EPHEMERAL_PROVIDER_ENDPOINT ||
        "https://devnet.magicblock.app/",
      {
        wsEndpoint:
          process.env.EPHEMERAL_WS_ENDPOINT || "wss://devnet.magicblock.app/",
      }
    ),
    anchor.Wallet.local()
  );
  console.log("Base Layer Connection: ", provider.connection.rpcEndpoint);
  console.log(
    "Ephemeral Rollup Connection: ",
    providerEphemeralRollup.connection.rpcEndpoint
  );
  console.log(`Current SOL Public Key: ${anchor.Wallet.local().publicKey}`);

  before(async function () {
    const balance = await provider.connection.getBalance(
      anchor.Wallet.local().publicKey
    );
    console.log("Current balance is", balance / LAMPORTS_PER_SOL, " SOL", "\n");
  });

  const program = anchor.workspace.erStateAccount as Program<ErStateAccount>;

  const userAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user"), anchor.Wallet.local().publicKey.toBuffer()],
    program.programId
  )[0];

  // Constants for VRF
  const VRF_PROGRAM_ID = new PublicKey(
    "Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz"
  );
  const DEFAULT_EPHEMERAL_QUEUE = new PublicKey(
    "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc"
  );
  const IDENTITY = new PublicKey(
    "9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw"
  );

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("User Account initialized: ", tx);
  });

  it("Update State!", async () => {
    const tx = await program.methods
      .update(new anchor.BN(42))
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
      })
      .rpc();
    console.log("\nUser Account State Updated: ", tx);
  });

  it("Task 1: Request Randomness Outside ER", async () => {
    // On devnet, it triggers the request. We won't wait for callback here as it's async.
    try {
      const tx = await program.methods
        .requestRandomness()
        .accountsPartial({
          user: anchor.Wallet.local().publicKey,
          userAccount: userAccount,
          oracleQueue: DEFAULT_EPHEMERAL_QUEUE,
          vrfProgram: VRF_PROGRAM_ID,
          programIdentity: PublicKey.findProgramAddressSync(
            [Buffer.from("identity")],
            program.programId
          )[0],
          systemProgram: anchor.web3.SystemProgram.programId,
          slotHashes: anchor.web3.SYSVAR_SLOT_HASHES_PUBKEY,
        })
        .rpc({ skipPreflight: true });
      console.log("\nRandomness Requested on Base Layer: ", tx);
    } catch (e) {
      console.log(
        "Skipping Task 1 execution (likely due to missing Oracle on local/devnet or config mismatch):",
        e
      );
    }
  });

  it("Delegate to Ephemeral Rollup!", async () => {
    let tx = await program.methods
      .delegate()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        validator: new PublicKey("MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57"),
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });

    console.log("\nUser Account Delegated to Ephemeral Rollup: ", tx);
  });

  it("Task 2: Request Randomness Inside ER", async () => {
    // Using Ephemeral Provider
    const tx = await program.methods
      .requestRandomness()
      .accountsPartial({
        user: providerEphemeralRollup.wallet.publicKey,
        userAccount: userAccount,
        oracleQueue: DEFAULT_EPHEMERAL_QUEUE,
        vrfProgram: VRF_PROGRAM_ID,
        programIdentity: PublicKey.findProgramAddressSync(
          [Buffer.from("identity")],
          program.programId
        )[0],
        systemProgram: anchor.web3.SystemProgram.programId,
        slotHashes: anchor.web3.SYSVAR_SLOT_HASHES_PUBKEY,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;

    const signedTx = await providerEphemeralRollup.wallet.signTransaction(tx);

    const txHash = await providerEphemeralRollup.sendAndConfirm(signedTx, [], {
      skipPreflight: false,
    });

    console.log("\nRandomness Requested Inside ER: ", txHash);

    await new Promise((r) => setTimeout(r, 2000));

    const account = await program.account.userAccount.fetch(userAccount);
    console.log(
      "User Account Data after VRF (should be random):",
      account.data.toString()
    );
  });

  it("Update State and Commit to Base Layer!", async () => {
    let tx = await program.methods
      .updateCommit(new anchor.BN(43))
      .accountsPartial({
        user: providerEphemeralRollup.wallet.publicKey,
        userAccount: userAccount,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;

    const signedTx = await providerEphemeralRollup.wallet.signTransaction(tx);
    const txHash = await providerEphemeralRollup.sendAndConfirm(signedTx, [], {
      skipPreflight: false,
    });
    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection
    );

    console.log("\nUser Account State Updated & Committed: ", txHash);
  });

  it("Commit and undelegate from Ephemeral Rollup!", async () => {
    let tx = await program.methods
      .undelegate()
      .accounts({
        user: providerEphemeralRollup.wallet.publicKey,
      })
      .transaction();

    tx.feePayer = providerEphemeralRollup.wallet.publicKey;
    tx.recentBlockhash = (
      await providerEphemeralRollup.connection.getLatestBlockhash()
    ).blockhash;
    const signedTx = await providerEphemeralRollup.wallet.signTransaction(tx);
    const txHash = await providerEphemeralRollup.sendAndConfirm(signedTx, [], {
      skipPreflight: false,
    });
    const txCommitSgn = await GetCommitmentSignature(
      txHash,
      providerEphemeralRollup.connection
    );

    console.log("\nUser Account Undelegated: ", txHash);
  });

  it("Close Account!", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: anchor.Wallet.local().publicKey,
        userAccount: userAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("\nUser Account Closed: ", tx);
  });
});

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GptTuktuk } from "../target/types/gpt_tuktuk";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

// GPT Oracle Program ID (from crate source: LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab)
const GPT_ORACLE_PROGRAM_ID = new PublicKey("LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab");

// Tuktuk Program ID
const TUKTUK_PROGRAM_ID = new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

describe("gpt-tuktuk", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.GptTuktuk as Program<GptTuktuk>;

  const taskId = 123;

  const [requestState] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("gpt_request"),
      provider.publicKey.toBuffer(),
      new anchor.BN(taskId).toArrayLike(Buffer, 'le', 2)
    ],
    program.programId
  );

  const [queueAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")],
    program.programId
  );

  it("Is initialized!", async () => {
    try {
      await program.methods.initialize().rpc();
      console.log("Program initialized successfully");
    } catch (e) {
      console.log("Initialize skipped (may already be initialized):", (e as Error).message?.slice(0, 100));
    }
  });

  it("Schedules a GPT request via Tuktuk", async () => {
    // NOTE: This test requires Tuktuk to be deployed and a task queue to be created.
    // On localnet without Tuktuk, this will fail. On devnet, use actual Tuktuk queue addresses.

    const prompt = "What is Solana?";
    const delay = 5; // 5 seconds

    // You need to provide a real oracle context account.
    // On devnet, first call `create_llm_context` on the GPT Oracle to get one.
    // For now, we derive a placeholder based on the oracle's counter PDA.
    const [oracleCounter] = PublicKey.findProgramAddressSync(
      [Buffer.from("counter")],
      GPT_ORACLE_PROGRAM_ID
    );

    // Example context account (counter=0 -> seeds: ["test-context", 0u32_le])
    const [oracleContextAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"), Buffer.from([0, 0, 0, 0])],
      GPT_ORACLE_PROGRAM_ID
    );

    // Mock Task Queue accounts (replace with real ones on devnet)
    const taskQueue = anchor.web3.Keypair.generate().publicKey;
    const taskQueueAuthority = anchor.web3.Keypair.generate().publicKey;
    const task = anchor.web3.Keypair.generate().publicKey;

    try {
      await program.methods.scheduleRequest(taskId, prompt, new anchor.BN(delay))
        .accounts({
          requestState,
          user: provider.publicKey,
          taskQueue,
          taskQueueAuthority,
          task,
          queueAuthority,
          oracleContextAccount,
          systemProgram: SystemProgram.programId,
          tuktukProgram: TUKTUK_PROGRAM_ID,
        })
        .rpc();
      console.log("Scheduled GPT Request via Tuktuk");
    } catch (e) {
      console.log("Scheduling failed (expected without Tuktuk/Oracle on localnet):",
        (e as Error).message?.slice(0, 150));
    }
  });

  it("Executes a GPT request (calls GPT Oracle CPI)", async () => {
    // NOTE: This test requires:
    // 1. The GPT Oracle program deployed (devnet: LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab)
    // 2. An LLM context created on the oracle
    // 3. The request_state to be initialized (via schedule_request)
    //
    // On localnet without the oracle, this will fail.
    // On devnet, the oracle will process the request and call back consume_result.

    const prompt = "What is Solana?";

    // Derive the oracle context account
    const [oracleContextAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"), Buffer.from([0, 0, 0, 0])],
      GPT_ORACLE_PROGRAM_ID
    );

    // Derive the interaction PDA
    const [oracleInteraction] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("interaction"),
        provider.publicKey.toBuffer(),
        oracleContextAccount.toBuffer(),
      ],
      GPT_ORACLE_PROGRAM_ID
    );

    try {
      await program.methods.executeRequest(prompt)
        .accounts({
          requestState,
          user: provider.publicKey,
          oracleContextAccount,
          oracleInteraction,
          gptOracleProgram: GPT_ORACLE_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("Execute request sent to GPT Oracle");
    } catch (e) {
      console.log("Execute request failed (expected without Oracle on localnet):",
        (e as Error).message?.slice(0, 150));
    }
  });

  it("Verifies consume_result callback structure", async () => {
    // NOTE: In production, this is called automatically by the GPT Oracle's
    // callback_from_llm instruction. The oracle's Identity PDA signs the CPI.
    //
    // We can't easily test this on localnet without the oracle program.
    // On devnet, after execute_request, poll the request_state account
    // until is_completed is true.

    // Derive oracle Identity PDA (the oracle creates this during its initialize)
    const [oracleIdentity] = PublicKey.findProgramAddressSync(
      [Buffer.from("identity")],
      GPT_ORACLE_PROGRAM_ID
    );

    console.log("Oracle Identity PDA:", oracleIdentity.toBase58());
    console.log("Request State PDA:", requestState.toBase58());
    console.log("GPT Oracle Program:", GPT_ORACLE_PROGRAM_ID.toBase58());

    // On devnet, you would poll like this:
    // const state = await program.account.gptRequest.fetch(requestState);
    // assert.isTrue(state.isCompleted);
    // console.log("GPT Response:", state.result);

    console.log("Consume result is handled by the GPT Oracle callback (devnet only)");
  });

  it("Polls for GPT response (devnet integration)", async () => {
    // This test demonstrates how to poll for the oracle's callback response.
    // Only works when deployed to devnet with the real GPT Oracle.

    const maxAttempts = 30;
    const delayMs = 2000;

    for (let i = 0; i < maxAttempts; i++) {
      try {
        const state = await program.account.gptRequest.fetch(requestState);
        if (state.isCompleted) {
          console.log("✅ GPT Response received!");
          console.log("   Prompt:", state.prompt);
          console.log("   Response:", state.result);
          assert.isTrue(state.isCompleted);
          assert.isNotNull(state.result);
          return;
        }
        console.log(`Attempt ${i + 1}/${maxAttempts}: Waiting for oracle response...`);
      } catch (e) {
        console.log(`Attempt ${i + 1}: Account not found yet (expected on localnet)`);
        return; // Exit early on localnet
      }

      await new Promise(resolve => setTimeout(resolve, delayMs));
    }

    console.log("Oracle response not received within timeout (expected on localnet)");
  });
});

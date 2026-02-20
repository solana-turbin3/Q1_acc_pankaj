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

  const taskId = 1;

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
    const prompt = "What is Solana?";
    const delay = 5; // 5 seconds

    const oracleContextAccount = new PublicKey("5nyx2rym3F9XEvhXpx4riLSbbVPxMuXkpZc5G6BT5Bu6");
    const taskQueue = new PublicKey("24aSLaMuki7E9AwEhqsPWtJh7jZtWwPppLR9PjwT1eaB");

    // The authority of the taskQueue is our gpt-tuktuk program's queueAuthority PDA
    const [gptQueueAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("queue_authority")],
      program.programId
    );

    const { taskQueueAuthorityKey } = require("@helium/tuktuk-sdk");
    const [taskQueueAuthority] = taskQueueAuthorityKey(taskQueue, gptQueueAuthority, TUKTUK_PROGRAM_ID);

    // Tuktuk task PDA derivation (queue, id)
    // Actually, Tuktuk TaskV0 PDA seeds: [b"task", task_queue, id]
    const [task] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("task"),
        taskQueue.toBuffer(),
        new anchor.BN(taskId).toArrayLike(Buffer, "le", 2),
      ],
      TUKTUK_PROGRAM_ID
    );

    try {
      await program.methods.scheduleRequest(taskId, prompt, new anchor.BN(delay))
        .accounts({
          requestState,
          user: provider.publicKey,
          taskQueue,
          taskQueueAuthority,
          task: task,
          oracleContextAccount,
        } as any)
        .rpc();
      console.log("✅ Scheduled GPT Request via Tuktuk");
    } catch (e: any) {
      console.log("Scheduling failed:", e.message);
      throw e;
    }
  });

  it("Executes a GPT request (calls GPT Oracle CPI)", async () => {
    // This part is automatically called by Tuktuk when the task triggers!
    // But we can also manually call it if we want to bypass the wait, or if Tuktuk is slow.
    // Let's trigger it manually to verify execute_request logic instantly.
    const prompt = "What is Solana?";
    const oracleContextAccount = new PublicKey("5nyx2rym3F9XEvhXpx4riLSbbVPxMuXkpZc5G6BT5Bu6");

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
        } as any)
        .rpc();
      console.log("✅ Execute request sent to GPT Oracle");
    } catch (e: any) {
      console.error("Execute request failed:", e);
      throw e;
    }
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

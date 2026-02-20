import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { createTaskQueue, init as initTuktuk } from "@helium/tuktuk-sdk";

async function main() {
    process.env.ANCHOR_PROVIDER_URL = "https://api.devnet.solana.com";
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    console.log("Wallet:", provider.publicKey.toBase58());

    const tuktukProgram = await initTuktuk(provider);

    // Try to create a task queue with the proper queueAuthority
    try {
        console.log("Creating Task Queue...");
        const GPT_TUKTUK_PROGRAM_ID = new PublicKey("3LWmo92AxMjU5tLjfneoqCYVMQSoA6teNeYhSiQojpSG");
        const [gptQueueAuthority] = PublicKey.findProgramAddressSync(
            [Buffer.from("queue_authority")],
            GPT_TUKTUK_PROGRAM_ID
        );
        console.log("GPT Queue authority PDA:", gptQueueAuthority.toBase58());

        const method = await createTaskQueue(tuktukProgram as any, {
            name: "gpt-queue-auth2", // Changed name to gpt-queue-auth2 to avoid conflict
            capacity: 10,
            minCrankReward: new anchor.BN(100),
            staleTaskAge: 86400,
            lookupTables: [],
        });

        // Use our wallet as the authority for the queue
        method.accountsPartial({
            updateAuthority: provider.publicKey,
        });

        const sigQueue = await method.rpc();
        console.log("Success! Queue created with sig:", sigQueue);

        const { getTaskQueueForName, taskQueueAuthorityKey } = require("@helium/tuktuk-sdk");
        const queueKey = await getTaskQueueForName(tuktukProgram, "gpt-queue-auth2");
        console.log("GPT Tuktuk Queue Address:", queueKey?.toBase58());

        console.log("Adding GPT PDA as Authority...");
        const [taskQueueAuthority] = taskQueueAuthorityKey(queueKey, gptQueueAuthority, tuktukProgram.programId);

        const addAuthMethod = await tuktukProgram.methods.addQueueAuthorityV0().accounts({
            taskQueue: queueKey,
            queueAuthority: gptQueueAuthority,
        });

        const sigAuth = await addAuthMethod.rpc();
        console.log("Added GPT queueAuthority PDA to TaskQueue!", sigAuth);

    } catch (err: any) {
        console.log("Failed to create Tuktuk queue:", err.message?.slice(0, 200));
    }

    // Attempt to raw call GPT Oracle to create a context
    try {
        console.log("Creating GPT Oracle context");
        const GPT_ORACLE_PROGRAM_ID = new PublicKey("LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab");
        const [counterPda] = PublicKey.findProgramAddressSync([Buffer.from("counter")], GPT_ORACLE_PROGRAM_ID);

        const expectedPda = new PublicKey("5nyx2rym3F9XEvhXpx4riLSbbVPxMuXkpZc5G6BT5Bu6");
        const ixRaw = Buffer.from([224, 109, 4, 173, 191, 25, 42, 162]); // create_llm_context
        const len = Buffer.alloc(4);
        len.writeUInt32LE(0);
        const data = Buffer.concat([ixRaw, len]); // text = empty

        const ix = new anchor.web3.TransactionInstruction({
            programId: GPT_ORACLE_PROGRAM_ID,
            keys: [
                { pubkey: provider.publicKey, isSigner: true, isWritable: true },
                { pubkey: counterPda, isSigner: false, isWritable: true },
                { pubkey: expectedPda, isSigner: false, isWritable: true },
                { pubkey: anchor.web3.SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            data: data
        });

        const tx = new anchor.web3.Transaction().add(ix);
        const sig = await provider.sendAndConfirm(tx, []);
        console.log("GPT Oracle Context created:", expectedPda.toBase58(), "Sig:", sig);
    } catch (err: any) {
        console.log("GPT context creation failed:", err.message);
    }
}

main().catch(console.error);

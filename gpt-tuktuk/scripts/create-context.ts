import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

async function main() {
    process.env.ANCHOR_PROVIDER_URL = "https://api.devnet.solana.com";
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const GPT_ORACLE = new PublicKey("LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab");
    const [counterPda] = PublicKey.findProgramAddressSync([Buffer.from("counter")], GPT_ORACLE);

    // Read current counter
    const info = await provider.connection.getAccountInfo(counterPda);
    if (!info) throw new Error("Counter not found");
    const count = info.data.readUInt32LE(8);
    console.log("Current counter:", count);

    // Derive context PDA for the next counter value
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(count);
    const [contextPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("test-context"), buf],
        GPT_ORACLE
    );
    console.log("Next context PDA:", contextPda.toBase58());

    // Build create_llm_context instruction
    const discriminator = Buffer.from([224, 109, 4, 173, 191, 25, 42, 162]);
    const text = "GPT Oracle for Solana";
    const textLen = Buffer.alloc(4);
    textLen.writeUInt32LE(text.length);
    const data = Buffer.concat([discriminator, textLen, Buffer.from(text)]);

    const ix = new anchor.web3.TransactionInstruction({
        programId: GPT_ORACLE,
        keys: [
            { pubkey: provider.publicKey, isSigner: true, isWritable: true },
            { pubkey: counterPda, isSigner: false, isWritable: true },
            { pubkey: contextPda, isSigner: false, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        data: data,
    });

    const tx = new anchor.web3.Transaction().add(ix);
    const sig = await provider.sendAndConfirm(tx, []);
    console.log("Created context! Sig:", sig);
    console.log("New context account:", contextPda.toBase58());

    // Derive interaction PDA
    const [interactionPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("interaction"), provider.publicKey.toBuffer(), contextPda.toBuffer()],
        GPT_ORACLE
    );
    console.log("Interaction PDA:", interactionPda.toBase58());
}

main().catch(console.error);

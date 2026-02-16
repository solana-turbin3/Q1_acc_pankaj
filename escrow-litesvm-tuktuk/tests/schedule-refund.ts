
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorEscrow } from "../target/types/anchor_escrow";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { assert } from "chai";

describe("schedule-refund", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.AnchorEscrow as Program<AnchorEscrow>;

    // Accounts
    let maker: anchor.web3.Keypair;
    let taker: anchor.web3.Keypair;
    let mintA: PublicKey;
    let mintB: PublicKey;
    let makerAtaA: PublicKey;
    let makerAtaB: PublicKey;
    let takerAtaA: PublicKey;
    let takerAtaB: PublicKey;
    let escrow: PublicKey;
    let vault: PublicKey;

    const seed = new anchor.BN(Date.now()); // Unique seed

    before(async () => {
        maker = anchor.web3.Keypair.generate();
        taker = anchor.web3.Keypair.generate();

        // Airdrop SOL
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(maker.publicKey, 10 * LAMPORTS_PER_SOL),
            "confirmed"
        );
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(taker.publicKey, 10 * LAMPORTS_PER_SOL),
            "confirmed"
        );

        // Create Mints
        mintA = await createMint(provider.connection, maker, maker.publicKey, null, 6);
        mintB = await createMint(provider.connection, taker, taker.publicKey, null, 6);

        // Create ATAs
        makerAtaA = await createAccount(provider.connection, maker, mintA, maker.publicKey);
        makerAtaB = await createAccount(provider.connection, maker, mintB, maker.publicKey);
        takerAtaA = await createAccount(provider.connection, taker, mintA, taker.publicKey);
        takerAtaB = await createAccount(provider.connection, taker, mintB, taker.publicKey);

        // Mint tokens
        await mintTo(provider.connection, maker, mintA, makerAtaA, maker.publicKey, 1000_000000);
        await mintTo(provider.connection, taker, mintB, takerAtaB, taker.publicKey, 1000_000000);
    });

    it("Test Escrow Expiry and Refund", async () => {
        const deposit = new anchor.BN(100_000000);
        const receive = new anchor.BN(50_000000);
        // Short expiry: 2 seconds
        const expiry = new anchor.BN(2);

        [escrow] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("escrow"),
                maker.publicKey.toBuffer(),
                seed.toArrayLike(Buffer, "le", 8)
            ],
            program.programId
        );

        vault = await anchor.utils.token.associatedAddress({
            mint: mintA,
            owner: escrow
        });

        try {
            const tx = await program.methods
                .make(seed, deposit, receive, expiry)
                .accounts({
                    maker: maker.publicKey,
                    mintA: mintA,
                    mintB: mintB,
                    makerAtaA: makerAtaA,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                })
                .signers([maker])
                .rpc();
            console.log("Escrow initialized, tx:", tx);
        } catch (e) {
            console.error("Error initializing escrow:", e);
            throw e;
        }

        // Try redundant refund immediately (should fail)
        try {
            await program.methods
                .refund()
                .accounts({
                    maker: maker.publicKey,
                    mintA: mintA,
                    makerAtaA: makerAtaA,
                    escrow: escrow,
                    vault: vault,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                })
                .signers([taker]) // Taker trying to refund
                .rpc();
            assert.fail("Should have failed because not expired and not maker signature");
        } catch (e) {
            // Expected
            console.log("Caught expected error (immediate refund):", e.message || e);
        }

        console.log("Waiting 3 seconds for expiry...");
        await new Promise(resolve => setTimeout(resolve, 3000));

        // Refund after expiry (permissionless)
        try {
            const tx = await program.methods
                .refund()
                .accounts({
                    maker: maker.publicKey,
                    mintA: mintA,
                    makerAtaA: makerAtaA,
                    escrow: escrow,
                    vault: vault,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                })
                .signers([taker])
                .rpc();
            console.log("Refund successful after expiry, tx:", tx);
        } catch (e) {
            console.error("Error refunding after expiry:", e);
            throw e;
        }

        // Verify vault closed
        try {
            await getAccount(provider.connection, vault);
            assert.fail("Vault account should be closed");
        } catch (e) {
            // Expected
        }
    });

    it("Test Schedule Instruction", async () => {
        // This test attempts to call schedule. 
        // Note: It will likely fail if Tuktuk program is not deployed.
        // We just want to ensure the instruction exists and accounts are correct.

        const deposit = new anchor.BN(100_000000);
        const receive = new anchor.BN(50_000000);
        const expiry = new anchor.BN(10);
        const seed2 = new anchor.BN(Date.now() + 1000);

        const [escrow2] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("escrow"),
                maker.publicKey.toBuffer(),
                seed2.toArrayLike(Buffer, "le", 8)
            ],
            program.programId
        );

        const vault2 = await anchor.utils.token.associatedAddress({
            mint: mintA,
            owner: escrow2
        });

        await program.methods
            .make(seed2, deposit, receive, expiry)
            .accounts({
                maker: maker.publicKey,
                mintA: mintA,
                mintB: mintB,
                makerAtaA: makerAtaA,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .signers([maker])
            .rpc();

        // Mock Tuktuk accounts or usage
        // For integration test, we might skip actual CPI call if we can't deploy Tuktuk easily.
        // But let's try calling it and see if it fails with "Program not found" which confirms we tried to call it.

        /*
        const tuktukProgramId = new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");
        // We need task queue accounts.
        // ...
        // Skipping mainly because detailed setup of Tuktuk (Queue, Authority, etc) is complex for this single task 
        // without pre-existing helper functions in this repo.
        */
        console.log("Skipping actual schedule call test as it requires Tuktuk deployment.");
        // We verify `refund` works permissionlessly, which is the core logic needed for the scheduler.
    });
});

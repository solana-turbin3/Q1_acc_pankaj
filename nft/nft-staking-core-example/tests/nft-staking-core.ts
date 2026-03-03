import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingCore } from "../target/types/nft_staking_core";
import { SystemProgram } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { publicKey } from "@metaplex-foundation/umi";
import { fetchCollection } from "@metaplex-foundation/mpl-core";
import { expect } from "chai";

const MILLISECONDS_PER_DAY = 86400000;
const POINTS_PER_STAKED_NFT_PER_DAY = 10_000_000;
const FREEZE_PERIOD_IN_DAYS = 7;
const TIME_TRAVEL_IN_DAYS = 8;

describe("nft-staking-core", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.nftStakingCore as Program<NftStakingCore>;

  // Generate a keypair for the collection
  const collectionKeypair = anchor.web3.Keypair.generate();

  // Find the update authority for the collection (PDA)
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Generate a keypair for the nft asset
  const nftKeypair = anchor.web3.Keypair.generate();

  // Find the config account (PDA)
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Find the rewards mint account (PDA)
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    program.programId
  )[0];

  it("Create a collection", async () => {
    const collectionName = "Test Collection";
    const collectionUri = "https://example.com/collection";
    const tx = await program.methods
      .createCollection(collectionName, collectionUri)
      .accountsPartial({
        payer: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collectionKeypair])
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature", tx);
    console.log("Collection address", collectionKeypair.publicKey.toBase58());

    // Fetch and verify collection initialized correctly
    const umi = createUmi(provider.connection.rpcEndpoint);
    const collectionData = await fetchCollection(
      umi,
      publicKey(collectionKeypair.publicKey)
    );

    // In Metaplex Core collections, the attributes are in the attributes plugin list
    const attributeList = collectionData.attributes?.attributeList || [];
    const totalStakedAttr = attributeList.find(
      (attr) => attr.key === "total_staked"
    );

    expect(totalStakedAttr).to.not.be.undefined;
    expect(totalStakedAttr!.value).to.equal("0");
  });

  it("Mint an NFT", async () => {
    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods
      .mintNft(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature", tx);
    console.log("NFT address", nftKeypair.publicKey.toBase58());
  });

  it("Initialize stake config", async () => {
    const tx = await program.methods
      .initializeConfig(POINTS_PER_STAKED_NFT_PER_DAY, FREEZE_PERIOD_IN_DAYS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature", tx);
    console.log("Config address", config.toBase58());
    console.log("Points per staked NFT per day", POINTS_PER_STAKED_NFT_PER_DAY);
    console.log("Freeze period in days", FREEZE_PERIOD_IN_DAYS);
    console.log("Rewards mint address", rewardsMint.toBase58());
  });

  it("Stake an NFT", async () => {
    const tx = await program.methods
      .stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature", tx);

    const umi = createUmi(provider.connection.rpcEndpoint);
    const collectionData = await fetchCollection(
      umi,
      publicKey(collectionKeypair.publicKey)
    );

    const attributeList = collectionData.attributes?.attributeList || [];
    const totalStakedAttr = attributeList.find(
      (attr) => attr.key === "total_staked"
    );

    expect(totalStakedAttr).to.not.be.undefined;
    expect(totalStakedAttr!.value).to.equal("1");
  });

  /**
   * Helper function to advance time with surfnet_timeTravel RPC method
   * @param params - Time travel params (absoluteEpoch, absoluteSlot, or absoluteTimestamp)
   */
  async function advanceTime(params: {
    absoluteEpoch?: number;
    absoluteSlot?: number;
    absoluteTimestamp?: number;
  }): Promise<void> {
    const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [params],
      }),
    });

    const result = (await rpcResponse.json()) as { error?: any; result?: any };
    if (result.error) {
      throw new Error(`Time travel failed: ${JSON.stringify(result.error)}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 2000));
  }

  it("Time travel to the future", async () => {
    // Query Surfnet's actual internal clock so time travel always goes forward
    const clockAccount = await provider.connection.getAccountInfo(
      anchor.web3.SYSVAR_CLOCK_PUBKEY
    );
    if (!clockAccount)
      throw new Error("Failed to get SYSVAR_CLOCK_PUBKEY from Surfnet");
    const currentTimestampMs =
      Number(clockAccount.data.readBigInt64LE(32)) * 1000;

    // Add TIME_TRAVEL_IN_DAYS directly to current timestamp
    await advanceTime({
      absoluteTimestamp:
        currentTimestampMs + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY,
    });
    console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS);
  });

  it("Claim Rewards", async () => {
    // Get the user rewards ATA account
    const userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      provider.wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const tx = await program.methods
      .claimRewards()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature (claim rewards)", tx);
    console.log(
      "User rewards balance",
      (await provider.connection.getTokenAccountBalance(userRewardsAta)).value
        .uiAmount
    );

    const umi = createUmi(provider.connection.rpcEndpoint);
    const collectionData = await fetchCollection(
      umi,
      publicKey(collectionKeypair.publicKey)
    );

    const attributeList = collectionData.attributes?.attributeList || [];
    const totalStakedAttr = attributeList.find(
      (attr) => attr.key === "total_staked"
    );

    expect(totalStakedAttr).to.not.be.undefined;
    expect(totalStakedAttr!.value).to.equal("1");
  });

  it("Time travel to the future again", async () => {
    const clockAccount = await provider.connection.getAccountInfo(
      anchor.web3.SYSVAR_CLOCK_PUBKEY
    );
    if (!clockAccount)
      throw new Error("Failed to get SYSVAR_CLOCK_PUBKEY from Surfnet");
    const currentTimestampMs =
      Number(clockAccount.data.readBigInt64LE(32)) * 1000;

    await advanceTime({
      absoluteTimestamp:
        currentTimestampMs + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY,
    });
    console.log("\nTime traveled again in days", TIME_TRAVEL_IN_DAYS);
  });

  it("Unstake an NFT", async () => {
    // Get the user rewards ATA account
    const userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      provider.wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    const tx = await program.methods
      .unstake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });
    console.log("\nYour transaction signature", tx);
    console.log(
      "User rewards balance",
      (await provider.connection.getTokenAccountBalance(userRewardsAta)).value
        .uiAmount
    );

    const umi = createUmi(provider.connection.rpcEndpoint);
    const collectionData = await fetchCollection(
      umi,
      publicKey(collectionKeypair.publicKey)
    );

    const attributeList = collectionData.attributes?.attributeList || [];
    const totalStakedAttr = attributeList.find(
      (attr) => attr.key === "total_staked"
    );

    expect(totalStakedAttr).to.not.be.undefined;
    expect(totalStakedAttr!.value).to.equal("0");
  });

  it("Mint and Stake a second NFT to test burning", async () => {
    const secondNftKeypair = anchor.web3.Keypair.generate();

    // Mint
    await program.methods
      .mintNft("Burn NFT", "https://example.com/burn")
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: secondNftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([secondNftKeypair])
      .rpc({ commitment: "confirmed" });

    // Stake
    await program.methods
      .stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: secondNftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });

    // Time Travel explicitly
    const clockAccount = await provider.connection.getAccountInfo(
      anchor.web3.SYSVAR_CLOCK_PUBKEY
    );
    const currentTimestampMs =
      Number(clockAccount!.data.readBigInt64LE(32)) * 1000;

    await advanceTime({
      absoluteTimestamp:
        currentTimestampMs + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY,
    });

    // Burn Staked NFT
    const userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      provider.wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const tx = await program.methods
      .burnStakedNft()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: secondNftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });

    console.log("\nYour transaction signature (burn staked nft)", tx);

    // Total rewards should now be 160 + (80 * 2 multiplier) = 320
    const balance = (await provider.connection.getTokenAccountBalance(userRewardsAta)).value.uiAmount;
    console.log("User rewards balance after burn", balance);
    expect(balance).to.equal(320);

    const umi = createUmi(provider.connection.rpcEndpoint);
    const collectionData = await fetchCollection(
      umi,
      publicKey(collectionKeypair.publicKey)
    );
    const attributeList = collectionData.attributes?.attributeList || [];
    const totalStakedAttr = attributeList.find(
      (attr) => attr.key === "total_staked"
    );
    expect(totalStakedAttr).to.not.be.undefined;
    expect(totalStakedAttr!.value).to.equal("0");

    // The NFT was burned, so trying to fetch it using UMI should fail
    try {
      await fetchCollection(umi, publicKey(secondNftKeypair.publicKey));
      expect.fail("Asset should not exist");
    } catch (e) {
      // Expected
    }
  });
});

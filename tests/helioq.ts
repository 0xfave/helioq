import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Helioq } from "../target/types/helioq";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("helioq", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  
  const program = anchor.workspace.Helioq as Program<Helioq>;
  
  const adminKeypair = Keypair.generate();
  const serverKeypair = Keypair.generate();
  const serverId = "test-server-1";

  before(async () => {
    // Fund admin account with SOL
    const signature = await provider.connection.requestAirdrop(
      adminKeypair.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);
  });

  it("Initialize admin account", async () => {
    const tx = await program.methods
      .initialize()
      .accounts({
        adminAccount: adminKeypair.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([adminKeypair])
      .rpc();

    const account = await program.account.adminAccount.fetch(adminKeypair.publicKey);
    expect(account.authority.toString()).to.equal(provider.wallet.publicKey.toString());
  });

  it("Register server", async () => {
    const tx = await program.methods
      .registerServer(serverId)
      .accounts({
        adminAccount: adminKeypair.publicKey,
        server: serverKeypair.publicKey,
        owner: provider.wallet.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([serverKeypair])
      .rpc();

    const serverData = await program.account.server.fetch(serverKeypair.publicKey);
    expect(serverData.id).to.equal(serverId);
    expect(serverData.owner.toString()).to.equal(provider.wallet.publicKey.toString());
  });

  it("Submit metrics", async () => {
    const tx = await program.methods
      .submitMetrics(99, new anchor.BN(10), new anchor.BN(1 * LAMPORTS_PER_SOL))
      .accounts({
        adminAccount: adminKeypair.publicKey,
        server: serverKeypair.publicKey,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const serverData = await program.account.server.fetch(serverKeypair.publicKey);
    expect(serverData.pendingRewards.toNumber()).to.equal(1 * LAMPORTS_PER_SOL);
  });

  it("Deposit rewards", async () => {
    const initialBalance = await provider.connection.getBalance(adminKeypair.publicKey);
    const depositAmount = new anchor.BN(2 * LAMPORTS_PER_SOL);

    const tx = await program.methods
      .depositRewards(depositAmount)
      .accounts({
        adminAccount: adminKeypair.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const adminData = await program.account.adminAccount.fetch(adminKeypair.publicKey);
    const finalBalance = await provider.connection.getBalance(adminKeypair.publicKey);

    expect(adminData.rewardPool.toNumber()).to.equal(2 * LAMPORTS_PER_SOL);
    expect(finalBalance - initialBalance).to.equal(2 * LAMPORTS_PER_SOL);
  });

  it("Claim rewards should fail due to cooldown", async () => {
    const initialBalance = await provider.connection.getBalance(provider.wallet.publicKey);

    try {
      await program.methods
        .claimRewards()
        .accounts({
          adminAccount: adminKeypair.publicKey,
          server: serverKeypair.publicKey,
          owner: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      expect(e.toString()).to.include("ClaimCooldownActive");
    }

    const finalBalance = await provider.connection.getBalance(provider.wallet.publicKey);
    expect(finalBalance).to.equal(initialBalance);
  });

  it("Deactivate server", async () => {
    const tx = await program.methods
      .deactivateServer()
      .accounts({
        adminAccount: adminKeypair.publicKey,
        server: serverKeypair.publicKey,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const serverData = await program.account.server.fetch(serverKeypair.publicKey);
    expect(serverData.active).to.equal(false);
  });
});

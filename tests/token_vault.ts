import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenVault } from "../target/types/token_vault";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { expect } from "chai";
import { PublicKey, SystemProgram, Transaction, Keypair } from "@solana/web3.js";

// --- Configuration ---
// Set your amount precision (e.g., 9 for standard tokens like USDC/SOL)
const DECIMAL_PLACES = 9; 
const LAMPORTS_PER_TOKEN = 10 ** DECIMAL_PLACES;

// Utility function to pause execution
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

describe("token_vault (Local Validator)", () => {
  // Configure the client to use the local cluster (Anchor.toml default)
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  
  // Load the program from the workspace
  const program = anchor.workspace.TokenVault as Program<TokenVault>;
  const payer = provider.wallet.publicKey;
  
  // State variables for testing
  let mint: PublicKey;
  let userTokenAccount: PublicKey;
  let vault: PublicKey;
  let vaultAuthority: PublicKey;
  let vaultTokenAccount: PublicKey;
  let vaultBump: number;
  let authorityBump: number;

  const depositAmount = 500 * LAMPORTS_PER_TOKEN;
  const withdrawAmount = 100 * LAMPORTS_PER_TOKEN;

  before("Setup: Creating Mint, User Account, and finding PDAs", async () => {
    console.log("--- 🔑 Running Setup on Local Validator ---");
    
    // 1. Create a NEW MINT for testing (requires payer to have SOL for rent)
    // We use provider.wallet.payer to sign creation, but the payer Public Key is payer
    mint = await createMint(
      provider.connection,
      provider.wallet.payer as Keypair, 
      payer, // Mint Authority
      payer, // Freeze Authority
      DECIMAL_PLACES 
    );
    console.log(`✅ Test Mint Created: ${mint.toBase58().slice(0, 10)}...`);

    // Ensure payer has SOL for transaction fees
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(payer, 100000000000), 
        "confirmed"
    );

    // 2. Create the user's Token Account (ATA)
    userTokenAccount = await getAssociatedTokenAddress(mint, payer, false, TOKEN_PROGRAM_ID);

    // 3. Mint tokens to the user's account
    await mintTo(
      provider.connection,
      provider.wallet.payer as Keypair,
      mint,
      userTokenAccount,
      payer, // Mint Authority
      1000 * LAMPORTS_PER_TOKEN // 1000 tokens
    );
    console.log(`✅ 1000 tokens minted to user ATA.`);

    // 4. Find the Program Derived Addresses (PDAs)
    [vault, vaultBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), payer.toBuffer()],
      program.programId
    );
    [vaultAuthority, authorityBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("authority"), vault.toBuffer()],
      program.programId
    );
    console.log(`✅ PDAs derived. Vault: ${vault.toBase58().slice(0, 10)}... Authority: ${vaultAuthority.toBase58().slice(0, 10)}...`);
  });
  
  // --- Test Suite ---

  it("1. Initialize vault and token account", async () => {
    await program.methods
      .initializeVault(vaultBump, authorityBump)
      .accounts({
        vault,
        vaultAuthority,
        tokenAccount: PublicKey.default, // Placeholder, actual account is initialized on-chain
        mint,
        payer,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    vaultTokenAccount = vaultAccount.tokenAccount;
    
    expect(vaultAccount.authority.toString()).to.equal(payer.toString());
    expect(vaultAccount.isLocked).to.be.false;
    console.log(`\t✅ Vault initialized successfully! Vault Token Account: ${vaultTokenAccount.toBase58().slice(0, 10)}...`);
  });

  it("2. Deposit tokens into the vault", async () => {
    // Check initial user balance
    const userPreBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accounts({
        vault,
        userTokenAccount,
        vaultTokenAccount,
        authority: payer,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBalance = await getAccount(provider.connection, vaultTokenAccount);
    const userPostBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    expect(vaultBalance.amount.toString()).to.equal(depositAmount.toString());
    expect(userPostBalance.toString()).to.equal(userPreBalance.sub(new anchor.BN(depositAmount)).toString());
    console.log(`\t✅ ${depositAmount / LAMPORTS_PER_TOKEN} tokens deposited.`);
  });

  it("3. Withdraw tokens when unlocked", async () => {
    // Check initial user balance (after deposit)
    const userPreBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    await program.methods
      .withdraw(new anchor.BN(withdrawAmount))
      .accounts({
        vault,
        vaultAuthority,
        userTokenAccount,
        vaultTokenAccount,
        authority: payer,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBalance = await getAccount(provider.connection, vaultTokenAccount);
    const userPostBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    const expectedVaultRemaining = depositAmount - withdrawAmount;
    expect(vaultBalance.amount.toString()).to.equal(expectedVaultRemaining.toString());
    expect(userPostBalance.toString()).to.equal(userPreBalance.add(new anchor.BN(withdrawAmount)).toString());
    console.log(`\t✅ ${withdrawAmount / LAMPORTS_PER_TOKEN} tokens withdrawn.`);
  });
  
  it("4. Lock vault with a short time-based condition", async () => {
    // Set unlock time to 5 seconds from now
    const currentUnixTime = Math.floor(Date.now() / 1000);
    const unlockTime = currentUnixTime + 5; 
    
    await program.methods
      .lockVault(new anchor.BN(unlockTime))
      .accounts({
        vault,
        authority: payer,
      })
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    expect(vaultAccount.isLocked).to.be.true;
    expect(vaultAccount.unlockTimestamp.toString()).to.equal(unlockTime.toString());
    console.log(`\t✅ Vault locked. Unlock time set for ${new Date(unlockTime * 1000).toLocaleTimeString()}.`);
  });

  it("5. FAIL to withdraw when locked (as expected)", async () => {
    try {
      await program.methods
        .withdraw(new anchor.BN(10 * LAMPORTS_PER_TOKEN))
        .accounts({
          vault,
          vaultAuthority,
          userTokenAccount,
          vaultTokenAccount,
          authority: payer,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      // If the transaction succeeds, force a failure
      expect.fail("Withdrawal should have failed because the vault is locked.");
    } catch (error) {
      // Check for the custom error message 'Vault is still locked'
      expect(error.error.errorMessage).to.include("Vault is still locked");
      console.log("\t✅ Withdrawal correctly blocked due to time lock.");
    }
  });

  it("6. Unlock vault after time passes", async () => {
    console.log("\t...Pausing for 6 seconds to wait for time lock to expire...");
    // Wait for the lock time to pass (5 seconds lock + 1 second buffer)
    await delay(6000); 

    await program.methods
      .unlockVault()
      .accounts({
        vault,
        authority: payer,
      })
      .rpc();

    const vaultAccount = await program.account.vault.fetch(vault);
    expect(vaultAccount.isLocked).to.be.false;
    console.log("\t✅ Vault unlocked successfully!");
  });
  
  it("7. Withdraw after unlock", async () => {
    // Should succeed now that the vault is unlocked
    const finalWithdrawAmount = 50 * LAMPORTS_PER_TOKEN;
    const userPreBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    await program.methods
      .withdraw(new anchor.BN(finalWithdrawAmount))
      .accounts({
        vault,
        vaultAuthority,
        userTokenAccount,
        vaultTokenAccount,
        authority: payer,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultBalance = await getAccount(provider.connection, vaultTokenAccount);
    const userPostBalance = (await getAccount(provider.connection, userTokenAccount)).amount;

    // Expected vault balance: 500 (deposit) - 100 (1st withdraw) - 50 (final withdraw) = 350
    const expectedFinalBalance = depositAmount - withdrawAmount - finalWithdrawAmount;
    
    expect(vaultBalance.amount.toString()).to.equal(expectedFinalBalance.toString());
    expect(userPostBalance.toString()).to.equal(userPreBalance.add(new anchor.BN(finalWithdrawAmount)).toString());
    console.log(`\t✅ Final withdrawal successful. Vault balance: ${expectedFinalBalance / LAMPORTS_PER_TOKEN}`);
  });

});

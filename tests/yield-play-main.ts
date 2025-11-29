import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { YieldPlayMain } from "../target/types/yield_play_main";
import { PublicKey, SystemProgram, TransactionInstruction, LAMPORTS_PER_SOL, Keypair, PACKET_DATA_SIZE, Transaction } from "@solana/web3.js";
import { program } from "@coral-xyz/anchor/dist/cjs/native/system";
import {
    Orao,
    networkStateAccountAddress,
    randomnessAccountAddress,
    FulfillBuilder,
    InitBuilder,
    NetworkState
} from "@orao-network/solana-vrf";

import {
  getDepositIx, getWithdrawIx, // get instructions
  getDepositContext, getWithdrawContext, // get context accounts for CPI
} from "@jup-ag/lend/earn";
import {assert} from "chai";
import {
    getOrCreateAssociatedTokenAccount,
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountInstruction,
    createMint,
    mintTo,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    getAccount,
} from "@solana/spl-token";
import { token } from "@coral-xyz/anchor/dist/cjs/utils/index.js";

describe("yield-play-main", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.yieldPlayMain as Program<YieldPlayMain>;

  const admin = provider.wallet;
  let adminPaymentAta: PublicKey;

  let fTokenMint: PublicKey;
  let collateralTokenAccount: PublicKey;
  let fakeLendingAccount: PublicKey;
  let fakeSupplyLiquidityAccount: PublicKey;
  let fakeSupplyPositionAccount: PublicKey;
  let fakeVaultAccount: PublicKey;
  let fakeLiquidityAccount: PublicKey;
  let claimAccount: PublicKey;
  let dummyLendingSetupReady = false;
  
  const vrf = new Orao(provider as any);

  const LOTTERY_STATE_SEED = Buffer.from("LOTTERY_STATE_SEED_3");
  const ROUND_STATE_SEED = Buffer.from("ROUND_STATE_SEED_2");
  const ROUND_VAULT_SIGNER_SEED = Buffer.from("ROUND_VAULT_SIGNER_SEED_2");
  const USER_STATE_SEED = Buffer.from("USER_STATE_SEED_2");
  const RANDOMNESS_ACCOUNT_SEED = Buffer.from("orao-vrf-randomness-request");
  const CONFIG_ACCOUNT_SEED = Buffer.from("orao-vrf-network-configuration");
  
  // USDC devnet mint and Jupiter Lend program
  const USDC_DEVNET_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const JUPITER_LEND_PROGRAM = new PublicKey("JUPBNB1C2iuWGvov2PF5qhEyqMt3mW4kqvX1dvvYvTy");

  let lotteryStatePDA: PublicKey;
  let roundVaultAta: PublicKey;
  let paymentMint: PublicKey;
  let vaultRoundSignerPDA: PublicKey;
  let destinationAta: PublicKey;
  let firstRoundPDA: PublicKey;
  let users : anchor.web3.Keypair[] = [];
  let usersState: PublicKey[] = [];


  async function waitForFulfillment(pda: PublicKey, randomSeed: Uint8Array) {
    const maxAttempts = 60; // 3 minutes max (60 * 3 seconds)
    let attempts = 0;
    
    while (attempts < maxAttempts) {
      try {
        // Get randomness data using the seed
        const randomnessData = await vrf.getRandomness(Buffer.from(randomSeed));
        if (randomnessData && randomnessData.getFulfilledRandomness()) {
          console.log("VRF fulfilled!");
          return;
        }
      } catch (error) {
        // Not yet fulfilled or error fetching
      }

      attempts++;
      console.log(`Waiting for VRF fulfill... (attempt ${attempts}/${maxAttempts})`);
      await new Promise((resolve) => setTimeout(resolve, 3000));
    }
    
    throw new Error("VRF fulfillment timeout - randomness was not fulfilled within 3 minutes");
  }


  async function transferLamports(
    provider: anchor.AnchorProvider,
    user: anchor.web3.PublicKey,
    amount: number
  ) {
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: admin.publicKey,
        toPubkey: user,
        lamports: amount,
      }),
    );
    await provider.sendAndConfirm(tx,)
  }
  before(async () => {
    // Add your setup here.
    lotteryStatePDA =  PublicKey.findProgramAddressSync(
      [LOTTERY_STATE_SEED],
      program.programId
    )[0];
    console.log("Lottery State PDA: ", lotteryStatePDA.toBase58());

    // Use USDC devnet mint for testing with Jupiter Lend
    // paymentMint = USDC_DEVNET_MINT;
    // console.log("Payment Mint (USDC Devnet): ", paymentMint.toBase58());

    // paymentMint = await createMint(provider.connection, provider.wallet.payer, provider.wallet.publicKey, null, 6);
    // console.log("Payment Mint: ", paymentMint.toBase58());

    paymentMint = new PublicKey("3fqZiPHypmkdaWARbB6LmLzMYChzoUkvmYqvSDPS8y5w");

    // const adminAta = await getOrCreateAssociatedTokenAccount(
    //   provider.connection,
    //   admin.payer,
    //   paymentMint,
    //   admin.publicKey,
    //   false,
    //   undefined,
    //   TOKEN_PROGRAM_ID,
    //   ASSOCIATED_TOKEN_PROGRAM_ID,
    // );
    // adminPaymentAta = adminAta.address;


  });

  it("Is initialized!", async () => {
    let lotteryState;
    try {
      lotteryState = await program.account.lotteryState.fetch(lotteryStatePDA);
      
    } catch (error) {
       const tx = await program.methods
      .initialize()
      .accounts({
        authority: admin.publicKey,
        lotteryState: lotteryStatePDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([])
      .rpc();
      console.log("Your transaction signature", tx);
      lotteryState = await program.account.lotteryState.fetch(lotteryStatePDA);
    }
    console.log("Lottery State: ");
    console.log("    admin: ", lotteryState.admin.toBase58());
    console.log("    global_round_counter: ", lotteryState.globalRoundCounter.toNumber());
    console.log("    is_pause: ", lotteryState.isPause);
    
   
    // assert.equal(lotteryState.admin.toBase58(), user.publicKey.toBase58());
    // assert.equal(lotteryState.ticketBasePrice.toNumber(), args.ticketBasePrice);
    // assert.equal(lotteryState.ticketPriceJump.toNumber(), args.ticketPriceJump);
    // assert.equal(lotteryState.ticketTimeJump.toNumber(), args.ticketTimeJump);
    // assert.equal(lotteryState.globalRoundCounter.toNumber(), 0);
    // assert.equal(lotteryState.isPause, false);

  });

  it ("Create Round!", async () => {
    let lotteryState = await program.account.lotteryState.fetch(lotteryStatePDA);
    firstRoundPDA = PublicKey.findProgramAddressSync(
      [ROUND_STATE_SEED, new BN(lotteryState.globalRoundCounter).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
    console.log("First Round PDA: ", firstRoundPDA.toBase58());


    vaultRoundSignerPDA = PublicKey.findProgramAddressSync(
      [ROUND_VAULT_SIGNER_SEED, lotteryState.globalRoundCounter.toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
    roundVaultAta = getAssociatedTokenAddressSync(paymentMint, vaultRoundSignerPDA, true);
    const createIx = createAssociatedTokenAccountInstruction(
      provider.wallet.publicKey, // payer
      roundVaultAta, // ata
      vaultRoundSignerPDA, // owner
      paymentMint, // mint
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    await provider.sendAndConfirm(new anchor.web3.Transaction().add(createIx));
    console.log("Round Vault ATA: ", roundVaultAta.toBase58());

    let firstRoundState;
    try {
      firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    } catch (error) {
      let arg = {
        roundId: new BN(1),
        startTs: new BN(Math.floor(Date.now() / 1000) + 1),
        endTs: new BN(Math.floor(Date.now()/1000) + 30),
        gapTime: new BN(1),
        ticketBasePrice: new BN(1_000_000), // 1 token
        ticketPriceJump: new BN(1_000_000), // 1 token
      };
      const tx = await program.methods
        .createRound(arg)
        .accounts({
          authority: admin.publicKey,
          lotteryState: lotteryStatePDA,
          roundState: firstRoundPDA,
          vaultRoundSigner: vaultRoundSignerPDA,
          paymentMint: paymentMint,
          roundVaultAta: roundVaultAta,
          destinationMint: paymentMint,
          destinationAta: roundVaultAta,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([])
        .rpc();
      console.log("Your transaction signature", tx);
      firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    }
    
    console.log("First Round State: ");
    console.log("    admin: ", firstRoundState.admin.toBase58());
    console.log("    round_id: ", firstRoundState.roundId.toNumber());
    console.log("    total_deposit: ", firstRoundState.totalDeposit.toNumber());
    console.log("    total_refunded: ", firstRoundState.totalRefunded.toNumber());
    console.log("    total_tickets: ", firstRoundState.totalTickets.toNumber());
    console.log("    start_ts: ", firstRoundState.startTs.toNumber());
    console.log("    end_ts: ", firstRoundState.endTs.toNumber());
    console.log("    gap_time: ", firstRoundState.gapTime.toNumber());
    console.log("    status: ", firstRoundState.status);
    console.log("    ticket_base_price: ", firstRoundState.ticketBasePrice.toNumber());
    console.log("    ticket_price_jump: ", firstRoundState.ticketPriceJump.toNumber());
    console.log("    price_per_ticket: ", firstRoundState.pricePerTicket.toNumber());
    console.log("    round_seed: ", firstRoundState.roundSeed);
    console.log("    vrf_seed:", firstRoundState.vrfSeed);

    //create ATA for round vault
    
  });

  it.skip("Get the seed from VRF!", async () => {
    let lotteryState = await program.account.lotteryState.fetch(lotteryStatePDA);
    let count = lotteryState.globalRoundCounter.toNumber()-1;
    let firstRoundPDA = PublicKey.findProgramAddressSync(
      [ROUND_STATE_SEED, new BN(count).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
    let firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    
    let randomNumberPda = PublicKey.findProgramAddressSync(
        [RANDOMNESS_ACCOUNT_SEED, Buffer.from(firstRoundState.roundSeed)],
        vrf.programId // ORAO program ID on devnet
      )[0];

    const networkState = await vrf.getNetworkState();
    let treasury = networkState.config.treasury;
    

    // Request randomness
    await program.methods.requestResult()
    .accounts({
      authority: admin.publicKey,
      roundState: firstRoundPDA,
      randomNumberAcct: randomNumberPda,
      config: networkStateAccountAddress(),
      treasury: treasury,
      vrf: vrf.programId,
      systemProgram: SystemProgram.programId,
    })
    .signers([])
    .rpc();
    await waitForFulfillment(randomNumberPda, new Uint8Array(firstRoundState.roundSeed));
    const randomnessData = await vrf.getRandomness(Buffer.from(firstRoundState.roundSeed));
    if (randomnessData) {
      const fulfilledRandomness = randomnessData.getFulfilledRandomness();
      if (fulfilledRandomness) {
        console.log("Fulfilled Randomness Seed:", Array.from(fulfilledRandomness));
      }
    }

    // Get vrf_seed
    await program.methods.fulfillResult()
    .accounts({
      roundState: firstRoundPDA,
      randomNumberAcct: randomNumberPda,
    })
    .rpc();
    firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    console.log("VRF Seed in Round State: ", firstRoundState.vrfSeed);
  });
  it.skip("Update Price!", async () => {
    await new Promise(resolve => setTimeout(resolve, 5000)); //wait 5 seconds
    const ix = await program.methods.updatePrice()
    .accounts({
      authority: admin.publicKey,
      roundState: firstRoundPDA,
      lotteryState: lotteryStatePDA,
    })
    .rpc();
    console.log("Update Price IX: ", ix);
    let roundState = await program.account.roundState.fetch(firstRoundPDA);
    console.log("New price per ticket: ", roundState.pricePerTicket.toNumber());
  });
  it("Enter Round!", async () => {
    
    let firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
      console.log("Round State before user enter: ");

      console.log("    total_deposit: ", firstRoundState.totalDeposit.toNumber());
      console.log("    total_tickets: ", firstRoundState.totalTickets.toNumber());
      console.log("    price_per_ticket: ", firstRoundState.pricePerTicket.toNumber());
    for (let i = 0; i < 5; i++) {
      
      const user = anchor.web3.Keypair.generate();
      await transferLamports(provider, user.publicKey, 0.01 * LAMPORTS_PER_SOL);

      console.log("User ", i, user.publicKey.toBase58(), "Balance: ", await provider.connection.getBalance(user.publicKey));
      const userAta = getAssociatedTokenAddressSync(paymentMint, user.publicKey);
      const createIx = createAssociatedTokenAccountInstruction(
        provider.wallet.publicKey, // payer
        userAta, // ata
        user.publicKey, // owner
        paymentMint, // mint
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );
      await provider.sendAndConfirm(new anchor.web3.Transaction().add(createIx));
      await mintTo(
        provider.connection,
        admin.payer,
        paymentMint,
        userAta,
        admin.publicKey,
        10_000_000, // 10 tokens
        [],
        undefined,
        TOKEN_PROGRAM_ID
      )
      let userAtaAccount = await getAccount(
        provider.connection,
        userAta,
        undefined, 
        TOKEN_PROGRAM_ID
      );
      const userRoundStatePDA = PublicKey.findProgramAddressSync(
        [USER_STATE_SEED, firstRoundState.roundId.toArrayLike(Buffer, "le", 8), user.publicKey.toBuffer()],
        program.programId
      )[0];
      console.log(`User ${i} ATA: `, userAta.toBase58(), " Balance: ", Number(userAtaAccount.amount));

      // Enter Round
      users.push(user);
      await program.methods.enterRound(new BN(2)).accounts({
        user: user.publicKey,
        lotteryState: lotteryStatePDA,
        roundState: firstRoundPDA,
        userRoundState: userRoundStatePDA,
        paymentMint: paymentMint,
        roundVaultAta: roundVaultAta,
        userAta: userAta,
        vaultRoundSigner: vaultRoundSignerPDA,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();
      

      userAtaAccount = await getAccount(
        provider.connection,
        userAta,
        undefined, 
        TOKEN_PROGRAM_ID
      );
      console.log(`User ${i} ATA Balance after enter: `, Number(userAtaAccount.amount));

      let userRoundState = await program.account.userRoundState.fetch(userRoundStatePDA);
      console.log("User Round State: ");
      console.log("    user: ", userRoundState.user.toBase58());
      console.log("    round_id: ", userRoundState.roundId.toNumber());
      console.log("    deposit_amount: ", userRoundState.depositAmount.toNumber());
      console.log("    ticket_count: ", userRoundState.ticketCount.toNumber());
      console.log("    is_claimed: ", userRoundState.isClaimed);
      console.log("---------------------------------------------------");
      usersState.push(userRoundStatePDA);
    }
    const roundStateAfter = await program.account.roundState.fetch(firstRoundPDA);
      console.log("Round State after: ");
      console.log("    total_deposit: ", roundStateAfter.totalDeposit.toNumber());
      console.log("    total_tickets: ", roundStateAfter.totalTickets.toNumber());
    let vaultAtaAccount = await getAccount(
      provider.connection,
      roundVaultAta,
      undefined, 
      TOKEN_PROGRAM_ID
    );
    console.log("Round Vault ATA Balance: ", Number(vaultAtaAccount.amount));
  });

  // it.skip("Deposit to Jupiter lending on devnet with USDC", async () => {
  //   // Ensure vault has some USDC balance
  //   let vaultBalance = await getAccount(
  //     provider.connection,
  //     roundVaultAta,
  //     undefined,
  //     TOKEN_PROGRAM_ID,
  //   );

  //   console.log("Vault balance before deposit:", vaultBalance.amount.toString());

  //   if (vaultBalance.amount === BigInt(0)) {
  //     console.log("Warning: Vault has zero balance. You need to fund the vault with USDC devnet tokens first.");
  //     console.log("Vault ATA:", roundVaultAta.toBase58());
  //     return;
  //   }

  //   // Get Jupiter Lend deposit context
  //   const depositAmount = BigInt(1_000_000); // 1 USDC (6 decimals)
  //   const { accounts: depositAccounts } = await getDepositContext({
  //     wallet: vaultRoundSignerPDA,
  //     tokenMint: paymentMint,
  //     amount: depositAmount,
  //   });

  //   console.log("Jupiter Lend Deposit Accounts:");
  //   console.log("  lending:", depositAccounts.lending.toBase58());
  //   console.log("  fTokenMint:", depositAccounts.fTokenMint.toBase58());
  //   console.log("  lendingProgram:", JUPITER_LEND_PROGRAM.toBase58());

    // try {
    //   const tx = await program.methods
    //     .depositToLending()
    //     .accounts({
    //       authority: admin.publicKey,
    //       roundState: firstRoundPDA,
    //       vaultRoundSigner: vaultRoundSignerPDA,
    //       paymentMint: paymentMint,
    //       roundVaultAta: roundVaultAta,
    //       recipientTokenAccount: depositAccounts.recipientTokenAccount,
    //       lendingAdmin: depositAccounts.lendingAdmin,
    //       lending: depositAccounts.lending,
    //       fTokenMint: depositAccounts.fTokenMint,
    //       supplyTokenReservesLiquidity: depositAccounts.supplyTokenReservesLiquidity,
    //       lendingSupplyPositionOnLiquidity: depositAccounts.lendingSupplyPositionOnLiquidity,
    //       rateModel: depositAccounts.rateModel,
    //       vault: depositAccounts.vault,
    //       liquidity: depositAccounts.liquidity,
    //       liquidityProgram: depositAccounts.liquidityProgram,
    //       rewardsRateModel: depositAccounts.rewardsRateModel,
    //       tokenProgram: TOKEN_PROGRAM_ID,
    //       associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    //       systemProgram: SystemProgram.programId,
    //       lendingProgram: JUPITER_LEND_PROGRAM,
    //     })
    //     .rpc();
      
    //   console.log("Deposit transaction:", tx);
      
    //   // Check round state was updated
    //   const roundStateAfter = await program.account.roundState.fetch(firstRoundPDA);
    //   console.log("Total farmed amount after deposit:", roundStateAfter.totalFarmedAmount.toString());
    // } catch (err: any) {
    //   console.error("Deposit failed:", err);
    //   throw err;
    // }
  // });

  // it.skip("Withdraw from Jupiter lending on devnet with USDC", async () => {
  //   // Get Jupiter Lend withdraw context
  //   const { accounts: withdrawAccounts } = await getWithdrawContext({
  //     wallet: vaultRoundSignerPDA,
  //     tokenMint: paymentMint,
  //   });

  //   console.log("Jupiter Lend Withdraw Accounts:");
  //   console.log("  lending:", withdrawAccounts.lending.toBase58());
  //   console.log("  fTokenMint:", withdrawAccounts.fTokenMint.toBase58());
  //   console.log("  collateralTokenAccount:", withdrawAccounts.collateralTokenAccount.toBase58());
  //   console.log("  claimAccount:", withdrawAccounts.claimAccount?.toBase58() || "none");

  //   // Check if vault has collateral tokens to withdraw
  //   try {
  //     const collateralBalance = await getAccount(
  //       provider.connection,
  //       withdrawAccounts.collateralTokenAccount,
  //       undefined,
  //       TOKEN_PROGRAM_ID,
  //     );
  //     console.log("Collateral token balance:", collateralBalance.amount.toString());

  //     if (collateralBalance.amount === BigInt(0)) {
  //       console.log("No collateral tokens to withdraw. Run deposit test first.");
  //       return;
  //     }
  //   } catch (err) {
  //     console.log("Collateral token account does not exist yet. Run deposit test first.");
  //     return;
  //   }

  //   try {
  //     const tx = await program.methods
  //       .withdrawFromLending()
  //       .accounts({
  //         authority: admin.publicKey,
  //         roundState: firstRoundPDA,
  //         vaultRoundSigner: vaultRoundSignerPDA,
  //         paymentMint: paymentMint,
  //         roundVaultAta: roundVaultAta,
  //         collateralTokenAccount: withdrawAccounts.collateralTokenAccount,
  //         lendingAdmin: withdrawAccounts.lendingAdmin,
  //         lending: withdrawAccounts.lending,
  //         fTokenMint: withdrawAccounts.fTokenMint,
  //         supplyTokenReservesLiquidity: withdrawAccounts.supplyTokenReservesLiquidity,
  //         lendingSupplyPositionOnLiquidity: withdrawAccounts.lendingSupplyPositionOnLiquidity,
  //         rateModel: withdrawAccounts.rateModel,
  //         vault: withdrawAccounts.vault,
  //         claimAccount: withdrawAccounts.claimAccount || SystemProgram.programId,
  //         liquidity: withdrawAccounts.liquidity,
  //         liquidityProgram: withdrawAccounts.liquidityProgram,
  //         rewardsRateModel: withdrawAccounts.rewardsRateModel,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //         systemProgram: SystemProgram.programId,
  //         lendingProgram: JUPITER_LEND_PROGRAM,
  //       })
  //       .rpc();
      
  //     console.log("Withdraw transaction:", tx);
      
  //     // Check vault balance increased
  //     const vaultBalanceAfter = await getAccount(
  //       provider.connection,
  //       roundVaultAta,
  //       undefined,
  //       TOKEN_PROGRAM_ID,
  //     );
  //     console.log("Vault balance after withdraw:", vaultBalanceAfter.amount.toString());
      
  //     // Check round state was updated
  //     const roundStateAfter = await program.account.roundState.fetch(firstRoundPDA);
  //     console.log("Total farmed amount after withdraw:", roundStateAfter.totalFarmedAmount.toString());
  //   } catch (err: any) {
  //     console.error("Withdraw failed:", err);
  //     throw err;
  //   }
  // });

  it("Update Balance!", async () => {
    await mintTo(
        provider.connection,
        admin.payer,
        paymentMint,
        roundVaultAta,
        admin.publicKey,
        1_000_000, // 1 token
        [],
        undefined,
        TOKEN_PROGRAM_ID
      )
    let vaultAtaAccount = await getAccount(
      provider.connection,
      roundVaultAta,
      undefined, 
      TOKEN_PROGRAM_ID
    );
    console.log("Round Vault ATA Balance: ", Number(vaultAtaAccount.amount));
 
    const ix = await program.methods.updateBalance()
    .accounts({
      authority: admin.publicKey,
      roundState: firstRoundPDA,
      roundVaultAta: roundVaultAta,
      vaultRoundSigner: vaultRoundSignerPDA,
      paymentMint: paymentMint,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
    console.log("Update Balance IX: ", ix);
    let roundState = await program.account.roundState.fetch(firstRoundPDA);
    console.log("New total farmed amount: ", roundState.totalFarmedAmount.toNumber());
  });
  
  it("Choose winner!", async () => {

    let now = Math.floor(Date.now() / 1000);
    let firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    let waitingTime = firstRoundState.endTs.toNumber() + firstRoundState.gapTime.toNumber() - now + 2;
    console.log("Current ts: ", now);
    console.log("Round end ts: ", firstRoundState.endTs.toNumber());
    console.log("Waiting for round to end: ", waitingTime);
    await new Promise(resolve => setTimeout(resolve, waitingTime * 1000)); //wait to let round end

    const ix = await program.methods.chooseWinner()
    .accounts({
      authority: admin.publicKey,
      roundState: firstRoundPDA,
      firstPrize: users[0].publicKey,
      secondPrize: users[1].publicKey,
      thirdPrize: users[0].publicKey,
      firstPrizeRoundState: usersState[0],
      secondPrizeRoundState: usersState[1],
      thirdPrizeRoundState: usersState[0],
    })
    .rpc();
    firstRoundState = await program.account.roundState.fetch(firstRoundPDA);
    console.log("Choose Winner IX: ", ix);
    console.log("Winners chosen.");
    console.log("first prize: ", firstRoundState.firstPrize.toBase58());
    console.log("second prize: ", firstRoundState.secondPrize.toBase58());
    console.log("third prize: ", firstRoundState.thirdPrize.toBase58());
    console.log("Round status: ", firstRoundState.status);
    
  });
  it.skip("Deposit prizes to vault!", async () => {
    
    
  });

  it("Claim prizes!", async () => {
    const userAta = getAssociatedTokenAddressSync(paymentMint, users[0].publicKey);
    let userAtaAccount = await getAccount(
      provider.connection,
      userAta,
      undefined, 
      TOKEN_PROGRAM_ID
    );
    console.log("User 0 ATA Balance before claim: ", Number(userAtaAccount.amount));

    const firstRoundStateBefore = await program.account.roundState.fetch(firstRoundPDA);
    console.log("First Round State before claim: ");
    console.log("     total_refunded: ", firstRoundStateBefore.totalRefunded.toNumber());
    console.log("     state: ", firstRoundStateBefore.status);
    const ix = await program.methods.claim()
    .accounts({
      user: users[0].publicKey,
      roundState: firstRoundPDA,
      userRoundState: usersState[0],
      vaultRoundSigner: vaultRoundSignerPDA,
      roundVaultAta: roundVaultAta,
      paymentMint: paymentMint,
      userAta: userAta,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([users[0]])
    .rpc();
    console.log("Claim IX: ", ix);
    userAtaAccount = await getAccount(
      provider.connection,
      userAta,
      undefined, 
      TOKEN_PROGRAM_ID
    );
    console.log("User 0 ATA Balance after claim: ", Number(userAtaAccount.amount));
  });
});

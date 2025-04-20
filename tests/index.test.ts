import * as web3 from "@solana/web3.js";
import { initializeSolSignerKeypair} from "./initializeKeypair";  
import * as borsh from "borsh";
import * as fs from "fs";
import { Suite } from 'mocha';
import { CounterInstruction,  IncreaseCounterPayload } from "./schema";
import { DELEGATION_PROGRAM_ID, delegationRecordPdaFromDelegatedAccount, delegationMetadataPdaFromDelegatedAccount, delegateBufferPdaFromDelegatedAccountAndOwnerProgram, MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID } from "@magicblock-labs/ephemeral-rollups-sdk";

import dotenv from 'dotenv'
dotenv.config()
describe("Running tests:", async function(this: Suite) {
  const keypairPath = "target/deploy/ephemeral_rollup_counter_program-keypair.json";
  const secretKey = Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, "utf8")));
  const keypair = web3.Keypair.fromSecretKey(secretKey);
  const PROGRAM_ID = keypair.publicKey; 

  const connectionBaseLayer = new web3.Connection("https://api.devnet.solana.com", { wsEndpoint: "wss://api.devnet.solana.com" });
  const connectionEphemeralRollup = new web3.Connection(process.env.PROVIDER_ENDPOINT || "https://devnet.magicblock.app/", { wsEndpoint: process.env.WS_ENDPOINT || "wss://devnet.magicblock.app/" });
  const userKeypair = initializeSolSignerKeypair();  

  let [counterPda, _bump] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("counter_acc"), userKeypair.publicKey.toBuffer()],
    PROGRAM_ID
  );
  console.log("Program ID: ", PROGRAM_ID.toString())
  console.log("Counter PDA: ", counterPda.toString())

  it("Initialize counter on Solana", async function() {
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      }
    ]
    const serializedInstructionData = Buffer.concat([
      Buffer.from(CounterInstruction.InitializeCounter, 'hex'),
    ])
    const initializeIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(initializeIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionBaseLayer, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)

  });

  it("Increase counter on Solana", async function() {

    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      }
    ]
    const serializedInstructionData = Buffer.concat([
      Buffer.from(CounterInstruction.IncreaseCounter, 'hex'),
      borsh.serialize(IncreaseCounterPayload.schema, new IncreaseCounterPayload(1))
    ])
    const increaseCounterIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(increaseCounterIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionBaseLayer, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)
  });

  it("Delegate counter to ER", async function() {
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: delegateBufferPdaFromDelegatedAccountAndOwnerProgram(counterPda, PROGRAM_ID),
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: delegationRecordPdaFromDelegatedAccount(counterPda),
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: delegationMetadataPdaFromDelegatedAccount(counterPda),
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: DELEGATION_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
    ]
    const serializedInstructionData = Buffer.from(CounterInstruction.Delegate, 'hex')
    const delegateIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(delegateIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionBaseLayer, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)

  });

  it("Increase counter on ER (1)", async function() {
    const start = Date.now();
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      }
    ]
    const serializedInstructionData = Buffer.concat([
      Buffer.from(CounterInstruction.IncreaseCounter, 'hex'),
      borsh.serialize(IncreaseCounterPayload.schema, new IncreaseCounterPayload(1))
    ])
    const initializeIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(initializeIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionEphemeralRollup, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)

    const duration = Date.now() - start;
    console.log(`(${duration}ms)`);
  });

  it("Commit counter state on ER to Solana", async function() {
    const start = Date.now();
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: MAGIC_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: MAGIC_CONTEXT_ID,
        isSigner: false,
        isWritable: true,
      }
    ]
    const serializedInstructionData = Buffer.from(CounterInstruction.Commit, 'hex')
    const commitIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(commitIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionEphemeralRollup, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)
    const duration = Date.now() - start;
    console.log(`(${duration}ms)`);
  });

  it("Increase counter on ER (2)", async function() {
    const start = Date.now();
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      }
    ]
    const serializedInstructionData = Buffer.concat([
      Buffer.from(CounterInstruction.IncreaseCounter, 'hex'),
      borsh.serialize(IncreaseCounterPayload.schema, new IncreaseCounterPayload(1))
    ])
    const initializeIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(initializeIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionEphemeralRollup, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)

    const duration = Date.now() - start;
    console.log(`(${duration}ms)`);
  });

  it("Commit and undelegate counter on ER to Solana", async function() {
    const start = Date.now();
    const tx = new web3.Transaction();
    const keys = [
      {
        pubkey: userKeypair.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: counterPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: MAGIC_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: MAGIC_CONTEXT_ID,
        isSigner: false,
        isWritable: true,
      }
    ]
    const serializedInstructionData = Buffer.from(CounterInstruction.CommitAndUndelegate, 'hex')
    const undelegateIx = new web3.TransactionInstruction({
      keys: keys,
      programId: PROGRAM_ID,
      data: serializedInstructionData
    });
    tx.add(undelegateIx);
    const txHash = await web3.sendAndConfirmTransaction(connectionEphemeralRollup, tx, [userKeypair],
      {
        skipPreflight: true,
        commitment: "confirmed"
      }
    );
    console.log("txId:", txHash)

    const duration = Date.now() - start;
    console.log(`(${duration}ms)`);
  });
});

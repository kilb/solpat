// export ANCHOR_WALLET=/home/ke/.config/solana/id.json
// export ANCHOR_PROVIDER_URL="https://api.devnet.solana.com"
import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { Solpat } from '../../target/types/solpat';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
// import NodeWallet from '@project-serum/anchor/dist/cjs/provider';
import { PublicKey, Keypair, clusterApiUrl, SystemProgram, Transaction, Connection, Commitment } from '@solana/web3.js';
// import idl from './idl.json';
const idl = require('./idl.json');

const assert = require("assert");
const fs = require('fs');
const programID = new PublicKey(idl.metadata.address);
const options = anchor.Provider.defaultOptions();

function getKeypair() {
  let data = fs.readFileSync('/home/ke/.config/solana/id.json', 'utf8');
  let secretKey = Uint8Array.from(JSON.parse(data));
  return Keypair.fromSecretKey(secretKey);
}

let myKey = getKeypair();

const priceFeedAccount = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";
const AggregatorPublicKey = new PublicKey(priceFeedAccount);
const myMintAccount = "DCWj38SJkuZfy4UZDJkHsCEXZbJ3xBHQetw4oTX7z2uz";
const myMintPublickey = new PublicKey(myMintAccount);
const tokenUserAccount = "CMcmPxyd2m92f2GAUea1zTkparTZZQzkz8Fn2JFoAozB";
const token_user = new PublicKey(tokenUserAccount);
// Configure the client to use the local cluster.
// const connection = new Connection(clusterApiUrl("devnet"));
// const wallet = anchor.Provider.env();

// const provider = new anchor.Provider(connection, wallet, options);
// anchor.setProvider(provider);
// anchor.setProvider(anchor.Provider.env());
const provider = anchor.Provider.env();
anchor.setProvider(provider);
// anchor.setProvider(anchor.Provider.env());

const program = anchor.workspace.Solpat as Program<Solpat>;
const wallet = program.provider.wallet;

let pool_id = new anchor.BN(7);

async function createPool() {
  console.log("program id", program.programId.toBase58());
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  console.log("pool_account_pda", pool_account_pda.toBase58());
  const [token_vault_pda, _token_vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("token")), pool_account_pda.toBuffer()],
    program.programId
  );

  const tx = await program.rpc.createPool(
    pool_id,
    new anchor.BN(0), // duration: 0s 
    new anchor.BN(10), // fee_rate: 10/10000
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        token_vault: token_vault_pda,
        feedAccount: AggregatorPublicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMint: myMintPublickey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let poolAccount = await program.account.pool.fetch(pool_account_pda);
  assert.ok(
    poolAccount.authority.equals(wallet.publicKey)
  );
  assert.ok(
    poolAccount.nextRound.toNumber() == 2
  );
  return "OK";
}

async function startRound() {
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  console.log("pool_account_pda", pool_account_pda.toBase58());

  let poolAccount2 = await program.account.pool.fetch(pool_account_pda);
  assert.ok(
    poolAccount2.nextRound.toNumber() == 2
  );

  const [next_round_pda, _next_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.toBuffer("be", 8)],
    program.programId
  );

  const tx = await program.rpc.startRound(
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        nextRound: next_round_pda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMint: myMintPublickey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let roundAccount = await program.account.round.fetch(next_round_pda);
  assert.ok(
    roundAccount.bonus.toNumber() == 0
  );
  return "OK"
}

async function betRound() {
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  //可以将round id记录在后台中，减少链查询
  let poolAccount2 = await program.account.pool.fetch(pool_account_pda);

  const [cur_round_pda, _cur_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.subn(1).toBuffer("be", 8)],
    program.programId
  );

  const [token_vault_pda, _token_vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("token")), pool_account_pda.toBuffer()],
    program.programId
  );

  const [user_bet_pda, _user_bet_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("bet")), cur_round_pda.toBuffer(), wallet.publicKey.toBuffer()],
    program.programId
  );

  const tx = await program.rpc.bet(
    new anchor.BN(10000), // bet amount
    poolAccount2.nextRound.subn(1),
    0,
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        tokenVault: token_vault_pda,
        tokenUser: token_user,
        curRound: cur_round_pda,
        userBet: user_bet_pda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let userBet = await program.account.userBet.fetch(user_bet_pda);
  assert.ok(
    userBet.betDown.toNumber() == 10000
  );
  let roundAccount = await program.account.round.fetch(cur_round_pda);
  assert.ok(
    roundAccount.depositDown.toNumber() == 10000
  );
  return "OK"
}

async function lockRound() {
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  let poolAccount2 = await program.account.pool.fetch(pool_account_pda);
  assert.ok(
    poolAccount2.nextRound.toNumber() == 3
  );

  const [cur_round_pda, _cur_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.subn(1).toBuffer("be", 8)],
    program.programId
  );

  const [next_round_pda, _next_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.toBuffer("be", 8)],
    program.programId
  );

  const tx = await program.rpc.lockRound(
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        nextRound: next_round_pda,
        curRound: cur_round_pda,
        feedAccount: AggregatorPublicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMint: myMintPublickey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let roundAccount = await program.account.round.fetch(cur_round_pda);
  assert.ok(
    roundAccount.status == 1
  );
  return "OK"
}

async function processRound() {
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  let poolAccount2 = await program.account.pool.fetch(pool_account_pda);
  // assert.ok(
  //   poolAccount2.nextRound.toNumber() == 4
  // );

  const [pre_round_pda, _pre_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.subn(2).toBuffer("be", 8)],
    program.programId
  );

  const [cur_round_pda, _cur_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.subn(1).toBuffer("be", 8)],
    program.programId
  );

  const [next_round_pda, _next_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), poolAccount2.nextRound.toBuffer("be", 8)],
    program.programId
  );

  const tx = await program.rpc.processRound(
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        nextRound: next_round_pda,
        curRound: cur_round_pda,
        preRound: pre_round_pda,
        feedAccount: AggregatorPublicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMint: myMintPublickey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let roundAccount = await program.account.round.fetch(pre_round_pda);
  console.log("Price", roundAccount.lockPrice.toNumber(), roundAccount.closedPrice.toNumber());
  return "OK"
}

async function claimRound(round_id) {
  //可以将round id记录在后台中，减少链查询
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;

  const [claim_round_pda, _claim_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), new anchor.BN(round_id).toBuffer("be", 8)],
    program.programId
  );

  const [token_vault_pda, _token_vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("token")), pool_account_pda.toBuffer()],
    program.programId
  );

  const [user_bet_pda, _user_bet_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("bet")), claim_round_pda.toBuffer(), wallet.publicKey.toBuffer()],
    program.programId
  );

  const tx = await program.rpc.claim(
    new anchor.BN(round_id),
    {
      accounts: {
        authority: wallet.publicKey,
        pool: pool_account_pda,
        tokenVault: token_vault_pda,
        tokenUser: token_user,
        curRound: claim_round_pda,
        userBet: user_bet_pda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let roundAccount = await program.account.round.fetch(claim_round_pda);
  console.log("roundAccount.accountsAmount", roundAccount.accountsAmount.toNumber());
  return "OK";
}

async function takeFeeRound(round_id) {
  //可以将round id记录在后台中，减少链查询
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  const [claim_round_pda, _claim_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), new anchor.BN(round_id).toBuffer("be", 8)],
    program.programId
  );

  const [token_vault_pda, _token_vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("token")), pool_account_pda.toBuffer()],
    program.programId
  );

  const tx = await program.rpc.takeFee(
    new anchor.BN(round_id), // round id
    {
      accounts: {
        authority: wallet.publicKey,
        tokenVault: token_vault_pda,
        tokenUser: token_user,
        curRound: claim_round_pda,
        pool: pool_account_pda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  let roundAccount = await program.account.round.fetch(claim_round_pda);
  assert.ok(
    roundAccount.status == 3
  );

  return "OK";
}

async function freeRound(round_id) {
  //可以将round id记录在后台中，减少链查询
  const [_pool_account_pda, _pool_account_bump] = await PublicKey.findProgramAddress(
    [pool_id.toBuffer("be", 8)],
    program.programId
  );
  let pool_account_pda = _pool_account_pda;
  let poolAccount2 = await program.account.pool.fetch(pool_account_pda);
  const [claim_round_pda, _claim_round_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("round")), pool_account_pda.toBuffer(), new anchor.BN(round_id).toBuffer("be", 8)],
    program.programId
  );

  const [token_vault_pda, _token_vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from(anchor.utils.bytes.utf8.encode("token")), pool_account_pda.toBuffer()],
    program.programId
  );

  const tx = await program.rpc.freeRound(
    new anchor.BN(round_id), // round id
    {
      accounts: {
        authority: wallet.publicKey,
        tokenVault: token_vault_pda,
        tokenUser: token_user,
        curRound: claim_round_pda,
        pool: pool_account_pda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      }
    });
  console.log("Your transaction signature", tx);
  return "OK";
}

createPool().then(console.log);
startRound().then(console.log);
betRound().then(console.log);
lockRound().then(console.log);
processRound().then(console.log);
claimRound().then(console.log);
takeFeeRound().then(console.log);
processRound().then(console.log);
freeRound().then(console.log);
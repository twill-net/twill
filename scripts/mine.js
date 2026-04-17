#!/usr/bin/env node
/**
 * Twill Network — PoC+PoSe Miner
 *
 * Mines TWL by solving SHA256(nonce || settlement_root || parent_hash) < difficulty.
 * At genesis the settlement_root is 0x000...000 and difficulty is 0x00FFFFFF...
 * (~256 hashes per block on average). Difficulty auto-adjusts every 2016 blocks.
 *
 * Bootstrap mode (first 10M TWL): submits unsigned proofs — no fee required.
 * After bootstrap: submits signed proofs — requires small TWL balance for fees.
 *
 * Usage:
 *   MNEMONIC="your twelve words" node scripts/mine.js
 *   RPC=ws://1.2.3.4:9944 MNEMONIC="..." node scripts/mine.js
 *
 * Requirements:
 *   npm install @polkadot/api @polkadot/keyring @polkadot/util-crypto sha256
 *   (or: cd scripts && npm install)
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');
const { cryptoWaitReady } = require('@polkadot/util-crypto');
const crypto = require('crypto');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const RPC_URL   = process.env.RPC   || 'ws://127.0.0.1:9944';
const MNEMONIC  = process.env.MNEMONIC || '//Alice';
const LOG_EVERY = parseInt(process.env.LOG_EVERY || '10000', 10);

// GPU helper. Compile with `cargo build --release -p twill-miner`; the host
// binary lives at target/release/twill-miner. Set TWILL_MINER=cpu to force the
// JS CPU path (useful if you don't have a GPU or want a portable smoke test).
const FORCE_CPU = (process.env.TWILL_MINER || '').toLowerCase() === 'cpu';
const MINER_BIN = process.env.TWILL_MINER_BIN || (() => {
  // Walk a few likely locations relative to this script.
  const candidates = [
    path.resolve(__dirname, '..', 'target', 'release', 'twill-miner'),
    path.resolve(__dirname, '..', 'target', 'debug', 'twill-miner'),
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return null;
})();

// 10M TWL in planck (12 decimals)
const BOOTSTRAP_THRESHOLD = BigInt('10000000000000000000');

function sha256(data) {
  return crypto.createHash('sha256').update(data).digest();
}

function toH256(hexOrBytes) {
  if (typeof hexOrBytes === 'string') {
    return Buffer.from(hexOrBytes.replace('0x', ''), 'hex');
  }
  return Buffer.from(hexOrBytes);
}

/**
 * CPU fallback — find a nonce whose hash beats the difficulty target.
 * Used only when the GPU helper is not built. Slow.
 */
async function findNonceCpu(settlementRoot, parentHash, difficulty, signal) {
  const rootBytes    = toH256(settlementRoot);
  const parentBytes  = toH256(parentHash);
  const diffBytes    = toH256(difficulty);

  let attempts = 0;
  const start = Date.now();

  while (!signal.aborted) {
    const nonce = crypto.randomBytes(32);
    const input = Buffer.concat([nonce, rootBytes, parentBytes]);
    const hash  = sha256(input);

    if (hashLessThan(hash, diffBytes)) {
      const elapsed = (Date.now() - start) / 1000;
      const rate = (attempts / elapsed).toFixed(0);
      console.log(`  ✓ Found nonce after ${attempts} hashes (${rate} H/s, cpu)`);
      return { nonce: '0x' + nonce.toString('hex'), hash: '0x' + hash.toString('hex') };
    }

    attempts++;
    if (attempts % LOG_EVERY === 0) {
      const elapsed = (Date.now() - start) / 1000;
      const rate = (attempts / elapsed).toFixed(0);
      process.stdout.write(`  Mining... ${attempts.toLocaleString()} hashes @ ${rate} H/s (cpu)\r`);
    }
  }

  return null;
}

/**
 * Persistent GPU helper subprocess. One per miner process. Each call writes
 * one job line, then awaits the next stdout line as the result.
 *
 * Jobs are queued — there is no concurrency. The caller is responsible for
 * cancelling (we just stop reading and let the helper finish its current
 * batch; the next job preempts via the AbortController signal).
 */
class GpuMinerProcess {
  constructor(binPath) {
    this.proc = spawn(binPath, [], { stdio: ['pipe', 'pipe', 'inherit'] });
    this.proc.on('exit', (code) => {
      console.error(`  GPU helper exited (code=${code}); falling back to CPU for next jobs`);
      this.dead = true;
    });
    this.buffer = '';
    this.queue = [];
    this.proc.stdout.on('data', (chunk) => {
      this.buffer += chunk.toString('utf8');
      let i;
      while ((i = this.buffer.indexOf('\n')) >= 0) {
        const line = this.buffer.slice(0, i).trim();
        this.buffer = this.buffer.slice(i + 1);
        if (line.length === 0) continue;
        const next = this.queue.shift();
        if (next) next(line);
      }
    });
  }

  /** Send one job and resolve with the parsed result line. */
  call(job) {
    return new Promise((resolve) => {
      this.queue.push((line) => {
        try { resolve(JSON.parse(line)); }
        catch { resolve({ found: false, error: 'bad-json', raw: line }); }
      });
      this.proc.stdin.write(JSON.stringify(job) + '\n');
    });
  }
}

let GPU = null;
function gpuOrInit() {
  if (FORCE_CPU) return null;
  if (!MINER_BIN) return null;
  if (GPU && !GPU.dead) return GPU;
  if (GPU && GPU.dead) return null;
  GPU = new GpuMinerProcess(MINER_BIN);
  return GPU;
}

async function findNonceGpu(settlementRoot, parentHash, difficulty, signal) {
  const gpu = gpuOrInit();
  if (!gpu) return null;

  let start = 0n;
  const start_t = Date.now();
  while (!signal.aborted) {
    const nonceBase = '0x' + crypto.randomBytes(32).toString('hex');
    const job = {
      settlement_root: settlementRoot,
      parent_hash: parentHash,
      target: difficulty,
      nonce_base: nonceBase,
      start: Number(start & 0xffffffffffffffffn),
      batches: 32,
    };
    const result = await gpu.call(job);
    if (signal.aborted) return null;
    if (result.found) {
      const elapsed = (Date.now() - start_t) / 1000;
      const rate = (Number(result.tries) / elapsed).toFixed(0);
      console.log(`  ✓ Found nonce after ${result.tries} hashes (${rate} H/s, gpu)`);
      return { nonce: result.nonce, hash: null };
    }
    start += BigInt(result.tries || 0);
  }
  return null;
}

async function findNonce(settlementRoot, parentHash, difficulty, signal) {
  // Prefer the Rust GPU helper if it built; otherwise fall back to JS CPU loop.
  const gpu = gpuOrInit();
  if (gpu) {
    const r = await findNonceGpu(settlementRoot, parentHash, difficulty, signal);
    if (r) return r;
    if (signal.aborted) return null;
  }
  return findNonceCpu(settlementRoot, parentHash, difficulty, signal);
}

function hashLessThan(hash, difficulty) {
  for (let i = 0; i < 32; i++) {
    if (hash[i] < difficulty[i]) return true;
    if (hash[i] > difficulty[i]) return false;
  }
  return false;
}

async function main() {
  await cryptoWaitReady();

  const provider = new WsProvider(RPC_URL);
  const api      = await ApiPromise.create({ provider });

  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const miner   = MNEMONIC.startsWith('//')
    ? keyring.addFromUri(MNEMONIC)
    : keyring.addFromMnemonic(MNEMONIC);

  console.log('Twill Miner started');
  console.log(`  RPC:    ${RPC_URL}`);
  console.log(`  Miner:  ${miner.address}`);
  if (FORCE_CPU) {
    console.log(`  Engine: cpu (forced via TWILL_MINER=cpu)`);
  } else if (MINER_BIN) {
    console.log(`  Engine: gpu (${MINER_BIN})`);
  } else {
    console.log(`  Engine: cpu fallback — build the GPU helper for real hashpower:`);
    console.log(`            cargo build --release -p twill-miner`);
  }
  console.log('');

  let miningController = null;

  // Subscribe to new blocks and start mining each one
  await api.rpc.chain.subscribeNewHeads(async (header) => {
    const blockNumber = header.number.toNumber();
    // Use the hash of the *current* head (not its parent).
    // The transaction executes in block N+1 where frame_system::parent_hash()
    // returns hash(block N) — i.e. the hash of the block we subscribed to.
    const parentHash  = header.hash.toHex();

    // Abort any ongoing mining for the previous block
    if (miningController) {
      miningController.abort();
    }
    miningController = new AbortController();
    const signal = miningController.signal;

    // Read current state
    const [settlementRootRaw, difficultyRaw, totalMintedRaw] = await Promise.all([
      api.query.mining.currentSettlementRoot(),
      api.query.mining.pocDifficulty(),
      api.query.mining.totalMinted(),
    ]);

    const settlementRoot = settlementRootRaw.toHex();
    const difficulty = difficultyRaw.isSome
      ? difficultyRaw.unwrap().toHex()
      : '0x00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';

    const totalMinted = BigInt(totalMintedRaw.toString());
    const isBootstrap = totalMinted < BOOTSTRAP_THRESHOLD;

    const modeLabel = isBootstrap ? 'bootstrap (fee-free)' : 'standard (signed)';
    console.log(`\nBlock #${blockNumber} — mining [${modeLabel}] (root: ${settlementRoot.slice(0,10)}...)`);

    const result = await findNonce(settlementRoot, parentHash, difficulty, signal);
    if (!result) return; // aborted by next block

    console.log(`  Submitting proof for block #${blockNumber}...`);

    try {
      if (isBootstrap) {
        // Bootstrap: submit unsigned — no fee, PoW is the spam protection
        const tx = api.tx.mining.submitPocProofUnsigned(miner.address, result.nonce, settlementRoot);
        await tx.send(({ status, events }) => {
          if (status.isInBlock) {
            let won = false;
            events.forEach(({ event }) => {
              if (event.section === 'mining' && event.method === 'BlockMined') {
                const [, reward] = event.data;
                const twl = (BigInt(reward.toString()) / BigInt('1000000000000')).toString();
                console.log(`  ✓ Block mined! Reward: ${twl} TWL`);
                won = true;
              }
            });
            if (!won) {
              console.log(`  ✗ Proof included but no reward (beaten to it)`);
            }
          }
        });
      } else {
        // Post-bootstrap: submit signed — small fee required
        await api.tx.mining
          .submitPocProof(result.nonce, settlementRoot)
          .signAndSend(miner, ({ status, events }) => {
            if (status.isInBlock) {
              let won = false;
              events.forEach(({ event }) => {
                if (event.section === 'mining' && event.method === 'BlockMined') {
                  const [, reward] = event.data;
                  const twl = (BigInt(reward.toString()) / BigInt('1000000000000')).toString();
                  console.log(`  ✓ Block mined! Reward: ${twl} TWL`);
                  won = true;
                }
              });
              if (!won) {
                console.log(`  ✗ Proof included but no reward (beaten to it)`);
              }
            }
          });
      }
    } catch (err) {
      if (!signal.aborted) {
        console.error(`  ✗ Submit error: ${err.message}`);
      }
    }
  });
}

main().catch(console.error);

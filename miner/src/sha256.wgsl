// Twill PoC GPU miner — SHA-256 brute force.
//
// Each invocation builds a candidate nonce by mixing the host-supplied 32-byte
// nonce_base with its global invocation index, then hashes
// SHA256(nonce || settlement_root || parent_hash) and compares the digest
// (treated as a big-endian 256-bit integer) against `threshold`. The first
// invocation to find a digest < threshold writes its candidate nonce into
// `out_nonce` via an atomic flag.
//
// Input layout (`Params`):
//   nonce_base       : 32 bytes — host-supplied entropy. The varying counter is
//                                 spliced into the LAST 8 bytes.
//   settlement_root  : 32 bytes
//   parent_hash      : 32 bytes
//   threshold        : 8 big-endian u32 words forming the 256-bit difficulty
//                      threshold, for direct comparison against the digest.
//   start            : u64 starting counter (split into two u32 for WGSL).

struct Params {
    nonce_base: array<u32, 8>,
    settlement_root: array<u32, 8>,
    parent_hash: array<u32, 8>,
    threshold: array<u32, 8>,
    start_lo: u32,
    start_hi: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> params: Params;
@group(0) @binding(1) var<storage, read_write> found_flag: atomic<u32>;
@group(0) @binding(2) var<storage, read_write> out_nonce: array<u32, 8>;

var<private> K: array<u32, 64> = array<u32, 64>(
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u,
    0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
    0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu,
    0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u,
    0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
    0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u,
    0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u,
    0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u
);

const H0: array<u32, 8> = array<u32, 8>(
    0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
    0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u
);

fn rotr(x: u32, n: u32) -> u32 {
    return (x >> n) | (x << (32u - n));
}

fn sha256_compress(state: ptr<function, array<u32, 8>>, w_in: ptr<function, array<u32, 64>>) {
    var w: array<u32, 64>;
    for (var i = 0u; i < 16u; i = i + 1u) {
        w[i] = (*w_in)[i];
    }
    for (var i = 16u; i < 64u; i = i + 1u) {
        let s0 = rotr(w[i - 15u], 7u) ^ rotr(w[i - 15u], 18u) ^ (w[i - 15u] >> 3u);
        let s1 = rotr(w[i - 2u], 17u) ^ rotr(w[i - 2u], 19u) ^ (w[i - 2u] >> 10u);
        w[i] = w[i - 16u] + s0 + w[i - 7u] + s1;
    }
    var a = (*state)[0]; var b = (*state)[1]; var c = (*state)[2]; var d = (*state)[3];
    var e = (*state)[4]; var f = (*state)[5]; var g = (*state)[6]; var h = (*state)[7];
    for (var i = 0u; i < 64u; i = i + 1u) {
        let S1 = rotr(e, 6u) ^ rotr(e, 11u) ^ rotr(e, 25u);
        let ch = (e & f) ^ (~e & g);
        let temp1 = h + S1 + ch + K[i] + w[i];
        let S0 = rotr(a, 2u) ^ rotr(a, 13u) ^ rotr(a, 22u);
        let mj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = S0 + mj;
        h = g; g = f; f = e; e = d + temp1;
        d = c; c = b; b = a; a = temp1 + temp2;
    }
    (*state)[0] = (*state)[0] + a;
    (*state)[1] = (*state)[1] + b;
    (*state)[2] = (*state)[2] + c;
    (*state)[3] = (*state)[3] + d;
    (*state)[4] = (*state)[4] + e;
    (*state)[5] = (*state)[5] + f;
    (*state)[6] = (*state)[6] + g;
    (*state)[7] = (*state)[7] + h;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (atomicLoad(&found_flag) != 0u) { return; }

    // Build the per-invocation nonce: take the host base, then overwrite the
    // last 8 bytes (= last two u32 words) with start_counter + gid.
    let counter_lo = params.start_lo + gid.x;
    let counter_hi = params.start_hi + select(0u, 1u, counter_lo < params.start_lo);

    var nonce: array<u32, 8>;
    for (var i = 0u; i < 6u; i = i + 1u) {
        nonce[i] = params.nonce_base[i];
    }
    nonce[6] = counter_hi;
    nonce[7] = counter_lo;

    // Two-block message (96 bytes payload + 1-byte 0x80 + zero pad + 8-byte length).
    // Block 0: nonce[0..8], settlement_root[0..8] = 16 words
    // Block 1: parent_hash[0..8], 0x80000000, 5 zeros, length_hi=0, length_lo=768 bits
    var state: array<u32, 8> = H0;

    var blk0: array<u32, 64>;
    for (var i = 0u; i < 8u; i = i + 1u) { blk0[i] = nonce[i]; }
    for (var i = 0u; i < 8u; i = i + 1u) { blk0[8u + i] = params.settlement_root[i]; }
    sha256_compress(&state, &blk0);

    var blk1: array<u32, 64>;
    for (var i = 0u; i < 8u; i = i + 1u) { blk1[i] = params.parent_hash[i]; }
    blk1[8] = 0x80000000u;
    for (var i = 9u; i < 14u; i = i + 1u) { blk1[i] = 0u; }
    blk1[14] = 0u;
    blk1[15] = 768u; // 96 bytes * 8 = 768 bits
    sha256_compress(&state, &blk1);

    // Big-endian compare of state (8 words) vs threshold (8 words).
    var winner: bool = false;
    for (var i = 0u; i < 8u; i = i + 1u) {
        if (state[i] < params.threshold[i]) { winner = true; break; }
        if (state[i] > params.threshold[i]) { break; }
    }

    if (winner) {
        // First finder wins; later writes are skipped.
        let prev = atomicCompareExchangeWeak(&found_flag, 0u, 1u);
        if (prev.exchanged) {
            for (var i = 0u; i < 8u; i = i + 1u) {
                out_nonce[i] = nonce[i];
            }
        }
    }
}

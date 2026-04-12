# Twill Network — Community & Board Launch Checklist

This document is the living mandate for the elected board and community.
The chain launches permissionlessly. Everything below is the human layer
that turns working software into a working network.

---

## Phase 0 — Before First Board Election (Anyone Can Do)

The chain is live. The board doesn't exist yet. These tasks happen first.

| # | Task | Who | Status | Notes |
|---|------|-----|--------|-------|
| 0.1 | Deploy mainnet node + public RPC endpoint | Any node operator | READY | Source code published. Node builds from source. Chain spec committed. First node operator runs `cargo build --release` and starts mining. |
| 0.2 | Publish Polkadot.js connection config | Community | DONE | `docs/polkadotjs-config.json` committed to repo. Token: TWL, 12 decimals, SS58 prefix 42. |
| 0.3 | Announce chain launch | Community | PENDING | Post on crypto forums, Nostr, wherever. No official channel — anyone can spread the word. |
| 0.4 | First election announcement | Community | PENDING | Post on all channels: nominations open for 7 days |
| 0.5 | Community members nominate for board | Nominees | PENDING | Requires 100 TWL (mined or acquired). Max 7 seats. |
| 0.6 | Community votes on nominees | TWL holders | PENDING | 1 TWL = 1 vote. 7-day voting window. |

---

## Phase 1 — Board's First Actions

The board is seated. These are mandatory before Twill can operate at scale.

**How the board funds itself:** There is no protocol treasury. The board is funded by:
- A community donation wallet (publicly published, fully transparent). Early miners and believers send what they can.
- Grants from aligned organizations (open-source foundations, carbon market participants, crypto infrastructure funds).
- Once the chain has value, entities that profit from a healthy Twill (miners, exchanges, bridges) have incentive to contribute.

The board does what it can afford, in order of priority. Legal first. Audit when funded. This is how Bitcoin's ecosystem works.

### 1A. Legal Structure

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 1.1 | Form legal entity | CRITICAL | Wyoming DAO LLC or Cayman Foundation Company. Foundation holds no equity — purpose-driven. Cost: ~$2-5k filing, no pre-allocated supply needed. |
| 1.2 | Publish community donation wallet | CRITICAL | A public TWL address (and BTC address) for community contributions. Fully transparent on-chain. This is the board's operating fund. |
| 1.3 | Engage crypto-specialized legal counsel | CRITICAL | Firms: Fenwick & West, Debevoise, Perkins Coie, or local equivalent. Funded by community donations. |
| 1.4 | Obtain token classification legal memo | CRITICAL | Written opinion that TWL is a utility token. Mining-only distribution (100% mined, no pre-mine, no founder allocation) is the strongest possible argument against Howey. |
| 1.5 | Register as a Money Service Business (MSB) if required | HIGH | US FinCEN registration for certain crypto activities. Legal counsel advises. |

### 1B. Security

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 1.6 | Engage external security audit | CRITICAL | Pallets to audit: settlement (HTLC), reserve, governance. Firms: Quarkslab, SR Labs, Oak Security, Zellic. Cost: $50-150k. Funded by community donations + approached as grant to entities with economic interest. |
| 1.7 | Publish audit report | HIGH | Full report published publicly. No hiding findings. |
| 1.8 | Establish bug bounty program | HIGH | Funded from community donation wallet. Minimum: $10k pool for critical findings. |

---

## Phase 2 — Carbon Market Integration

Twill's carbon swap primitive only works if credits are real and verifiable.

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 2.1 | Contact Gold Standard Foundation | HIGH | They are more technically progressive than Verra. Start here. Goal: API access to registry + retirement endpoint. Website: goldstandard.org |
| 2.2 | Contact Verra (VCS Program) | HIGH | Largest voluntary carbon registry (~75% market share). More bureaucratic. Contact: registryservices@verra.org |
| 2.3 | Define on-chain retirement mechanism | HIGH | When tCO2e is retired on Twill, Verra/Gold Standard registry must update. Options: (a) registry API webhook, (b) oracle-confirmed retirement, (c) periodic batch reconciliation. |
| 2.4 | Pilot with one project | MEDIUM | Find one carbon credit project willing to register on Twill as proof-of-concept. A reforestation project in the 10,000-50,000 tCO2e range is ideal. |
| 2.5 | Publish carbon credit specification | MEDIUM | Technical doc explaining how tCO2e works on Twill. Registry partner docs site. |
| 2.6 | Engage compliance carbon market | LOW (later) | EU ETS, California Cap-and-Trade — these require much heavier regulatory engagement. Post-mainnet maturity. |

---

## Phase 3 — Exchange & Liquidity

TWL needs to be acquirable for people to participate in mining and governance.

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 3.1 | DEX listing | HIGH | Uniswap (via bridge) or a Substrate-native DEX. People need to be able to buy TWL to stake, nominate, and vote. |
| 3.2 | CEX outreach | MEDIUM | Gate.io, KuCoin, MEXC are more accessible for new chains. Binance/Coinbase require legal structure, audit, and track record. |
| 3.3 | Market maker engagement | MEDIUM | Spread is everything for a new asset. One market maker providing liquidity is more valuable than 10 DEX pools with no depth. |
| 3.4 | Fiat on-ramp | MEDIUM | MoonPay, Ramp, Transak integration for non-crypto users to buy TWL. Required for mainstream carbon market participants. |

---

## Phase 4 — Bridge Infrastructure

The reserve grows through atomic swaps. For the reserve to work at scale, the bridge layer must exist.

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 4.1 | Define bridge architecture | HIGH | Decision: (a) custodial launch partner (BitGo, Coinbase Custody) for speed, (b) threshold-sig bridge (FROST/MPC) for decentralization. Recommend: custodial for v1, trustless for v2. |
| 4.2 | Bitcoin bridge | HIGH | BTC → reserve Bitcoin address, oracle confirms, TWL released via atomic swap. The settlement pallet is already this — needs oracle confirmation of Bitcoin txids. |
| 4.3 | Ethereum bridge | MEDIUM | ETH/USDC → reserve Ethereum address. Similar oracle model. |
| 4.4 | Reserve address publication | HIGH | Publish the Twill reserve BTC/ETH addresses publicly. Anyone can verify the reserve by checking those addresses on-chain. Trustless proof. |

---

## Governance Rules (Encoded in Protocol)

For reference — what community proposals can and cannot do:

| Action | Who | Threshold |
|--------|-----|-----------|
| Text proposal (signaling) | Any TWL holder + 100 TWL deposit | 50% Aye, 10% quorum |
| Runtime upgrade (code change) | Any TWL holder + 100 TWL deposit | 50% Aye, 10% quorum |
| Board recall | Any TWL holder + 100 TWL deposit | 50% Aye, 10% quorum |
| Board election | Auto-triggered every 5 years | Top 7 nominees by vote weight |

**The board cannot:**
- Unilaterally change the protocol (requires community vote)
- Modify supply (hard-coded in primitives)
- Cancel an election

**The board can:**
- Sign contracts on behalf of the legal entity
- Propose runtime upgrades (community still votes)
- Engage with Verra, Gold Standard, exchanges
- Hire developers and auditors

**If the community wants a protocol-funded treasury later:**
Governance can vote to redirect a portion of settlement fees to a community pool. Example: change `FEE_STAKER_SHARE_BPS` from 10,000 (100%) to 9,500 (95% stakers, 5% community pool). This requires a standard governance proposal and community approval. It is not a pre-mine — it is ongoing earned revenue, transparently governed. Launch without it. Add it only if the community votes for it.

---

## Phase 5 — Cross-Border Settlement Corridors

Twill's settlement protocol works for any two parties with internet access. The board's job is to make this visible to the corridors where broken settlement causes the most pain.

| # | Task | Priority | Notes |
|---|------|----------|-------|
| 5.1 | Identify high-cost remittance corridors | HIGH | US→Mexico, US→Philippines, UAE→South Asia are the highest-volume, highest-fee corridors globally. These are the first target markets. |
| 5.2 | Contact payment facilitators in target corridors | HIGH | Local exchange operators, mobile money providers, and crypto OTC desks who already serve these corridors. Twill gives them atomic settlement rails they don't have today. |
| 5.3 | Carbon market integration outreach (banks) | HIGH | Institutional desks building carbon trading infrastructure need settlement rails. First mover contact with trading desks at HSBC, JPMorgan, and boutique carbon brokers. |
| 5.4 | Publish cross-border settlement case studies | MEDIUM | Show BTC↔Carbon atomic swap. Show cross-corridor TWL settlement. Real examples, real numbers, real comparison to SWIFT. |
| 5.5 | Legal review of settlement in key jurisdictions | HIGH | Settlement law varies by jurisdiction. Legal memo covering US, EU, Singapore, UAE on whether Twill settlement constitutes a regulated activity. |
| 5.6 | Partner with carbon registry on first live settlement | HIGH | A live atomic delivery-versus-payment swap of a carbon credit on Twill, witnessed by a registry, is worth more than any whitepaper. |

---

## Audit Requirements

Security audits are non-negotiable before the network handles significant value. This is the standard the board must meet.

### Pallets Requiring Audit (in priority order)

| Pallet | Risk Level | Reason |
|--------|-----------|--------|
| `pallet-settlement` | CRITICAL | Holds escrowed assets during HTLC. Lock/execute/refund logic must be airtight. |
| `pallet-mining` | CRITICAL | Block reward issuance and staker slashing. Any exploit mints supply or destroys stake. |
| `pallet-reserve` | HIGH | Tracks reserve vault valuation. Incorrect accounting breaks the floor price mechanism. |
| `pallet-carbon` | HIGH | Bond slashing and credit invalidation. Fraudulent credits undermine the carbon market. |
| `pallet-governance` | HIGH | Vote counting and proposal execution. Governance exploits can change any protocol parameter. |
| `pallet-oracle` | MEDIUM | Price manipulation can distort reserve valuation. |

### Audit Firms (Substrate-Experienced)

- **SR Labs** — Substrate specialists. Audited Polkadot, Moonbeam, Acala.
- **Oak Security** — Rust and FRAME specialists. Audited multiple Substrate pallets.
- **Quarkslab** — Cryptographic protocol experts. Strong on HTLC/hashlock logic.
- **Zellic** — Blockchain security. Cross-chain bridge experience directly relevant to settlement.

### Audit Process

1. **Scope definition** — Board publishes exact commit hash to be audited.
2. **Firm engagement** — Competitive process. Two firms minimum for critical pallets.
3. **Public findings** — All findings published in full. No suppression.
4. **Remediation** — Fixes submitted as governance proposals. Community votes before deployment.
5. **Re-audit** — Critical and high findings require re-audit of the fix.
6. **Ongoing** — Every runtime upgrade touching audited pallets requires incremental re-audit of changed code.

### Bug Bounty

Minimum pool: 10,000 TWL for critical findings (post-mining pool established).
Scope: All pallets in production. Report to the board's published contact address.
Response time commitment: 48 hours acknowledgement, 14 days triage.

---

## Mission

> Twill is the cross-border atomic settlement rail. Any asset. Any border. No intermediary.
> Built by miners and holders. Owned by nobody.

---

*Board: maintain this document as a live checklist. Check off tasks as completed.
Community: hold the board to this. That is governance.*

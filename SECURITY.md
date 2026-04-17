# Security Policy

Twill is a permissionless Layer 1 with no admin keys. Bugs in the runtime, the consensus path, the settlement engine, or the bridges can move real value. Coordinated disclosure protects the network and gives operators time to patch.

## Reporting a Vulnerability

If you believe you've found a vulnerability, please **report it privately first**. Do not open a public issue, post in chat, or publish a write-up until the network has had a chance to patch.

Two channels:

1. **Encrypted email** (preferred):
   `security@twill.network` — PGP key fingerprint will be published in the repo at `docs/security-key.asc` once the community board ratifies the contact rotation.
2. **Direct message a board member** on the community Discord (`#security` channel exists for coordination only — do not post vulnerability details there).

Please include:

- A description of the issue and the affected component (pallet, runtime version, node binary, miner script, bridge contract, etc.).
- Steps to reproduce, including the chain spec / dev-mode invocation if possible.
- The impact you believe the issue has (consensus break, supply inflation, fund loss, governance bypass, oracle manipulation, etc.).
- Whether you've shared this information anywhere else.

## Scope

In scope:

- All pallets in `pallets/`
- The runtime in `runtime/`
- The node binary in `node/`
- Chain spec generation in `node/src/chain_spec.rs` and `scripts/build-mainnet-spec.sh`
- The miner crate (when published) and the JS miner script in `scripts/`
- Documented bridge contracts once published
- Genesis configuration

Out of scope:

- Issues in third-party dependencies (report upstream — we'll track and pull patches).
- Vulnerabilities that require physical access to a victim's machine.
- Denial-of-service against a single node that does not affect the network as a whole.
- Issues in the polkadot-sdk itself unless we've patched it locally.

## What to Expect

- An initial acknowledgement within a few days.
- A coordinated patch timeline. Critical bugs (consensus, supply, fund loss) get the fastest turnaround the community can manage.
- Public disclosure after the patch ships, with credit to the reporter unless they'd prefer to remain anonymous.

## Bounties

There is no funded bug bounty at genesis — there is no treasury and no foundation. The community board may propose a bounty program funded by the governance share of settlement fees once the chain is live. Until then, reporters get acknowledgement, credit, and the satisfaction of having protected real value.

## Supported Versions

Only the latest tagged release on the main branch is actively supported. Run a current node — old binaries will not be patched in place.

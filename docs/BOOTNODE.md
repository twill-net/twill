# Running a Twill Bootnode

Bootnodes are the discovery layer of the network. A new node that starts for
the first time needs at least one reachable peer to join the gossip mesh — that
is the bootnode's only job. They do not validate. They do not mine. They do
not hold balances or private keys beyond their node identity. They exist to
answer one question: *"I'm new here, who else is out there?"*

More bootnodes means faster peer discovery, lower risk of the network ever
being unreachable, and no single operator can unilaterally gate who connects.

---

## Why run one

- **You care about the network's availability.** If every miner depends on one
  bootnode and that bootnode goes down, nobody new can join until it's back.
- **You want to help decentralize discovery.** The genesis bootnode is
  operated by the founder. Every additional bootnode reduces reliance on that
  single point.
- **You already run a full node** and want it to help newcomers.

Running a bootnode does not earn block rewards. This is infrastructure, not
mining. If you want rewards, run a miner (see `JOIN.md`).

---

## Requirements

| Resource | Minimum |
|----------|---------|
| OS | Ubuntu 22.04 LTS (glibc 2.35), fresh install |
| RAM | 1 GB (2 GB recommended) |
| Disk | 20 GB SSD |
| CPU | 1 vCPU (2 recommended for faster initial build) |
| Network | Static public IPv4, port `30333/tcp` reachable |
| Uptime expectation | 24/7 — you're advertising yourself as reachable |

Any cheap VPS works: Vultr, Hetzner, DigitalOcean, OVH, Scaleway. Budget target
is ~$5–10/month.

---

## One-shot setup

SSH into your fresh VPS as root and run:

```bash
curl -L https://raw.githubusercontent.com/twill-net/twill/main/scripts/bootnode-setup.sh -o /tmp/bootnode-setup.sh
bash /tmp/bootnode-setup.sh
```

The script will:

1. Install build dependencies (`clang`, `protobuf-compiler`, `rustup`).
2. Create a 4 GB swapfile (Substrate link step is memory-hungry on small boxes).
3. Configure `ufw` to allow only `22/tcp` (SSH) and `30333/tcp` (p2p), and
   enable `fail2ban`.
4. Create a non-root `twill` user and copy your SSH authorized_keys.
5. Install Rust and build `twill` from source (30–90 minutes on 2 vCPU).
6. Generate a stable ed25519 node key at
   `/home/twill/chain-data/chains/mainnet/network/secret_ed25519`.
7. Install and start a `twill-bootnode` systemd service that auto-restarts.
8. Print your public multiaddr at the end, e.g.:

```
/ip4/203.0.113.7/tcp/30333/p2p/12D3KooWExampleExampleExampleExampleExample
```

**This multiaddr is your bootnode's identity. Share it with the community.**

---

## Key backup

The single file that matters is:

```
/home/twill/chain-data/chains/mainnet/network/secret_ed25519
```

If you lose it, your bootnode comes back with a different PeerId — and
everyone who had your old multiaddr in their config can no longer find you.

Back it up to an offline location:

```bash
scp root@your-bootnode:/home/twill/chain-data/chains/mainnet/network/secret_ed25519 \
    ~/twill-bootnode-key.bak
chmod 600 ~/twill-bootnode-key.bak
```

---

## Getting listed

After your bootnode is confirmed reachable, announce it:

- **Discord:** post your multiaddr in `#bootnodes` (or the general channel if
  that doesn't exist yet).
- **GitHub issue:** open an issue titled `Add bootnode: <your node name>` on
  <https://github.com/twill-net/twill/issues> with the multiaddr and your
  public contact (Discord handle, email, or GitHub username). This creates a
  public record of who operates it.

New miners and full-node operators can pass your multiaddr to their node via:

```bash
./twill --chain mainnet-raw.json \
        --bootnodes /ip4/203.0.113.7/tcp/30333/p2p/12D3KooWExample...
```

If enough bootnodes come online and prove reliable, the chain spec itself
(`mainnet-raw.json`) gets updated on the next release to embed the additional
multiaddrs, so new joiners pick them up by default.

---

## Health checks

Once running, verify from any other machine:

```bash
# TCP reachability
nc -zv -w 3 your-bootnode.example.com 30333

# The bootnode's systemd status (from the box itself)
ssh root@your-bootnode 'systemctl status twill-bootnode | head -20'

# Recent logs
ssh root@your-bootnode 'journalctl -u twill-bootnode -n 50 --no-pager'
```

A healthy bootnode log shows:

```
📋 Chain specification: Twill Network
🔨 Initializing Genesis block/state (... header-hash: 0x5c94…b001)
🏷  Local node identity is: 12D3KooW...
🔍 Discovered new external address for our node: /ip4/.../tcp/30333/p2p/...
```

The genesis header-hash **must** be `0x5c94…b001`. If yours differs, you built
from an old fork or applied a wrong chain spec — rebuild from a clean clone of
`main` at tag `v0.2.4` or later.

---

## Common issues

**Build fails with OOM killer:**
The swapfile step in the setup script exists for this. If you ran the script
before it was added, create one manually:
```bash
fallocate -l 4G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile
echo '/swapfile none swap sw 0 0' >> /etc/fstab
```

**Bootnode starts but no peers discover it:**
- Confirm port 30333 is open at the VPS provider firewall *and* in `ufw`.
- `ufw status` should show `30333/tcp ALLOW`.
- Try `nc -zv your-ip 30333` from an external machine.

**PeerId changes every restart:**
You are using `--tmp` or the node key file is missing. The service unit
installed by `bootnode-setup.sh` passes `--node-key-file` explicitly to avoid
this. Verify with `systemctl cat twill-bootnode`.

**GLIBC version error after updating:**
You built on Ubuntu 24.04 (glibc 2.39) and tried to run on 22.04 (glibc 2.35).
Rebuild on the box itself, or build in a 22.04 container. CI release artifacts
are built on 22.04 for this reason.

---

## Retiring a bootnode

If you need to stop operating:

1. Give 2 weeks' notice in Discord so nobody's config depends on yours.
2. Stop the service: `systemctl stop twill-bootnode && systemctl disable twill-bootnode`.
3. Close the GitHub issue that announced your multiaddr.

Your node key is still a credential — delete it or keep it secured. If you
come back online later with the same key, the same multiaddr works again.

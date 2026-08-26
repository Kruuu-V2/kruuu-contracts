# kruuu-contracts

CosmWasm smart contracts for Kruuu on [Verona](https://docs.verona.dev) (the chain formerly known as XION).

## Contracts

### `certificate-registry`

Anchors Kruuu certifications on-chain. Design decisions:

- **Registry, not NFT.** Certificates are soulbound records keyed by an opaque
  `cert_id` (`cert:{certeficateToUsers.id}` — 1:1 with the backend row). No
  transfers, no wallet custody required from talents or institutes.
- **Recipients are opaque Kruuu user ids**, never wallet addresses. The legacy
  email-derived addresses are forgeable by anyone who knows the email and must
  never authorize anything on-chain.
- **Governance and issuance are separate powers.**
  - The **owner** (a cw3-flex multisig backed by a cw4-group: 2-of-3 —
    two founders + one cold-storage backup key) controls the issuer allowlist
    and ownership transfer, and holds the CosmWasm migration admin.
  - **Issuers** (the backend relayer key, held in a secret manager, never in
    git) can only issue and revoke. A leaked relayer key can spam
    certificates but can never take over the contract; the multisig rotates
    it with one proposal.
- **Backfill-ready.** `issue_batch` (≤100/tx, atomic) accepts an explicit
  `issued_at` so historical web2 certificates keep their real dates.
- **Revocation preserves the record** — `revoked` flag + timestamp + reason,
  still queryable, so a revoked certificate is provably revoked rather than
  merely missing.
- Each record carries a `content_hash` (sha256 of the canonical metadata
  JSON) plus an optional `metadata_uri` (IPFS), so anyone can verify the
  off-chain document against the chain.

## Develop

```bash
cargo test          # unit + cw-multi-test integration tests
cargo wasm          # release build for wasm32-unknown-unknown
```

On Windows without MSVC Build Tools, use the GNU toolchain:
`rustup override set stable-x86_64-pc-windows-gnu` (already set in this
checkout). `.cargo/config.toml` carries the `--allow-undefined` linker flag
newer rust-lld needs to emit the CosmWasm host functions as wasm imports.

Production artifacts must be built with [cosmwasm/optimizer](https://github.com/CosmWasm/optimizer)
for reproducible, size-optimized wasm before storing on-chain.

## Deployment order (testnet first)

1. Store + instantiate `cw4-group` (members: the two founder keys + backup key).
2. Store + instantiate `cw3-flex-multisig` (group = step 1, threshold 2).
3. Store `certificate-registry`; instantiate with `owner` = the multisig
   address, `issuers` = [relayer address], and **CosmWasm admin = the
   multisig** (`--admin` flag) so migrations also require 2-of-3.
4. Fund a treasury/fee grant for the relayer's gas.
5. Rehearse on testnet before mainnet: rotate the issuer via proposal,
   swap a group member, run a contract migration.

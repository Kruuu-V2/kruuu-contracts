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

## Generate a signer key (founders + backup)

```bash
git clone https://github.com/Kruuu-V2/kruuu-contracts.git
cd kruuu-contracts
bun install
bun run generate-key
```

Runs fully offline. Write the 24-word mnemonic on paper (twice-checked),
never store it digitally, and share **only** the printed `xion1...` address.
Run it once per signer; the backup key's paper goes somewhere that isn't a
founder's laptop.

## Develop

```bash
cargo test          # unit + cw-multi-test integration tests
cargo wasm          # release build for wasm32-unknown-unknown
```

`rust-toolchain.toml` pins Rust **1.86.0**. Rust 1.87+ ships a wasm32 std
compiled with bulk-memory instructions, which wasmd's static validation
rejects (`bulk memory support is not enabled`) no matter what target
features the contract itself is built with. On Windows without MSVC Build
Tools use the GNU host: `rustup override set 1.86.0-x86_64-pc-windows-gnu`.
`.cargo/config.toml` carries the `--allow-undefined` linker flag rust-lld
needs to emit the CosmWasm host functions as wasm imports, and disables the
post-MVP wasm features for the contract crate.

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

## Deploy to testnet

```bash
cargo wasm                      # build the registry
bun run fetch-artifacts         # + cw4-group / cw3-flex from cw-plus v2.0.0
bun run generate-key            # once for the deployer, once for the relayer
# fund the deployer at https://faucet.xion.burnt.com (2 XION / 24h)

DEPLOYER_MNEMONIC="..." \
RELAYER_ADDRESS=xion1... \
MEMBERS=xion1founderA,xion1founderB,xion1backup \
THRESHOLD=2 \
bun run deploy

bun run check                   # proves owner/admin/issuer wiring
bun run balance xion1...        # uxion balance of any address
RELAYER_MNEMONIC="..." bun run issue '{"cert_id":"cert:1","institution_id":1,"recipient_id":2,"template_id":3}'
bun run cert cert:1             # read a record back
```

Mainnet (`NETWORK=mainnet`) has governance-gated code uploads
(`code_upload_access: Nobody`). cw4-group and cw3-flex-multisig v2.0.0 are
already stored there (codes 10 and 9, checksums verified); the registry wasm
from the CI "Reproducible wasm build" must be stored through governance, then:

```bash
NETWORK=mainnet CW4_GROUP_CODE_ID=10 CW3_FLEX_CODE_ID=9 REGISTRY_CODE_ID=<from governance> MEMBERS=... RELAYER_ADDRESS=... bun run deploy
```

The script verifies each preset code id's on-chain checksum against the local
artifact before instantiating.

`deployments/<chain-id>.json` is committed; mnemonics never are. Fees use
a 2x gas multiplier (`GAS_MULTIPLIER` in `scripts/lib/chain.ts`): the
testnet's simulation under-reports state-write gas by ~50%.

### xion-testnet-2 (deployed 2026-08-30)

| | address |
|---|---|
| registry | `xion1sjj73a9smku0uadrqvgqmhv8kvu0z26sd9uf40x4k2y2aa2px8pssuqhzp` |
| multisig (owner + admin) | `xion152ahzty7vns6gj85uspdgdmkmghp5emzk7r5l25uc66tqafqpvrqnd8l7m` |
| group (members) | `xion18acawa4ly3z9q60lnfppvjmxlmdstzz50awzugndzky00f3kl2yquf8rur` |
| relayer (issuer) | `xion1l2ghvypk85fqdwhypu02nlz48lr7enq4dlze3s` |

Explorer: `https://explorer.burnt.com/xion-testnet-2/account/<address>`.

## Governance (2-of-3 rehearsal)

Every action is: one member proposes → members vote → anyone executes.
Members need a little gas (faucet once).

```bash
# member A
SIGNER_MNEMONIC="..." bun run gov:propose "Rotate relayer" \
  '{"update_issuers":{"add":["xion1new"],"remove":["xion1old"]}}'
# member B
SIGNER_MNEMONIC="..." bun run gov:vote <id> yes       # proposer's vote is implicit
# anyone
SIGNER_MNEMONIC="..." bun run gov:execute <id>
bun run gov:proposals
```

Targets: registry messages (default), `--target group` for membership
(`update_members`), `--target registry-migrate` for code upgrades.

### One-command rehearsal (no third member needed)

Threshold is 2, so one person holding two member keys can run the whole
propose → vote → execute loop alone. It adds a throwaway issuer and removes
it again, proving the multisig controls the registry:

```bash
bun run gov:rehearse
```

It prompts for each mnemonic with hidden input (paste the 24 words, press
Enter). Setting `PROPOSER_MNEMONIC` / `VOTER_MNEMONIC` in the environment
skips the prompts. Both keys need a little gas. Membership and code are
never touched.

Every script that signs (`deploy`, `issue`, `gov:*`) prompts the same way
when its `*_MNEMONIC` variable is not set.

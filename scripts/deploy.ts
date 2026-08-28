/**
 * Full deployment ceremony. Produces deployments/<chain-id>.json.
 *
 * env:
 *   DEPLOYER_MNEMONIC  throwaway funded key; holds no power once this finishes
 *   MEMBERS            comma-separated multisig member addresses
 *   THRESHOLD          votes required to pass (default 2)
 *   RELAYER_ADDRESS    backend issuer address
 *   NETWORK            testnet (default)
 *
 * Order matters: the group is created with the deployer as a temporary admin
 * only because the multisig needs the group address to exist first. Admin is
 * handed to the multisig right after, and the registry is born owned and
 * migration-administered by the multisig — the deployer never holds either.
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { deploymentPath, explorerTx, requireEnv, signer, type Deployment } from './lib/chain'

const members = requireEnv('MEMBERS').split(',').map((s) => s.trim()).filter(Boolean)
const threshold = Number(process.env.THRESHOLD ?? 2)
const relayer = requireEnv('RELAYER_ADDRESS')
if (members.length < threshold) throw new Error(`threshold ${threshold} exceeds ${members.length} members`)

const { client, address: deployer, config } = await signer('DEPLOYER_MNEMONIC')
console.log(`deployer ${deployer} on ${config.chainId}`)
const balance = await client.getBalance(deployer, config.denom)
console.log(`balance ${balance.amount}${config.denom}`)
if (Number(balance.amount) < 500_000) {
  throw new Error('deployer needs at least 0.5 XION; use the faucet at https://faucet.xion.burnt.com')
}

async function store(name: string) {
  const wasm = readFileSync(`artifacts/${name}`)
  const res = await client.upload(deployer, wasm, 'auto', `kruuu ${name}`)
  console.log(`stored ${name} → code ${res.codeId}  ${explorerTx(res.transactionHash)}`)
  return res.codeId
}

const codeIds = {
  cw4Group: await store('cw4_group.wasm'),
  cw3FlexMultisig: await store('cw3_flex_multisig.wasm'),
  certificateRegistry: await store('certificate_registry.wasm'),
}

const group = await client.instantiate(
  deployer,
  codeIds.cw4Group,
  { admin: deployer, members: members.map((addr) => ({ addr, weight: 1 })) },
  'kruuu-governance-group',
  'auto',
  { admin: deployer },
)
console.log(`group      ${group.contractAddress}`)

const multisig = await client.instantiate(
  deployer,
  codeIds.cw3FlexMultisig,
  {
    group_addr: group.contractAddress,
    threshold: { absolute_count: { weight: threshold } },
    max_voting_period: { time: 7 * 24 * 3600 },
    executor: null,
    proposal_deposit: null,
  },
  'kruuu-governance-multisig',
  'auto',
  { admin: deployer },
)
console.log(`multisig   ${multisig.contractAddress}`)

// Hand the group to the multisig: membership changes now need a passed vote.
await client.execute(deployer, group.contractAddress, { update_admin: { admin: multisig.contractAddress } }, 'auto')
await client.updateAdmin(deployer, group.contractAddress, multisig.contractAddress, 'auto')
await client.updateAdmin(deployer, multisig.contractAddress, multisig.contractAddress, 'auto')
console.log('group + multisig admin → multisig')

const registry = await client.instantiate(
  deployer,
  codeIds.certificateRegistry,
  { owner: multisig.contractAddress, issuers: [relayer] },
  'kruuu-certificate-registry',
  'auto',
  { admin: multisig.contractAddress },
)
console.log(`registry   ${registry.contractAddress}`)

const deployment: Deployment = {
  chainId: config.chainId,
  deployedAt: new Date().toISOString(),
  deployer,
  codeIds,
  contracts: {
    group: group.contractAddress,
    multisig: multisig.contractAddress,
    registry: registry.contractAddress,
  },
  members,
  threshold,
  relayer,
}
writeFileSync(deploymentPath(), JSON.stringify(deployment, null, 2) + '\n')
console.log(`\nwrote ${deploymentPath()}\nnext: bun run check`)

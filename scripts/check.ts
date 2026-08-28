/** Proves the chain of custody after deploy: who owns and administers what. */
import { loadDeployment, reader } from './lib/chain'

const d = loadDeployment()
const { client } = await reader()

const registryConfig = await client.queryContractSmart(d.contracts.registry, { config: {} })
const registryInfo = await client.getContract(d.contracts.registry)
const groupAdmin = await client.queryContractSmart(d.contracts.group, { admin: {} })
const groupMembers = await client.queryContractSmart(d.contracts.group, { list_members: {} })
const multisigThreshold = await client.queryContractSmart(d.contracts.multisig, { threshold: {} })

const ok = (cond: boolean) => (cond ? '✓' : '✗')
console.log(`${ok(registryConfig.owner === d.contracts.multisig)} registry owner is the multisig`)
console.log(`${ok(registryInfo.admin === d.contracts.multisig)} registry migration admin is the multisig`)
console.log(`${ok(groupAdmin.admin === d.contracts.multisig)} group admin is the multisig`)
console.log(`${ok(registryConfig.issuers.includes(d.relayer))} relayer is an issuer`)
console.log(`${ok(!registryConfig.issuers.includes(d.deployer))} deployer is NOT an issuer`)
console.log(`  members   ${groupMembers.members.map((m: { addr: string }) => m.addr).join(', ')}`)
console.log(`  threshold ${JSON.stringify(multisigThreshold)}`)
console.log(`  issued    ${registryConfig.total_issued}`)

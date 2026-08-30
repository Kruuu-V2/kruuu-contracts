/**
 * Full 2-of-3 governance rehearsal, run by one person holding two member keys
 * (or two people on one machine). Nothing here needs the third member.
 *
 * It rotates the issuer allowlist through a throwaway address and back:
 *   1. PROPOSER opens "add issuer X"        2. VOTER votes yes (threshold met)
 *   3. VOTER executes                        4. registry config shows X
 *   5. PROPOSER opens "remove issuer X"     6. VOTER votes yes, executes
 *   7. registry config no longer shows X
 * Membership and code are never touched, so the worst case is a harmless
 * extra issuer that the second half removes again.
 *
 *   PROPOSER_MNEMONIC="..." VOTER_MNEMONIC="..." bun run gov:rehearse
 *
 * Both keys must be group members with a little gas (bun run balance <addr>).
 */
import { toUtf8 } from '@cosmjs/encoding'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { loadDeployment, explorerTx, signer, GAS_MULTIPLIER } from '../lib/chain'

const d = loadDeployment()
const proposer = await signer('PROPOSER_MNEMONIC')
const voter = await signer('VOTER_MNEMONIC')

if (proposer.address === voter.address) {
  throw new Error('PROPOSER_MNEMONIC and VOTER_MNEMONIC are the same key; the rehearsal needs two members')
}

const members: { members: { addr: string; weight: number }[] } = await proposer.client.queryContractSmart(
  d.contracts.group,
  { list_members: {} },
)
for (const who of [proposer, voter]) {
  if (!members.members.some((m) => m.addr === who.address)) {
    throw new Error(`${who.address} is not a member of the group`)
  }
  const balance = await who.client.getBalance(who.address, 'uxion')
  if (Number(balance.amount) < 50_000) {
    throw new Error(`${who.address} has ${balance.amount} uxion, fund it first (bun run balance ${who.address})`)
  }
}

const throwaway = await DirectSecp256k1HdWallet.generate(12, { prefix: 'xion' })
const [{ address: tempIssuer }] = await throwaway.getAccounts()

const issuers = async () => {
  const config: { issuers: string[] } = await proposer.client.queryContractSmart(d.contracts.registry, { config: {} })
  return config.issuers
}

const registryExecute = (msg: unknown) => ({
  wasm: {
    execute: {
      contract_addr: d.contracts.registry,
      msg: Buffer.from(toUtf8(JSON.stringify(msg))).toString('base64'),
      funds: [],
    },
  },
})

const runProposal = async (title: string, msg: unknown) => {
  console.log(`\n▶ ${title}`)
  const opened = await proposer.client.execute(
    proposer.address,
    d.contracts.multisig,
    { propose: { title, description: title, msgs: [registryExecute(msg)] } },
    GAS_MULTIPLIER,
  )
  const id = Number(opened.events.flatMap((e) => e.attributes).find((a) => a.key === 'proposal_id')?.value)
  console.log(`  proposal #${id} opened by ${proposer.address}  ${explorerTx(opened.transactionHash)}`)

  const voted = await voter.client.execute(
    voter.address,
    d.contracts.multisig,
    { vote: { proposal_id: id, vote: 'yes' } },
    GAS_MULTIPLIER,
  )
  const proposal = await voter.client.queryContractSmart(d.contracts.multisig, { proposal: { proposal_id: id } })
  console.log(`  ${voter.address} voted yes  ${explorerTx(voted.transactionHash)}  status: ${proposal.status}`)
  if (proposal.status !== 'passed') throw new Error(`proposal #${id} did not pass (status ${proposal.status})`)

  const executed = await voter.client.execute(
    voter.address,
    d.contracts.multisig,
    { execute: { proposal_id: id } },
    GAS_MULTIPLIER,
  )
  console.log(`  executed  ${explorerTx(executed.transactionHash)}`)
  return id
}

console.log(`registry  ${d.contracts.registry}`)
console.log(`multisig  ${d.contracts.multisig}  threshold 2`)
console.log(`proposer  ${proposer.address}`)
console.log(`voter     ${voter.address}`)
console.log(`throwaway issuer for this rehearsal: ${tempIssuer}`)
console.log(`issuers before: ${(await issuers()).join(', ')}`)

await runProposal('Rehearsal: add temporary issuer', { update_issuers: { add: [tempIssuer], remove: [] } })
const afterAdd = await issuers()
if (!afterAdd.includes(tempIssuer)) throw new Error('issuer was not added; the multisig does not control the registry')
console.log(`  ✓ registry now lists ${tempIssuer}`)

await runProposal('Rehearsal: remove temporary issuer', { update_issuers: { add: [], remove: [tempIssuer] } })
const afterRemove = await issuers()
if (afterRemove.includes(tempIssuer)) throw new Error('issuer was not removed')
console.log(`  ✓ registry back to: ${afterRemove.join(', ')}`)

console.log('\nRehearsal passed: two member keys proposed, voted, executed, and the registry obeyed both times.')

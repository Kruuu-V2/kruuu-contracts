/** Lists proposals and their votes. No key needed. */
import { loadDeployment, reader } from '../lib/chain'

const d = loadDeployment()
const { client } = await reader()
const { proposals } = await client.queryContractSmart(d.contracts.multisig, { list_proposals: { limit: 30 } })
if (proposals.length === 0) console.log('no proposals yet')
for (const p of proposals) {
  const { votes } = await client.queryContractSmart(d.contracts.multisig, { list_votes: { proposal_id: p.id } })
  const tally = votes.map((v: { voter: string; vote: string }) => `${v.voter.slice(0, 12)}…:${v.vote}`).join(' ')
  console.log(`#${p.id}  ${p.status.padEnd(9)} ${p.title}  [${tally}]`)
}

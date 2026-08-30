/** env: SIGNER_MNEMONIC. usage: bun run gov:vote <proposal-id> yes|no|abstain */
import { loadDeployment, explorerTx, signer, GAS_MULTIPLIER } from '../lib/chain'

const [id, vote = 'yes'] = process.argv.slice(2)
if (!id) throw new Error('usage: vote <proposal-id> [yes|no|abstain]')

const d = loadDeployment()
const { client, address } = await signer('SIGNER_MNEMONIC')
const res = await client.execute(
  address,
  d.contracts.multisig,
  { vote: { proposal_id: Number(id), vote } },
  GAS_MULTIPLIER,
)
console.log(`${address} voted ${vote} on #${id}  ${explorerTx(res.transactionHash)}`)
const proposal = await client.queryContractSmart(d.contracts.multisig, { proposal: { proposal_id: Number(id) } })
console.log(`status: ${proposal.status}${proposal.status === 'passed' ? '  →  bun run gov:execute ' + id : ''}`)

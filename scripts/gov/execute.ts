/** env: SIGNER_MNEMONIC (anyone). usage: bun run gov:execute <proposal-id> */
import { loadDeployment, explorerTx, signer, GAS_MULTIPLIER } from '../lib/chain'

const [id] = process.argv.slice(2)
if (!id) throw new Error('usage: execute <proposal-id>')

const d = loadDeployment()
const { client, address } = await signer('SIGNER_MNEMONIC')
const res = await client.execute(
  address,
  d.contracts.multisig,
  { execute: { proposal_id: Number(id) } },
  GAS_MULTIPLIER,
)
console.log(`proposal #${id} executed by ${address}  ${explorerTx(res.transactionHash)}`)

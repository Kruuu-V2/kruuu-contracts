/**
 * Open a multisig proposal that executes a message on the registry.
 *
 * env: SIGNER_MNEMONIC (any group member)
 * usage:
 *   bun run gov:propose "Rotate relayer" '{"update_issuers":{"add":["xion1new"],"remove":["xion1old"]}}'
 *   bun run gov:propose "Add co-founder" '{"update_members":{"add":[{"addr":"xion1...","weight":1}],"remove":[]}}' --target group
 *   bun run gov:propose "Migrate registry" '{"migrate":{"new_code_id":42}}' --target registry-migrate
 */
import { toUtf8 } from '@cosmjs/encoding'
import { loadDeployment, explorerTx, signer, GAS_MULTIPLIER } from '../lib/chain'

const [title, rawMsg, ...flags] = process.argv.slice(2)
if (!title || !rawMsg) throw new Error('usage: propose <title> <json-msg> [--target registry|group|registry-migrate]')
const target = flags.includes('--target') ? flags[flags.indexOf('--target') + 1] : 'registry'

const d = loadDeployment()
const { client, address } = await signer('SIGNER_MNEMONIC')
const msg = JSON.parse(rawMsg)

let cosmosMsg: unknown
if (target === 'registry-migrate') {
  cosmosMsg = {
    wasm: {
      migrate: {
        contract_addr: d.contracts.registry,
        new_code_id: msg.migrate.new_code_id,
        msg: Buffer.from(toUtf8(JSON.stringify(msg.migrate.msg ?? {}))).toString('base64'),
      },
    },
  }
} else {
  const contract_addr = target === 'group' ? d.contracts.group : d.contracts.registry
  cosmosMsg = {
    wasm: {
      execute: {
        contract_addr,
        msg: Buffer.from(toUtf8(JSON.stringify(msg))).toString('base64'),
        funds: [],
      },
    },
  }
}

const res = await client.execute(
  address,
  d.contracts.multisig,
  { propose: { title, description: title, msgs: [cosmosMsg] } },
  GAS_MULTIPLIER,
)
const id = res.events
  .flatMap((e) => e.attributes)
  .find((a) => a.key === 'proposal_id')?.value
console.log(`proposal #${id} opened by ${address}  ${explorerTx(res.transactionHash)}`)
console.log(`others vote with:  bun run gov:vote ${id} yes`)

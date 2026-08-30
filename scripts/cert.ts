/**
 * Reads one certificate record from the registry.
 *
 *   bun run cert cert:1
 */
import { loadDeployment, reader } from './lib/chain'

const certId = process.argv[2]
if (!certId) {
  console.error('usage: bun run cert <cert_id>')
  process.exit(1)
}

const { client } = await reader()
const deployment = loadDeployment()
const res = await client.queryContractSmart(deployment.contracts.registry, { certificate: { cert_id: certId } })
console.log(JSON.stringify(res, null, 2))

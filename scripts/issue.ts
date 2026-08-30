/**
 * Issues one certificate through the relayer (smoke test / manual repair).
 *
 *   RELAYER_MNEMONIC="..." bun run issue '{"cert_id":"cert:1","institution_id":1,"recipient_id":2,"template_id":3,"content_hash":"<64 hex>","metadata_uri":null,"issued_at":null}'
 */
import { createHash } from 'node:crypto'
import { loadDeployment, explorerTx, signer, GAS_MULTIPLIER } from './lib/chain'

const raw = process.argv[2]
if (!raw) {
  console.error('usage: bun run issue <CertificateInput json>')
  process.exit(1)
}
const cert = JSON.parse(raw)
if (!cert.content_hash) {
  cert.content_hash = createHash('sha256').update(JSON.stringify(cert)).digest('hex')
}

const { client, address } = await signer('RELAYER_MNEMONIC')
const deployment = loadDeployment()

const res = await client.execute(address, deployment.contracts.registry, { issue: { cert } }, GAS_MULTIPLIER)
console.log(`issued ${cert.cert_id}  gas ${res.gasUsed}  ${explorerTx(res.transactionHash)}`)

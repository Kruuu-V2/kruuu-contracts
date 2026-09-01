/**
 * Store artifacts/certificate_registry.wasm without instantiating anything.
 * For upgrading an existing deployment: store the new code, then open a
 * migration proposal with the printed code id:
 *
 *   bun run store
 *   bun run gov:propose "Upgrade registry" '{"migrate":{"new_code_id":<id>}}' --target registry-migrate
 *
 * env: DEPLOYER_MNEMONIC (any funded key; storing grants it no power), NETWORK
 */
import { readFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { explorerTx, signer, GAS_MULTIPLIER } from './lib/chain'

const wasm = readFileSync('artifacts/certificate_registry.wasm')
const checksum = createHash('sha256').update(wasm).digest('hex')

const { client, address, config } = await signer('DEPLOYER_MNEMONIC')
console.log(`deployer ${address} on ${config.chainId}`)
console.log(`artifact ${wasm.length} bytes, sha256 ${checksum}`)

const balance = await client.getBalance(address, config.denom)
if (Number(balance.amount) < 100_000) {
  throw new Error(`balance ${balance.amount}${config.denom} too low; storing needs ~0.1 XION`)
}

const res = await client.upload(address, wasm, GAS_MULTIPLIER, 'kruuu certificate_registry.wasm')
console.log(`stored → code ${res.codeId}  ${explorerTx(res.transactionHash)}`)
console.log(`next: bun run gov:propose "Upgrade registry" '{"migrate":{"new_code_id":${res.codeId}}}' --target registry-migrate`)

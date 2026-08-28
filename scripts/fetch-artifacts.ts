/**
 * Collects the three wasm artifacts into ./artifacts:
 *   - cw4_group.wasm and cw3_flex_multisig.wasm from the pinned cw-plus release
 *   - certificate_registry.wasm from `cargo wasm` (run that first)
 */
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { network } from './lib/chain'

const { cwPlusVersion } = network()
const base = `https://github.com/CosmWasm/cw-plus/releases/download/${cwPlusVersion}`
mkdirSync('artifacts', { recursive: true })

for (const name of ['cw4_group.wasm', 'cw3_flex_multisig.wasm']) {
  const target = `artifacts/${name}`
  if (existsSync(target)) {
    console.log(`✓ ${name} (cached)`)
    continue
  }
  const res = await fetch(`${base}/${name}`)
  if (!res.ok) throw new Error(`download failed for ${name}: ${res.status}`)
  writeFileSync(target, new Uint8Array(await res.arrayBuffer()))
  console.log(`✓ ${name} (${cwPlusVersion})`)
}

const built = 'target/wasm32-unknown-unknown/release/certificate_registry.wasm'
if (!existsSync(built)) throw new Error(`${built} missing — run \`cargo wasm\` first`)
copyFileSync(built, 'artifacts/certificate_registry.wasm')
console.log('✓ certificate_registry.wasm (local build)')

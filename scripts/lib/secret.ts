/**
 * Reads a secret from the terminal without echoing it. Used as the fallback
 * when a *_MNEMONIC env var is not set, so nobody has to fight their shell
 * to pass 24 words: just run the script and paste at the prompt.
 */
const normalize = (raw: string) => raw.trim().replace(/\s+/g, ' ')

export function readSecret(label: string): Promise<string> {
  const stdin = process.stdin
  process.stdout.write(`${label}: `)

  if (!stdin.isTTY) {
    return new Promise((resolve) => {
      let data = ''
      stdin.setEncoding('utf8')
      stdin.on('data', (chunk) => (data += chunk))
      stdin.on('end', () => resolve(normalize(data.split('\n')[0] ?? '')))
    })
  }

  return new Promise((resolve) => {
    let buffer = ''
    stdin.setRawMode(true)
    stdin.resume()
    stdin.setEncoding('utf8')

    const finish = () => {
      stdin.off('data', onData)
      stdin.setRawMode(false)
      stdin.pause()
      process.stdout.write('\n')
      resolve(normalize(buffer))
    }

    const onData = (chunk: string) => {
      for (const ch of chunk) {
        if (ch === '') process.exit(1)
        if (ch === '\n' && buffer.length === 0) continue
        if (ch === '\r' || ch === '\n') return finish()
        if (ch === '' || ch === '\b') {
          buffer = buffer.slice(0, -1)
          continue
        }
        buffer += ch
      }
    }

    stdin.on('data', onData)
  })
}

export async function readMnemonic(name: string): Promise<string> {
  const fromEnv = process.env[name]
  if (fromEnv) return normalize(fromEnv)

  const mnemonic = await readSecret(`${name} (paste the 24 words, input is hidden)`)
  const words = mnemonic ? mnemonic.split(' ').length : 0
  if (words !== 12 && words !== 24) {
    throw new Error(`${name}: expected 24 words, received ${words}`)
  }
  console.log(`  received ${words} words`)
  return mnemonic
}

/**
 * Generates a fresh Verona (XION) key pair, fully offline.
 *
 * Usage:   bun install && bun run generate-key
 *
 * Prints a 24-word mnemonic and its address. NOTHING is written to disk and
 * nothing leaves this machine. Write the mnemonic on paper, store it safely,
 * and share ONLY the address (xion1...) — the address is public information,
 * the mnemonic is the key itself.
 */
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'

const PREFIX = 'xion'

const wallet = await DirectSecp256k1HdWallet.generate(24, { prefix: PREFIX })
const [account] = await wallet.getAccounts()

const line = '─'.repeat(66)

console.log(`
${line}
  Verona (XION) key generated — OFFLINE, nothing was sent anywhere
${line}

  YOUR ADDRESS (public — this is what you share):

      ${account.address}

${line}
  YOUR MNEMONIC (secret — write on paper, then close this terminal):

      ${wallet.mnemonic.split(' ').slice(0, 8).join(' ')}
      ${wallet.mnemonic.split(' ').slice(8, 16).join(' ')}
      ${wallet.mnemonic.split(' ').slice(16, 24).join(' ')}

${line}
  RULES
  1. Write all 24 words on paper, in order. Check them twice.
  2. Never screenshot, never paste into chat, never store in a note app.
  3. Share only the xion1... address above.
  4. Anyone holding these 24 words IS this key. There is no reset.
${line}
`)

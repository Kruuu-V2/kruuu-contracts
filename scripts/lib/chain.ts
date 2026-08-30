import { readFileSync } from 'node:fs'
import { SigningCosmWasmClient, CosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'

export interface ChainConfig {
  chainId: string
  rpc: string
  denom: string
  gasPrice: string
  prefix: string
  cwPlusVersion: string
}

export const NETWORKS: Record<string, ChainConfig> = {
  testnet: {
    chainId: 'xion-testnet-2',
    rpc: 'https://rpc.xion-testnet-2.burnt.com:443',
    denom: 'uxion',
    gasPrice: '0.001uxion',
    prefix: 'xion',
    cwPlusVersion: 'v2.0.0',
  },
}

export function network(): ChainConfig {
  const name = process.env.NETWORK ?? 'testnet'
  const config = NETWORKS[name]
  if (!config) throw new Error(`unknown NETWORK "${name}" (known: ${Object.keys(NETWORKS).join(', ')})`)
  return config
}

export function requireEnv(name: string): string {
  const value = process.env[name]
  if (!value) throw new Error(`missing env ${name}`)
  return value
}

/** Signing client for the key in the given env var (mnemonic never touches disk). */
export const GAS_MULTIPLIER = 2

export async function signer(mnemonicEnv: string) {
  const config = network()
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(requireEnv(mnemonicEnv), {
    prefix: config.prefix,
  })
  const [account] = await wallet.getAccounts()
  const client = await SigningCosmWasmClient.connectWithSigner(config.rpc, wallet, {
    gasPrice: GasPrice.fromString(config.gasPrice),
  })
  return { client, address: account.address, config }
}

export async function reader() {
  const config = network()
  return { client: await CosmWasmClient.connect(config.rpc), config }
}

export interface Deployment {
  chainId: string
  deployedAt: string
  deployer: string
  codeIds: { cw4Group: number; cw3FlexMultisig: number; certificateRegistry: number }
  contracts: { group: string; multisig: string; registry: string }
  members: string[]
  threshold: number
  relayer: string
}

export function deploymentPath() {
  return `deployments/${network().chainId}.json`
}

export function loadDeployment(): Deployment {
  return JSON.parse(readFileSync(deploymentPath(), 'utf8'))
}

export function explorerTx(hash: string) {
  return `https://explorer.burnt.com/${network().chainId}/tx/${hash}`
}

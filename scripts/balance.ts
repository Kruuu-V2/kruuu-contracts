import { StargateClient } from '@cosmjs/stargate'
import { network } from './lib/chain'
const config = network()
const c = await StargateClient.connect(config.rpc)
console.log(`network ${config.chainId}`)
for (const a of process.argv.slice(2)) console.log(a, (await c.getBalance(a, config.denom)).amount, config.denom)

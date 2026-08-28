import { StargateClient } from '@cosmjs/stargate'
const c = await StargateClient.connect('https://rpc.xion-testnet-2.burnt.com:443')
for (const a of process.argv.slice(2)) console.log(a, (await c.getBalance(a, 'uxion')).amount, 'uxion')

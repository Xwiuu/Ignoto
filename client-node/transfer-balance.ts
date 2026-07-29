import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';

async function main() {
  const wsProvider = new WsProvider('ws://127.0.0.1:9944');
  const api = await ApiPromise.create({ provider: wsProvider });
  await cryptoWaitReady();

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  const bob = keyring.addFromUri('//Bob');

  console.log('[+] Submitting Balances.transfer...');
  const tx = api.tx.balances.transferAllowDeath(bob.address, 12345);
  await tx.signAndSend(alice, ({ status, events }) => {
    if (status.isInBlock || status.isFinalized) {
      console.log(`[+] Tx in block.`);
      events.forEach(({ event }) => {
        console.log(`    > ${event.section}.${event.method}`);
      });
      api.disconnect();
    }
  });
}

main().catch(err => {
  console.error('[-] Error:', err);
  process.exit(1);
});

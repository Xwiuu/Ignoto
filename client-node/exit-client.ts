import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';

async function main() {
  console.log('Triggering ShieldedWithdraw...');
  const wsProvider = new WsProvider('ws://127.0.0.1:9944');
  const api = await ApiPromise.create({ provider: wsProvider });
  await cryptoWaitReady();

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  const amount = 42069000000000;
  const address = '44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3X';

  const tx = api.tx.ignoto.shieldedWithdraw(amount, address);
  await tx.signAndSend(alice, ({ status, events }) => {
    if (status.isInBlock || status.isFinalized) {
      console.log(`[+] Tx included in block.`);
      events.forEach(({ event }) => {
        if (event.section === 'ignoto' && event.method === 'ShieldedWithdraw') {
          console.log('[+] ShieldedWithdraw event emitted!');
        }
      });
      api.disconnect();
    }
  });
}

main().catch(console.error);

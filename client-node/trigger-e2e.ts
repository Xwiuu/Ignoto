import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';

async function main() {
  const wsProvider = new WsProvider('ws://127.0.0.1:9944');
  const api = await ApiPromise.create({ provider: wsProvider });
  console.log('[+] Connected to Substrate node.');
  await cryptoWaitReady();

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  const mockCommitment1 = '0x' + '11'.repeat(32);
  const mockCommitment2 = '0x' + '22'.repeat(32);
  const mockCommitment3 = '0x' + '33'.repeat(32);
  const mockCommitment4 = '0x' + '44'.repeat(32);
  const mockNullifier1 = '0x' + '55'.repeat(32);
  const mockNullifier2 = '0x' + '66'.repeat(32);
  const proof = '0x' + '77'.repeat(128);
  const vk = '0x' + '88'.repeat(64);

  const tx = api.tx.ignoto.transferShielded(
    proof, vk,
    [mockCommitment1, mockCommitment2],
    [mockCommitment3, mockCommitment4],
    [mockNullifier1, mockNullifier2]
  );

  console.log('[+] Submitting transferShielded extrinsic...');
  await tx.signAndSend(alice, ({ status, events }) => {
    if (status.isInBlock || status.isFinalized) {
      console.log(`[+] Tx in block: ${status.isInBlock ? 'InBlock' : 'Finalized'}`);
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

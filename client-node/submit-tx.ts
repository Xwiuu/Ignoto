import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import axios from 'axios';

async function main() {
  console.log('🚀 Initializing Ignoto Client...');

  // 1. Initialize WebSocket provider and Polkadot API
  const wsProvider = new WsProvider('ws://127.0.0.1:9944');
  const api = await ApiPromise.create({ provider: wsProvider });
  console.log('🛰️ Connected to Substrate Node via WebSocket.');

  // Wait for cryptographic WASM to be ready (required for keyring)
  await cryptoWaitReady();

  // 2. Define standard mock data in hex format
  // commitments and nullifiers are arrays of [u8; 32]
  const mockCommitment1 = '0x' + '11'.repeat(32);
  const mockCommitment2 = '0x' + '22'.repeat(32);
  const mockCommitment3 = '0x' + '33'.repeat(32);
  const mockCommitment4 = '0x' + '44'.repeat(32);
  const mockNullifier1 = '0x' + '55'.repeat(32);
  const mockNullifier2 = '0x' + '66'.repeat(32);

  const inputsCommitment = [mockCommitment1, mockCommitment2];
  const outputsCommitment = [mockCommitment3, mockCommitment4];
  const inputsNullifier = [mockNullifier1, mockNullifier2];
  const proof = '0x' + '77'.repeat(128); // mock Groth16 proof bytes (Vec<u8>)
  const vk = '0x' + '88'.repeat(64);    // mock Verification Key bytes (Vec<u8>)

  // 3. Inspect the Pallet metadata to check exact argument names and types
  console.log('\n📖 Pallet Metadata Definition:');
  const methodMeta = api.tx.ignoto.transferShielded.meta;
  methodMeta.args.forEach((arg) => {
    console.log(` - ${arg.name.toString()}: ${arg.type.toString()}`);
  });

  // 4. Construct the Extrinsic call dynamically based on expected parameters
  let tx;
  const numArgs = methodMeta.args.length;
  console.log(`\n🛠️ Constructing extrinsic call with ${numArgs} arguments...`);

  if (numArgs === 5) {
    // Rust signature: transfer_shielded(origin, proof_bytes, vk_bytes, inputs_commitment, outputs_commitment, inputs_nullifier)
    tx = api.tx.ignoto.transferShielded(
      proof,
      vk,
      inputsCommitment,
      outputsCommitment,
      inputsNullifier
    );
  } else {
    // User requested order: transferShielded(inputsCommitment, outputsCommitment, inputsNullifier, proof)
    tx = api.tx.ignoto.transferShielded(
      inputsCommitment,
      outputsCommitment,
      inputsNullifier,
      proof
    );
  }

  // 5. Sign the transaction using standard dev account (Alice)
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');
  console.log(`🔑 Signing transaction using Alice developer account (${alice.address})...`);
  
  await tx.signAsync(alice);

  // 6. Convert the signed extrinsic to pure SCALE-encoded hex
  const hexExtrinsic = tx.toHex();
  console.log(`\n📦 Generated SCALE-encoded Extrinsic Hex:\n${hexExtrinsic}`);

  // Disconnect WebSocket once metadata and transaction construction is complete
  await api.disconnect();
  console.log('🔌 Disconnected from Substrate WebSocket.');

  // 7. Construct JSON-RPC 2.0 envelope
  const jsonRpcEnvelope = {
    jsonrpc: '2.0',
    method: 'author_submitExtrinsic',
    params: [hexExtrinsic],
    id: 1,
  };

  // 8. Send HTTP POST to Go Router
  console.log('\n📤 Dispatching JSON-RPC envelope to Go Router (http://localhost:8080/route-transaction)...');
  try {
    const response = await axios.post(
      'http://localhost:8080/route-transaction',
      jsonRpcEnvelope,
      {
        headers: { 'Content-Type': 'application/json' },
      }
    );
    console.log('✅ Final response received from Go Router:');
    console.log(JSON.stringify(response.data, null, 2));
  } catch (error: any) {
    console.error('❌ Error sending request through Go Router:');
    if (error.response) {
      console.error(`HTTP Status Code: ${error.response.status}`);
      console.error(JSON.stringify(error.response.data, null, 2));
    } else {
      console.error(error.message);
    }
  }
}

main().catch((err) => {
  console.error('❌ Fatal error in script:', err);
});

# ArthaJS - Official TypeScript SDK for ArthaChain

Production-ready TypeScript SDK for interacting with ArthaChain's decentralized storage, identity, and AI infrastructure.

## Features

✅ **Secure Local Signing** - Never transmits private keys over the network  
✅ **Full Identity Support** - DIDs, Verifiable Credentials, AI IDs  
✅ **Decentralized Storage** - SVDB with encryption and retrieval proofs  
✅ **AI Cloud** - Training jobs, inference, federated learning  
✅ **Type-Safe** - Full TypeScript support with comprehensive types  
✅ **Production-Ready** - No mocks, placeholders, or TODOs  

## Installation

```bash
npm install arthajs
```

## Quick Start

### Storage Operations

```typescript
import { ArthaJS } from 'arthajs';

const sdk = new ArthaJS('https://node.arthachain.io');

// Upload a file
const cid = await sdk.uploadFile('./myfile.txt');
console.log(`Uploaded: artha://${cid}`);

// Download a file
await sdk.downloadToFile(`artha://${cid}`, './downloaded.txt');

// Get file info
const info = await sdk.info(`artha://${cid}`);
console.log(info);
```

### Blockchain Interactions (with Local Signing)

```typescript
import { ArthaJS, TransactionSigner } from 'arthajs';

const signer = new TransactionSigner(
  'YOUR_PRIVATE_KEY', // Stays local, never transmitted
  'https://rpc.arthachain.io'
);

const sdk = new ArthaJS('https://node.arthachain.io');

// Settle a storage deal (signs locally)
const tx = await sdk.settle({
  signer,
  dealMarket: '0x...', // Contract address
  manifestRoot: '0x...',
  bytesServed: 1000000,
  provider: '0x...',
  totalWei: BigInt('1000000000000000000')
});

console.log(`Transaction hash: ${tx.hash}`);
await tx.wait(); // Wait for confirmation
```

### Identity Management

```typescript
import { ArthaID, TransactionSigner } from 'arthajs';

const signer = new TransactionSigner(
  'YOUR_PRIVATE_KEY',
  'https://rpc.arthachain.io'
);

const identity = new ArthaID('https://node.arthachain.io', signer);

// Create a DID
const { did, txHash } = await identity.createDID(
  '0x...authKey',
  '0x...encKey',
  '0x...metaCid',
  '0x...contractAddress'
);

console.log(`Created DID: ${did}`);
```

### Verifiable Credentials

```typescript
import { ArthaVC, TransactionSigner } from 'arthajs';

const signer = new TransactionSigner(
  'YOUR_PRIVATE_KEY',
  'https://rpc.arthachain.io'
);

const vc = new ArthaVC('https://node.arthachain.io', signer);

// Issue a credential
const { vcHash, txHash } = await vc.issueVC(
  '0x...contractAddress',
  'did:artha:0x...',  // Subject DID
  '0x...claimHash',
  '0x...docCid',
  Math.floor(Date.now() / 1000) + 31536000 // Expires in 1 year
);
```

### AI Operations

```typescript
import { ArthaJob } from 'arthajs';

const jobs = new ArthaJob('https://node.arthachain.io');

// Submit a training job
const job = await jobs.submitTrain({
  modelId: 'model:abc123',
  datasetId: 'dataset:xyz789',
  submitterDid: 'did:artha:0x...',
  epochs: 10,
  batchSize: 32,
  learningRate: 0.001,
  optimizer: 'adam',
  budget: 100
});

console.log(`Job ID: ${job.jobId}`);

// Check status
const status = await jobs.getStatus(job.jobId);
console.log(status);
```

### Dataset & Model Registry

```typescript
import { ArthaDataset, ArthaModel, TransactionSigner } from 'arthajs';

const signer = new TransactionSigner(
  'YOUR_PRIVATE_KEY',
  'https://rpc.arthachain.io'
);

// Register a dataset
const dataset = new ArthaDataset('https://node.arthachain.io', signer);
const datasetId = await dataset.register(
  '0x...contractAddress',
  '0x...rootCid',
  '0x...licenseCid',
  ['ml', 'nlp', 'public']
);

// Register a model
const model = new ArthaModel('https://node.arthachain.io', signer);
const modelId = await model.register('0x...contractAddress', {
  modelCid: '0x...',
  architecture: 'transformer',
  datasetId: datasetId,
  codeHash: '0x...',
  version: 'v1.0.0'
});
```

## API Reference

### Core Classes

#### `ArthaJS`
Main class for storage operations and blockchain interactions.

**Methods:**
- `uploadFile(filePath: string): Promise<string>`
- `downloadToFile(cidUri: string, outPath: string, range?: {start?: number, end?: number}): Promise<{status:number, bytes:number}>`
- `info(cidUri: string): Promise<any>`
- `settle(params): Promise<TransactionResponse>`
- `buildMerkleBranch(cid: string, index: number): Promise<{root, leaf, branch, index}>`
- `submitPayout(params): Promise<TransactionResponse>`

#### `ArthaID`
Decentralized identity management.

**Methods:**
- `createDID(authKey, encKey, metaCid, contract): Promise<{did, txHash}>`
- `rotateKeys(contract, newAuthKey, newEncKey): Promise<{txHash}>`
- `revokeDID(contract): Promise<{txHash}>`
- `verifySignature(did, messageHash, signature): Promise<{valid}>`

#### `ArthaVC`
Verifiable credentials management.

**Methods:**
- `issueVC(contract, subjectDid, claimHash, docCid, expiresAt): Promise<{vcHash, txHash}>`
- `revokeVC(contract, vcHash): Promise<{txHash}>`
- `verifyVC(vcHash): Promise<{valid, vc}>`
- `getVCsBySubject(subjectDid): Promise<{vcs}>`

#### `TransactionSigner`
Helper class for secure local signing.

**Constructor:**
```typescript
new TransactionSigner(privateKey: string, rpcUrl: string)
```

**Methods:**
- `signAndSend(tx): Promise<TransactionResponse>`
- `call(tx): Promise<string>`
- `getAddress(): string`
- `getNonce(): Promise<number>`
- `estimateGas(tx): Promise<bigint>`

## Security Best Practices

### ✅ DO:
- Use `TransactionSigner` for all blockchain operations
- Store private keys securely (environment variables, key management services)
- Verify transaction receipts after sending
- Use try-catch for error handling

### ❌ DON'T:
- Hardcode private keys in source code
- Share private keys or mnemonics
- Skip transaction confirmation
- Ignore error messages

## Environment Variables

```bash
# Optional - default node URL
ARTHA_NODE=https://node.arthachain.io

# For blockchain operations
ARTHA_RPC_URL=https://rpc.arthachain.io
ARTHA_PRIVATE_KEY=0x...
```

## CLI Usage

```bash
# Upload a file
ARTHA_NODE=https://node.arthachain.io node dist/index.js upload myfile.txt

# Get file info
node dist/index.js info artha://QmXYZ...

# Download a file
node dist/index.js download artha://QmXYZ... output.txt
```

## TypeScript Configuration

The SDK is built with TypeScript and includes full type definitions. Your `tsconfig.json` should include:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ES2022",
    "moduleResolution": "node",
    "esModuleInterop": true,
    "strict": true
  }
}
```

## Error Handling

All methods throw errors with descriptive messages:

```typescript
try {
  const cid = await sdk.uploadFile('./file.txt');
} catch (error) {
  console.error('Upload failed:', error.message);
  // Handle specific errors
  if (error.message.includes('404')) {
    console.log('Endpoint not found');
  }
}
```

## Support

- Documentation: https://docs.arthachain.io
- GitHub: https://github.com/arthachain/arthajs
- Discord: https://discord.gg/arthachain

## License

MIT

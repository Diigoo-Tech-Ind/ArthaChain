# Client-Side Encryption for SVDB

**Complete guide to end-to-end encryption in ArthaChain SVDB**

## Overview

SVDB supports client-side encryption where data is encrypted before upload and decrypted after retrieval. Keys never leave the client, ensuring true end-to-end encryption.

## Key Derivation

### 1. Master Key Generation

```typescript
import { argon2id } from '@noble/hashes/argon2';
import { randomBytes } from '@noble/hashes/utils';

// Generate master seed from user passphrase
function deriveMasterKey(passphrase: string, salt?: Uint8Array): Uint8Array {
  if (!salt) {
    salt = randomBytes(32); // Store this salt securely!
  }
  
  // Argon2id parameters (OWASP recommended)
  const masterKey = argon2id(passphrase, salt, {
    t: 3,        // iterations
    m: 65536,    // memory (64 MB)
    p: 4,        // parallelism
    dkLen: 32    // output length
  });
  
  return masterKey;
}
```

### 2. Derive Encryption Keys from DID

```typescript
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';

// Derive dataset-specific encryption key from master key + DID
function deriveDatasetKey(
  masterKey: Uint8Array,
  did: string,
  datasetId: string
): Uint8Array {
  const info = `artha-dataset-key-${did}-${datasetId}`;
  
  const datasetKey = hkdf(sha256, masterKey, undefined, info, 32);
  return datasetKey;
}

// Derive file-specific encryption key
function deriveFileKey(
  datasetKey: Uint8Array,
  filePath: string
): Uint8Array {
  const info = `artha-file-key-${filePath}`;
  
  const fileKey = hkdf(sha256, datasetKey, undefined, info, 32);
  return fileKey;
}
```

## Encryption

### XChaCha20-Poly1305 (Recommended)

```typescript
import { xchacha20poly1305 } from '@noble/ciphers/chacha';
import { randomBytes } from '@noble/hashes/utils';

async function encryptFile(
  plaintext: Uint8Array,
  key: Uint8Array
): Promise<{ ciphertext: Uint8Array; nonce: Uint8Array }> {
  // Generate random nonce (24 bytes for XChaCha20)
  const nonce = randomBytes(24);
  
  // Encrypt with authenticated encryption
  const cipher = xchacha20poly1305(key, nonce);
  const ciphertext = cipher.encrypt(plaintext);
  
  return { ciphertext, nonce };
}

async function decryptFile(
  ciphertext: Uint8Array,
  key: Uint8Array,
  nonce: Uint8Array
): Promise<Uint8Array> {
  const cipher = xchacha20poly1305(key, nonce);
  const plaintext = cipher.decrypt(ciphertext);
  
  if (!plaintext) {
    throw new Error('Decryption failed: invalid key or corrupted data');
  }
  
  return plaintext;
}
```

### AES-256-GCM (Alternative)

```typescript
async function encryptFileAES(
  plaintext: Uint8Array,
  key: Uint8Array
): Promise<{ ciphertext: Uint8Array; iv: Uint8Array; tag: Uint8Array }> {
  // Generate random IV (12 bytes for GCM)
  const iv = randomBytes(12);
  
  // Import key for Web Crypto API
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    key,
    { name: 'AES-GCM' },
    false,
    ['encrypt']
  );
  
  // Encrypt
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv, tagLength: 128 },
    cryptoKey,
    plaintext
  );
  
  // Split ciphertext and tag (last 16 bytes)
  const ctArray = new Uint8Array(ciphertext);
  const tag = ctArray.slice(-16);
  const ct = ctArray.slice(0, -16);
  
  return { ciphertext: ct, iv, tag };
}
```

## Key Storage

### Browser (IndexedDB)

```typescript
// Store encrypted master key in IndexedDB
async function storeEncryptedKey(
  userId: string,
  masterKey: Uint8Array,
  passphrase: string
): Promise<void> {
  // Derive key encryption key from passphrase
  const kekSalt = randomBytes(32);
  const kek = await deriveMasterKey(passphrase, kekSalt);
  
  // Encrypt master key
  const { ciphertext, nonce } = await encryptFile(masterKey, kek);
  
  // Store in IndexedDB
  const db = await openDB('artha-keys', 1);
  await db.put('keys', {
    userId,
    encryptedKey: ciphertext,
    nonce,
    kekSalt,
    createdAt: Date.now()
  });
}
```

### Hardware Security Module (HSM)

```typescript
// For enterprise: store keys in HSM
interface HSMProvider {
  generateKey(keyId: string): Promise<void>;
  encrypt(keyId: string, plaintext: Uint8Array): Promise<Uint8Array>;
  decrypt(keyId: string, ciphertext: Uint8Array): Promise<Uint8Array>;
}

class YubiHSMProvider implements HSMProvider {
  async generateKey(keyId: string): Promise<void> {
    // Generate key in YubiHSM
    // Key never leaves hardware
  }
  
  async encrypt(keyId: string, plaintext: Uint8Array): Promise<Uint8Array> {
    // Encrypt using HSM
    return new Uint8Array(); // Implementation specific
  }
  
  async decrypt(keyId: string, ciphertext: Uint8Array): Promise<Uint8Array> {
    // Decrypt using HSM
    return new Uint8Array();
  }
}
```

## Complete Upload Flow

```typescript
import { ArthaSVDB } from 'arthajs';

async function uploadEncryptedFile(
  svdb: ArthaSVDB,
  filePath: string,
  plaintext: Uint8Array,
  did: string,
  masterKey: Uint8Array
): Promise<string> {
  // 1. Generate dataset ID
  const datasetId = `dataset-${Date.now()}`;
  
  // 2. Derive encryption keys
  const datasetKey = deriveDatasetKey(masterKey, did, datasetId);
  const fileKey = deriveFileKey(datasetKey, filePath);
  
  // 3. Encrypt file
  const { ciphertext, nonce } = await encryptFile(plaintext, fileKey);
  
  // 4. Create metadata
  const metadata = {
    filePath,
    nonce: Buffer.from(nonce).toString('base64'),
    algorithm: 'xchacha20-poly1305',
    keyDerivation: 'hkdf-sha256',
    encrypted: true
  };
  
  // 5. Upload to SVDB
  const cid = await svdb.upload(ciphertext, metadata);
  
  console.log(`✅ Uploaded encrypted file: ${cid}`);
  return cid;
}
```

## Complete Download Flow

```typescript
async function downloadAndDecryptFile(
  svdb: ArthaSVDB,
  cid: string,
  did: string,
  masterKey: Uint8Array
): Promise<Uint8Array> {
  // 1. Download from SVDB
  const { data, metadata } = await svdb.download(cid);
  
  // 2. Verify encryption metadata
  if (!metadata.encrypted) {
    return data; // Not encrypted
  }
  
  // 3. Extract nonce
  const nonce = Buffer.from(metadata.nonce, 'base64');
  
  // 4. Derive decryption key
  const datasetKey = deriveDatasetKey(masterKey, did, metadata.datasetId || '');
  const fileKey = deriveFileKey(datasetKey, metadata.filePath);
  
  // 5. Decrypt
  const plaintext = await decryptFile(data, fileKey, nonce);
  
  console.log(`✅ Decrypted file from ${cid}`);
  return plaintext;
}
```

## Access Control Integration

### Token-Gated Encryption

```typescript
// Encrypt with VC-based access control
async function encryptWithAccessControl(
  plaintext: Uint8Array,
  requiredVC: string
): Promise<{ ciphertext: Uint8Array; policy: any }> {
  // 1. Generate ephemeral key
  const ephemeralKey = randomBytes(32);
  
  // 2. Encrypt data
  const { ciphertext, nonce } = await encryptFile(plaintext, ephemeralKey);
  
  // 3. Encrypt ephemeral key with VC holder's public key
  // (Retrieved from VCRegistry + DIDRegistry)
  const vcHolderPubKey = await getVCHolderPublicKey(requiredVC);
  const encryptedKey = await encryptWithPublicKey(ephemeralKey, vcHolderPubKey);
  
  // 4. Create access policy
  const policy = {
    type: 'vc-gated',
    requiredVC,
    encryptedKey: Buffer.from(encryptedKey).toString('base64'),
    nonce: Buffer.from(nonce).toString('base64')
  };
  
  return { ciphertext, policy };
}
```

## Key Rotation

```typescript
async function rotateEncryptionKey(
  oldKey: Uint8Array,
  cids: string[]
): Promise<{ newKey: Uint8Array; rotatedCids: string[] }> {
  // 1. Generate new key
  const newKey = randomBytes(32);
  
  const rotatedCids: string[] = [];
  
  for (const cid of cids) {
    // 2. Download and decrypt with old key
    const { data } = await svdb.download(cid);
    const plaintext = await decryptFile(data, oldKey, /* nonce */);
    
    // 3. Re-encrypt with new key
    const { ciphertext } = await encryptFile(plaintext, newKey);
    
    // 4. Upload and update reference
    const newCid = await svdb.upload(ciphertext);
    rotatedCids.push(newCid);
  }
  
  return { newKey, rotatedCids };
}
```

## Security Best Practices

### ✅ DO:
- Use Argon2id for password-based key derivation
- Generate unique nonces for every encryption operation
- Store nonces alongside ciphertext (they're not secret)
- Use authenticated encryption (AEAD): XChaCha20-Poly1305 or AES-GCM
- Derive per-file keys using HKDF
- Rotate keys periodically (every 90 days)
- Use hardware security modules for high-value keys

### ❌ DON'T:
- Reuse nonces with the same key (catastrophic for stream ciphers)
- Store master keys in plaintext
- Use ECB mode (ever!)
- Roll your own crypto
- Trust client-provided encryption (always verify)

## Platform-Specific Implementations

### Node.js

```typescript
import { createCipheriv, createDecipheriv, randomBytes } from 'crypto';

function encryptNodeJS(plaintext: Buffer, key: Buffer): { ciphertext: Buffer; iv: Buffer; tag: Buffer } {
  const iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', key, iv);
  
  const ciphertext = Buffer.concat([
    cipher.update(plaintext),
    cipher.final()
  ]);
  
  const tag = cipher.getAuthTag();
  
  return { ciphertext, iv, tag };
}
```

### React Native

```typescript
import * as Crypto from 'expo-crypto';

async function encryptReactNative(
  plaintext: string,
  key: string
): Promise<string> {
  const encrypted = await Crypto.digestStringAsync(
    Crypto.CryptoDigestAlgorithm.SHA256,
    plaintext + key
  );
  
  // Use react-native-aes-crypto for proper encryption
  return encrypted;
}
```

## Testing

```typescript
describe('Client-Side Encryption', () => {
  it('should encrypt and decrypt correctly', async () => {
    const plaintext = new Uint8Array([1, 2, 3, 4, 5]);
    const key = randomBytes(32);
    
    const { ciphertext, nonce } = await encryptFile(plaintext, key);
    const decrypted = await decryptFile(ciphertext, key, nonce);
    
    expect(decrypted).toEqual(plaintext);
  });
  
  it('should fail with wrong key', async () => {
    const plaintext = new Uint8Array([1, 2, 3]);
    const key1 = randomBytes(32);
    const key2 = randomBytes(32);
    
    const { ciphertext, nonce } = await encryptFile(plaintext, key1);
    
    await expect(
      decryptFile(ciphertext, key2, nonce)
    ).rejects.toThrow();
  });
});
```

## References

- [OWASP Key Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html)
- [Argon2 RFC 9106](https://datatracker.ietf.org/doc/html/rfc9106)
- [XChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha)
- [HKDF RFC 5869](https://datatracker.ietf.org/doc/html/rfc5869)

## Support

For questions: [https://docs.arthachain.online/encryption](https://docs.arthachain.online/encryption)


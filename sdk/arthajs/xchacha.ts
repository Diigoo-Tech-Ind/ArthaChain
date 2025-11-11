import { writeFileSync, readFileSync } from 'fs';
import { randomBytes } from '@stablelib/random';
import { wipe } from '@stablelib/wipe';
import { XChaCha20Poly1305 } from '@stablelib/xchacha20poly1305';

export function encryptFileToEnvelope(key: Uint8Array, inPath: string, outPath: string) {
  const nonce = randomBytes(24);
  const aad = randomBytes(16);
  const aead = new XChaCha20Poly1305(key);
  const plaintext = readFileSync(inPath);
  const sealed = aead.seal(nonce, plaintext, aad);
  writeFileSync(outPath, sealed);
  const envelope = {
    alg: 'XChaCha20-Poly1305',
    nonce_b64: Buffer.from(nonce).toString('base64'),
    aad_b64: Buffer.from(aad).toString('base64'),
  };
  wipe(key);
  return envelope;
}

export function decryptFileFromEnvelope(key: Uint8Array, envelope: { nonce_b64: string; aad_b64?: string }, inPath: string, outPath: string) {
  const nonce = Buffer.from(envelope.nonce_b64, 'base64');
  const aad = envelope.aad_b64 ? Buffer.from(envelope.aad_b64, 'base64') : undefined;
  const aead = new XChaCha20Poly1305(key);
  const sealed = readFileSync(inPath);
  const opened = aead.open(new Uint8Array(nonce), new Uint8Array(sealed), aad ? new Uint8Array(aad) : undefined);
  if (!opened) throw new Error('decrypt failed');
  writeFileSync(outPath, opened);
  wipe(key);
}

export function encryptBufferToEnvelope(key: Uint8Array, data: Uint8Array): { sealed: Uint8Array; envelope: { alg: string; nonce_b64: string; aad_b64: string } } {
  const nonce = randomBytes(24);
  const aad = randomBytes(16);
  const aead = new XChaCha20Poly1305(key);
  const sealed = aead.seal(nonce, data, aad);
  const envelope = {
    alg: 'XChaCha20-Poly1305',
    nonce_b64: Buffer.from(nonce).toString('base64'),
    aad_b64: Buffer.from(aad).toString('base64'),
  };
  wipe(key);
  return { sealed: new Uint8Array(sealed), envelope };
}



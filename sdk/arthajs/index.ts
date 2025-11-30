import { ethers } from 'ethers';
import http from 'http';
import https from 'https';
import { URL, pathToFileURL } from 'url';
import { readFileSync, writeFileSync } from 'fs';

type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

function requestRaw(method: HttpMethod, urlStr: string, headers: Record<string, string>, body?: Buffer): Promise<{ status: number, headers: Record<string, string | string[]>, body: Buffer }> {
  return new Promise((resolve, reject) => {
    const url = new URL(urlStr);
    const isHttps = url.protocol === 'https:';
    const opts: any = {
      method,
      hostname: url.hostname,
      port: url.port || (isHttps ? 443 : 80),
      path: url.pathname + (url.search || ''),
      headers,
    };
    const client = isHttps ? https : http;
    const req = client.request(opts, (res) => {
      const chunks: Buffer[] = [];
      res.on('data', (d) => chunks.push(Buffer.isBuffer(d) ? d : Buffer.from(d)));
      res.on('end', () => {
        const buf = Buffer.concat(chunks);
        const resHeaders: Record<string, string | string[]> = {};
        for (const [k, v] of Object.entries(res.headers)) { if (v !== undefined) resHeaders[k] = v as any; }
        resolve({ status: res.statusCode || 0, headers: resHeaders, body: buf });
      });
    });
    req.on('error', reject);
    if (body && body.length) req.write(body);
    req.end();
  });
}

function buildMultipart(fieldName: string, filename: string, data: Buffer): { body: Buffer, contentType: string } {
  const boundary = '----arthajs-' + Math.random().toString(16).slice(2);
  const pre = `--${boundary}\r\n` +
    `Content-Disposition: form-data; name="${fieldName}"; filename="${filename}"\r\n` +
    `Content-Type: application/octet-stream\r\n\r\n`;
  const post = `\r\n--${boundary}--\r\n`;
  const body = Buffer.concat([Buffer.from(pre, 'utf8'), data, Buffer.from(post, 'utf8')]);
  return { body, contentType: `multipart/form-data; boundary=${boundary}` };
}

/**
 * Transaction signer helper for interacting with Ethereum-compatible chains
 */
export class TransactionSigner {
  private wallet: ethers.Wallet;
  private provider: ethers.Provider;

  constructor(privateKey: string, rpcUrl: string) {
    this.provider = new ethers.JsonRpcProvider(rpcUrl);
    this.wallet = new ethers.Wallet(privateKey, this.provider);
  }

  async signAndSend(tx: ethers.TransactionRequest): Promise<ethers.TransactionResponse> {
    return await this.wallet.sendTransaction(tx);
  }

  async call(tx: ethers.TransactionRequest): Promise<string> {
    return await this.provider.call(tx);
  }

  getAddress(): string {
    return this.wallet.address;
  }

  async getNonce(): Promise<number> {
    return await this.provider.getTransactionCount(this.wallet.address, 'pending');
  }

  async estimateGas(tx: ethers.TransactionRequest): Promise<bigint> {
    return await this.provider.estimateGas(tx);
  }
}

export class ArthaJS {
  private baseUrl: string;
  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  async quote(provider: string, cidUri: string): Promise<any> {
    const payload = JSON.stringify({ provider, cid: cidUri });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/retrieval/quote`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`quote failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Settle a retrieval payment on-chain using local signing
   */
  async settle(params: {
    signer: TransactionSigner;
    dealMarket: string;
    manifestRoot: string;
    bytesServed: number;
    provider: string;
    totalWei: bigint;
    gasPrice?: bigint;
    gasLimit?: bigint;
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function settle(bytes32 manifestRoot, uint256 bytesServed, address provider, uint256 totalWei)'
    ]);
    const data = iface.encodeFunctionData('settle', [
      params.manifestRoot,
      params.bytesServed,
      params.provider,
      params.totalWei
    ]);

    const tx: ethers.TransactionRequest = {
      to: params.dealMarket,
      data,
      value: 0n,
      gasPrice: params.gasPrice,
      gasLimit: params.gasLimit
    };

    return await params.signer.signAndSend(tx);
  }

  /**
   * Aggregate settlement with merkle root
   */
  async settleAggregate(params: {
    signer: TransactionSigner;
    dealMarket: string;
    manifestRoot: string;
    merkleRoot: string;
    provider: string;
    totalWei: bigint;
    gasPrice?: bigint;
    gasLimit?: bigint;
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function settleAggregate(bytes32 manifestRoot, bytes32 merkleRoot, address provider, uint256 totalWei)'
    ]);
    const data = iface.encodeFunctionData('settleAggregate', [
      params.manifestRoot,
      params.merkleRoot,
      params.provider,
      params.totalWei
    ]);

    const tx: ethers.TransactionRequest = {
      to: params.dealMarket,
      data,
      value: 0n,
      gasPrice: params.gasPrice,
      gasLimit: params.gasLimit
    };

    return await params.signer.signAndSend(tx);
  }

  /**
   * Aggregate settlement with per-leaf proof
   */
  async settleAggregateWithProof(params: {
    signer: TransactionSigner;
    dealMarket: string;
    manifestRoot: string;
    merkleRoot: string;
    leaf: string;
    branch: string[];
    index: number;
    provider: string;
    totalWei: bigint;
    gasPrice?: bigint;
    gasLimit?: bigint;
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function settleAggregateWithProof(bytes32 manifestRoot, bytes32 merkleRoot, bytes32 leaf, bytes32[] branch, uint256 index, address provider, uint256 totalWei)'
    ]);
    const data = iface.encodeFunctionData('settleAggregateWithProof', [
      params.manifestRoot,
      params.merkleRoot,
      params.leaf,
      params.branch,
      params.index,
      params.provider,
      params.totalWei
    ]);

    const tx: ethers.TransactionRequest = {
      to: params.dealMarket,
      data,
      value: 0n,
      gasPrice: params.gasPrice,
      gasLimit: params.gasLimit
    };

    return await params.signer.signAndSend(tx);
  }

  async uploadFile(filePath: string): Promise<string> {
    const data = readFileSync(filePath);
    const { body, contentType } = buildMultipart('file', filePath.split('/').pop() || 'data.bin', data);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, { 'Content-Type': contentType }, body);
    if (resp.status < 200 || resp.status >= 300) throw new Error(`upload failed ${resp.status} ${resp.body.toString('utf8')}`);
    const json = JSON.parse(resp.body.toString('utf8'));
    if (!json || !json.cid) throw new Error('Malformed response');
    return json.cid as string;
  }

  async uploadFileWithEnvelope(filePath: string, envelope: { alg: string; salt_b64?: string; nonce_b64?: string; aad_b64?: string }): Promise<string> {
    const data = readFileSync(filePath);
    const { body, contentType } = buildMultipart('file', filePath.split('/').pop() || 'data.bin', data);
    const headers: Record<string, string> = { 'Content-Type': contentType, 'X-Artha-Envelope': JSON.stringify(envelope) };
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, headers, body);
    if (resp.status < 200 || resp.status >= 300) throw new Error(`upload failed ${resp.status} ${resp.body.toString('utf8')}`);
    const json = JSON.parse(resp.body.toString('utf8'));
    if (!json || !json.cid) throw new Error('Malformed response');
    return json.cid as string;
  }

  async downloadToFile(cidUri: string, outPath: string, range?: { start?: number, end?: number }): Promise<{ status: number, bytes: number }> {
    const cid = cidUri.replace(/^artha:\/\//, '');
    const headers: Record<string, string> = {};
    if (range) {
      if (range.start !== undefined && range.end !== undefined) headers['Range'] = `bytes=${range.start}-${range.end}`;
      else if (range.start !== undefined) headers['Range'] = `bytes=${range.start}-`;
      else if (range.end !== undefined) headers['Range'] = `bytes=0-${range.end}`;
    }
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/download/${cid}`, headers);
    if (!(resp.status === 200 || resp.status === 206)) throw new Error(`download failed ${resp.status} ${resp.body.toString('utf8')}`);
    writeFileSync(outPath, resp.body);
    return { status: resp.status, bytes: resp.body.length };
  }

  async info(cidUri: string): Promise<any> {
    const cid = cidUri.replace(/^artha:\/\//, '');
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/info/${cid}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`info failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async createDeal(params: { cid: string; size: number; replicas: number; months: number; maxPrice: number; signer?: TransactionSigner; dealMarket?: string; gasPrice?: bigint; gasLimit?: bigint; }): Promise<any> {
    const payload = JSON.stringify({
      cid: params.cid,
      size: params.size,
      replicas: params.replicas,
      months: params.months,
      maxPrice: params.maxPrice
    });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/deals`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`deals failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async setAccessPolicy(params: { cidUri: string; private: boolean; allowedDids?: string[]; token?: string }): Promise<any> {
    const payload = JSON.stringify({ cid: params.cidUri, private: params.private, allowedDids: params.allowedDids || [], token: params.token });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/policy`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`access policy failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async allowlistAdd(cidUri: string, did: string): Promise<any> {
    const payload = JSON.stringify({ cid: cidUri, did });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/allowlist/add`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`allowlist add failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async allowlistRemove(cidUri: string, did: string): Promise<any> {
    const payload = JSON.stringify({ cid: cidUri, did });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/allowlist/remove`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`allowlist remove failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async buildMerkleBranch(cid: string, index: number): Promise<{ root: string, leaf: string, branch: string[], index: number }> {
    const payload = JSON.stringify({ cid, index });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/branch`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`branch failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Submit proof payout using local signing
   */
  async submitPayout(params: {
    signer: TransactionSigner;
    dealMarket: string;
    root: string;
    leaf: string;
    index: number;
    branch: string[];
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function streamPayoutV2(bytes32 root, bytes32 salt, bytes32 leaf, bytes32[] branch, uint256 index)'
    ]);
    const salt = ethers.ZeroHash; // Or derive from block hash
    const data = iface.encodeFunctionData('streamPayoutV2', [
      params.root,
      salt,
      params.leaf,
      params.branch,
      params.index
    ]);

    const tx: ethers.TransactionRequest = {
      to: params.dealMarket,
      data,
      value: 0n
    };

    return await params.signer.signAndSend(tx);
  }

  async getActiveProviders(rpcUrl: string, contract: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/providers?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get providers failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getProviderOffer(provider: string, rpcUrl: string, contract: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/offer/${provider}?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get offer failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getProviderReputation(provider: string, rpcUrl: string, contract: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/reputation/${provider}?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get reputation failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Report latency using local signing
   */
  async reportLatency(params: {
    signer: TransactionSigner;
    contract: string;
    provider: string;
    root: string;
    latencyMs: number;
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function reportLatency(address provider, bytes32 root, uint256 latencyMs)'
    ]);
    const data = iface.encodeFunctionData('reportLatency', [
      params.provider,
      params.root,
      params.latencyMs
    ]);

    const tx: ethers.TransactionRequest = {
      to: params.contract,
      data,
      value: 0n
    };

    return await params.signer.signAndSend(tx);
  }

  async porepProveSeal(params: { root: string; randomness: string; provider: string; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/porep/prove_seal`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`prove seal failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Issue PoRep challenge using local signing
   */
  async porepChallenge(params: {
    signer: TransactionSigner;
    contract: string;
    commitment: string;
  }): Promise<ethers.TransactionResponse> {
    const iface = new ethers.Interface([
      'function issueChallenge(bytes32 commitment)'
    ]);
    const data = iface.encodeFunctionData('issueChallenge', [params.commitment]);

    const tx: ethers.TransactionRequest = {
      to: params.contract,
      data,
      value: 0n
    };

    return await params.signer.signAndSend(tx);
  }

  async aiTrain(params: { modelCid: string; datasetCid: string; epochs?: number; region?: string; zkEnabled?: boolean; gpuRequired?: boolean; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/ai/train`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`train failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async aiJobStatus(jobId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/ai/job/${jobId}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`job status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async aiDeploy(params: { modelCid: string; name?: string; region?: string; replicas?: number; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/ai/deploy`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`deploy failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async aiDeploymentStatus(deploymentId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/ai/deploy/${deploymentId}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`deployment status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async explorerProofs(cid: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/svdb/explorer/proofs/${encodeURIComponent(cid)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`explorer proofs failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async estimateCost(params: { size: number; replicas: number; months: number; rpcUrl?: string; priceOracle?: string; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/explorer/cost/estimate`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`cost estimate failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

// Simple CLI helper when run directly
try {
  const isMain = import.meta && process.argv[1] && (import.meta.url === pathToFileURL(process.argv[1]).href);
  if (isMain) {
    const [, , cmd, ...rest] = process.argv;
    const base = process.env.ARTHA_NODE || 'http://127.0.0.1:8080';
    const sdk = new ArthaJS(base);
    (async () => {
      if (cmd === 'upload' && rest[0]) {
        const cid = await sdk.uploadFile(rest[0]);
        console.log(cid);
      } else if (cmd === 'info' && rest[0]) {
        const info = await sdk.info(rest[0]);
        console.log(JSON.stringify(info));
      } else if (cmd === 'download' && rest[0] && rest[1]) {
        const out = await sdk.downloadToFile(rest[0], rest[1]);
        console.log(JSON.stringify(out));
      } else {
        console.error('Usage: ARTHA_NODE=... node index.ts upload <file> | info <artha://cid> | download <artha://cid> <out>');
        process.exit(2);
      }
    })().catch((e) => { console.error(e); process.exit(1); });
  }
} catch { }

export class ArthaID {
  private baseUrl: string;
  private signer?: TransactionSigner;

  constructor(baseUrl: string, signer?: TransactionSigner) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.signer = signer;
  }

  /**
   * Create DID on-chain using local signing
   */
  async createDID(authKey: string, encKey: string, metaCid: string, contract: string): Promise<{ did: string, txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function createDID(bytes32 authKey, bytes32 encKey, bytes32 metaCid) returns (bytes32)'
    ]);
    const data = iface.encodeFunctionData('createDID', [authKey, encKey, metaCid]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const did = `did:artha:${this.signer.getAddress()}`;

    return {
      did,
      txHash: receipt!.hash
    };
  }

  async getDID(did: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/did/${encodeURIComponent(did)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getDID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Rotate DID keys using local signing
   */
  async rotateKeys(contract: string, newAuthKey: string, newEncKey: string): Promise<{ txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function rotateKeys(bytes32 newAuthKey, bytes32 newEncKey)'
    ]);
    const data = iface.encodeFunctionData('rotateKeys', [newAuthKey, newEncKey]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    return { txHash: receipt!.hash };
  }

  /**
   * Revoke DID using local signing
   */
  async revokeDID(contract: string): Promise<{ txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function revokeDID()'
    ]);
    const data = iface.encodeFunctionData('revokeDID', []);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    return { txHash: receipt!.hash };
  }

  async verifySignature(did: string, messageHash: string, signature: string): Promise<{ valid: boolean }> {
    const payload = JSON.stringify({ did, messageHash, signature });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/verify`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifySignature failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaVC {
  private baseUrl: string;
  private signer?: TransactionSigner;

  constructor(baseUrl: string, signer?: TransactionSigner) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.signer = signer;
  }

  /**
   * Issue VC on-chain using local signing
   */
  async issueVC(contract: string, subjectDid: string, claimHash: string, docCid: string, expiresAt: number): Promise<{ vcHash: string, txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function issueVC(address subject, bytes32 claimHash, bytes32 docCid, uint256 expiresAt) returns (bytes32)'
    ]);

    const subjectAddress = subjectDid.replace('did:artha:', '');
    const data = iface.encodeFunctionData('issueVC', [subjectAddress, claimHash, docCid, expiresAt]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const vcHash = ethers.keccak256(ethers.toUtf8Bytes(`${subjectDid}-${claimHash}-${Date.now()}`));

    return {
      vcHash,
      txHash: receipt!.hash
    };
  }

  /**
   * Revoke VC using local signing
   */
  async revokeVC(contract: string, vcHash: string): Promise<{ txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function revokeVC(bytes32 vcHash)'
    ]);
    const data = iface.encodeFunctionData('revokeVC', [vcHash]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    return { txHash: receipt!.hash };
  }

  async verifyVC(vcHash: string): Promise<{ valid: boolean, vc: any }> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/${encodeURIComponent(vcHash)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifyVC failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getVCsBySubject(subjectDid: string): Promise<{ vcs: any[] }> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/subject/${encodeURIComponent(subjectDid)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getVCsBySubject failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async hasClaimType(subjectDid: string, claimType: string): Promise<{ has: boolean }> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/claim/${encodeURIComponent(subjectDid)}/${encodeURIComponent(claimType)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`hasClaimType failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaAIID {
  private baseUrl: string;
  private signer?: TransactionSigner;

  constructor(baseUrl: string, signer?: TransactionSigner) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.signer = signer;
  }

  /**
   * Create AI ID on-chain using local signing
   */
  async createAIID(contract: string, modelCid: string, datasetId: string, codeHash: string, version: string): Promise<{ aiid: string, txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function createAIID(bytes32 modelCid, bytes32 datasetId, bytes32 codeHash, string version) returns (bytes32)'
    ]);
    const data = iface.encodeFunctionData('createAIID', [modelCid, datasetId, codeHash, version]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const aiid = `aiid:artha:${ethers.keccak256(ethers.toUtf8Bytes(modelCid + datasetId))}`;

    return {
      aiid,
      txHash: receipt!.hash
    };
  }

  async getAIID(aiid: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/aiid/${encodeURIComponent(aiid)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getAIID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  /**
   * Rotate AI ID using local signing
   */
  async rotateAIID(contract: string, aiid: string, newModelCid: string, newVersion: string): Promise<{ newAiid: string, txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function rotateAIID(bytes32 aiid, bytes32 newModelCid, string newVersion) returns (bytes32)'
    ]);
    const aiidHash = ethers.keccak256(ethers.toUtf8Bytes(aiid));
    const data = iface.encodeFunctionData('rotateAIID', [aiidHash, newModelCid, newVersion]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const newAiid = `aiid:artha:${ethers.keccak256(ethers.toUtf8Bytes(newModelCid + newVersion))}`;

    return {
      newAiid,
      txHash: receipt!.hash
    };
  }

  async linkOwner(contract: string, aiid: string, ownerDid: string): Promise<{ txHash: string }> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function linkOwner(bytes32 aiid, address owner)'
    ]);
    const aiidHash = ethers.keccak256(ethers.toUtf8Bytes(aiid));
    const ownerAddress = ownerDid.replace('did:artha:', '');
    const data = iface.encodeFunctionData('linkOwner', [aiidHash, ownerAddress]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    return { txHash: receipt!.hash };
  }

  async getLineage(aiid: string): Promise<{ lineage: string[] }> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/aiid/${encodeURIComponent(aiid)}/lineage`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getLineage failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaPolicy {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  async checkAccess(cid: string, did: string, sessionToken: string): Promise<{ allowed: boolean, reason?: string }> {
    const payload = JSON.stringify({ cid, did, sessionToken });
    const resp = await requestRaw('POST', `${this.baseUrl}/policy/check`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`checkAccess failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async createSession(did: string, scope: string[]): Promise<{ sessionId: string, token: string }> {
    const payload = JSON.stringify({ did, scope });
    const resp = await requestRaw('POST', `${this.baseUrl}/policy/session/create`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`createSession failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async revokeSession(sessionId: string): Promise<{ success: boolean }> {
    const payload = JSON.stringify({ sessionId });
    const resp = await requestRaw('POST', `${this.baseUrl}/policy/session/revoke`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`revokeSession failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaAI {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  async scoreVCRisk(vcInput: any): Promise<{ risk: number, reasonCodes: string[], threshold: boolean }> {
    const payload = JSON.stringify(vcInput);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/risk/score`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`scoreVCRisk failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async detectAnomaly(nodeMetrics: any): Promise<{ anomalyScore: number, suggestedAction: string, anomalies: string[] }> {
    const payload = JSON.stringify(nodeMetrics);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/anomaly/detect`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`detectAnomaly failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async scoreReputation(reputationInput: any): Promise<{ arthaScore: number, flags: string[], riskLevel: string }> {
    const payload = JSON.stringify(reputationInput);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/reputation/score`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`scoreReputation failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async verifyAuthenticity(receipt: any): Promise<{ isAuthentic: boolean, confidence: number, provenance: string[] }> {
    const payload = JSON.stringify(receipt);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/authenticity/verify`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifyAuthenticity failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaDataset {
  constructor(private baseUrl: string, private signer?: TransactionSigner) { }

  /**
   * Register dataset on-chain using local signing
   */
  async register(contract: string, rootCid: string, licenseCid: string, tags: string[]): Promise<string> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function registerDataset(bytes32 rootCid, bytes32 licenseCid, string[] tags) returns (bytes32)'
    ]);
    const data = iface.encodeFunctionData('registerDataset', [rootCid, licenseCid, tags]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const datasetId = `dataset:${ethers.keccak256(ethers.toUtf8Bytes(rootCid))}`;
    return datasetId;
  }

  async list(owner?: string): Promise<any[]> {
    const url = owner ? `${this.baseUrl}/ai/dataset/list?owner=${encodeURIComponent(owner)}` : `${this.baseUrl}/ai/dataset/list`;
    const resp = await requestRaw('GET', url, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`list datasets failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getInfo(datasetId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/dataset/${encodeURIComponent(datasetId)}`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get dataset info failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaModel {
  constructor(private baseUrl: string, private signer?: TransactionSigner) { }

  /**
   * Register model on-chain using local signing
   */
  async register(contract: string, params: {
    modelCid: string,
    architecture: string,
    baseModelId?: string,
    datasetId: string,
    codeHash: string,
    version: string,
    licenseCid?: string
  }): Promise<string> {
    if (!this.signer) throw new Error('Signer required for on-chain operations');

    const iface = new ethers.Interface([
      'function registerModel(bytes32 modelCid, string architecture, bytes32 datasetId, bytes32 codeHash, string version) returns (bytes32)'
    ]);
    const data = iface.encodeFunctionData('registerModel', [
      params.modelCid,
      params.architecture,
      params.datasetId,
      params.codeHash,
      params.version
    ]);

    const tx = await this.signer.signAndSend({
      to: contract,
      data,
      value: 0n
    });

    const receipt = await tx.wait();
    const modelId = `model:${ethers.keccak256(ethers.toUtf8Bytes(params.modelCid))}`;
    return modelId;
  }

  async list(owner?: string): Promise<any[]> {
    const url = owner ? `${this.baseUrl}/ai/model/list?owner=${encodeURIComponent(owner)}` : `${this.baseUrl}/ai/model/list`;
    const resp = await requestRaw('GET', url, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`list models failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getLineage(modelId: string): Promise<string[]> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/lineage`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get model lineage failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async addCheckpoint(modelId: string, checkpointCid: string, metricsJsonCid: string, step: number): Promise<void> {
    const payload = JSON.stringify({ checkpointCid, metricsJsonCid, step });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/checkpoint`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`add checkpoint failed ${resp.status}`);
  }

  async publish(modelId: string, checkpointCid: string): Promise<void> {
    const payload = JSON.stringify({ checkpointCid });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/publish`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`publish model failed ${resp.status}`);
  }
}

export class ArthaJob {
  constructor(private baseUrl: string) { }

  async submitTrain(params: {
    modelId: string,
    datasetId: string,
    submitterDid: string,
    epochs: number,
    batchSize: number,
    learningRate: number,
    optimizer: string,
    budget: number
  }): Promise<{ jobId: string, status: string, estimatedCost: number, estimatedDurationSecs: number }> {
    const payload = JSON.stringify({
      modelId: params.modelId,
      datasetId: params.datasetId,
      submitterDid: params.submitterDid,
      params: {
        epochs: params.epochs,
        batchSize: params.batchSize,
        learningRate: params.learningRate,
        optimizer: params.optimizer,
        checkpointInterval: 500
      },
      budget: params.budget
    });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/train`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`submit train job failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async submitInfer(params: {
    modelId: string,
    inputCid?: string,
    inlineInput?: string,
    submitterDid: string,
    mode: string,
    maxTokens?: number,
    budget: number
  }): Promise<{ jobId: string, status: string }> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/infer`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`submit infer job failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async submitAgent(params: {
    agentSpecCid: string,
    submitterDid: string,
    goal: string,
    tools: string[],
    memoryPolicy: string,
    budget: number
  }): Promise<{ jobId: string, status: string }> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/agent`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`submit agent job failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getStatus(jobId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/status`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get job status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getLogs(jobId: string): Promise<string[]> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/logs`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get job logs failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async cancel(jobId: string): Promise<void> {
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/cancel`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`cancel job failed ${resp.status}`);
  }

  async getArtifacts(jobId: string): Promise<string[]> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/artifacts`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get job artifacts failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getToolCalls(jobId: string): Promise<any[]> {
    const resp = await requestRaw('GET', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/tool-calls`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get tool calls failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async recordToolCall(jobId: string, toolName: string, params: any, result: any): Promise<any> {
    const payload = JSON.stringify({ tool_name: toolName, params, result });
    const resp = await requestRaw('POST', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/tool-call`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`record tool call failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getAgentMemory(jobId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/memory`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get agent memory failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async updateAgentMemory(jobId: string, memoryCid: string): Promise<void> {
    const payload = JSON.stringify({ memory_cid: memoryCid });
    const resp = await requestRaw('POST', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/memory`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`update agent memory failed ${resp.status}`);
  }
}

export class ArthaFederated {
  constructor(private baseUrl: string) { }

  async startRound(params: {
    modelId: string,
    datasetIds: string[],
    rounds: number,
    dp: boolean,
    budget: number
  }): Promise<{ fedId: string, status: string }> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/start`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`start federated failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getRoundStatus(fedId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/status`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get federated status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async submitGradient(fedId: string, weights: number[], sampleCount: number): Promise<any> {
    const payload = JSON.stringify({ fed_id: fedId, weights, sample_count: sampleCount });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/gradient`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`submit gradient failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async triggerAggregation(fedId: string): Promise<any> {
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/aggregate`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`trigger aggregation failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaEvolution {
  constructor(private baseUrl: string) { }

  async start(params: {
    searchSpaceCid: string,
    population: number,
    generations: number,
    budget: number
  }): Promise<{ evoId: string, status: string }> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/evolve/start`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`start evolution failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getStatus(evoId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/evolve/${encodeURIComponent(evoId)}/status`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get evolution status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getPopulation(evoId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/evolve/${encodeURIComponent(evoId)}/population`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get evolution population failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaDeployment {
  constructor(private baseUrl: string) { }

  async deploy(params: {
    modelId: string,
    endpoint: string,
    replicas: number,
    maxTokens: number
  }): Promise<{ deploymentId: string, endpointUrl: string }> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/deploy`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`deploy model failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getStatus(deploymentId: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}/status`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`get deployment status failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async scale(deploymentId: string, replicas: number): Promise<void> {
    const payload = JSON.stringify({ replicas });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}/scale`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`scale deployment failed ${resp.status}`);
  }

  async undeploy(deploymentId: string): Promise<void> {
    const resp = await requestRaw('DELETE', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}`, {}, Buffer.alloc(0));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`undeploy failed ${resp.status}`);
  }
}

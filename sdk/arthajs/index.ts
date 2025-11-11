import http from 'http';
import https from 'https';
import { URL, pathToFileURL } from 'url';
import { readFileSync, writeFileSync } from 'fs';

type HttpMethod = 'GET' | 'POST';

function requestRaw(method: HttpMethod, urlStr: string, headers: Record<string,string>, body?: Buffer): Promise<{status:number, headers:Record<string,string|string[]>, body:Buffer}> {
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
        const resHeaders: Record<string,string|string[]> = {};
        for (const [k, v] of Object.entries(res.headers)) { if (v !== undefined) resHeaders[k] = v as any; }
        resolve({ status: res.statusCode || 0, headers: resHeaders, body: buf });
      });
    });
    req.on('error', reject);
    if (body && body.length) req.write(body);
    req.end();
  });
}

function buildMultipart(fieldName: string, filename: string, data: Buffer): {body: Buffer, contentType: string} {
  const boundary = '----arthajs-' + Math.random().toString(16).slice(2);
  const pre = `--${boundary}\r\n`+
              `Content-Disposition: form-data; name="${fieldName}"; filename="${filename}"\r\n`+
              `Content-Type: application/octet-stream\r\n\r\n`;
  const post = `\r\n--${boundary}--\r\n`;
  const body = Buffer.concat([Buffer.from(pre, 'utf8'), data, Buffer.from(post, 'utf8')]);
  return { body, contentType: `multipart/form-data; boundary=${boundary}` };
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

  async settle(params: { rpcUrl: string; chainId: number; privateKey: string; dealMarket: string; manifestRoot: string; bytesServed: number; provider: string; totalWei: number; gasPrice?: number; gasLimit?: number; nonce?: number; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/retrieval/settle`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`settle failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Aggregate settlement (merkle root, no per-leaf proof)
  async settleAggregate(params: { rpcUrl: string; chainId: number; privateKey: string; dealMarket: string; manifestRoot: string; merkleRoot: string; provider: string; totalWei: number; gasPrice?: number; gasLimit?: number; nonce?: number; }): Promise<any> {
    const payload = JSON.stringify({
      rpcUrl: params.rpcUrl,
      chainId: params.chainId,
      privateKey: params.privateKey,
      dealMarket: params.dealMarket,
      manifestRoot: params.manifestRoot,
      merkleRoot: params.merkleRoot,
      provider: params.provider,
      totalWei: params.totalWei,
      gasPrice: params.gasPrice,
      gasLimit: params.gasLimit,
      nonce: params.nonce,
    });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/retrieval/settle`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`aggregate settle failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Aggregate settlement with per-leaf proof under the aggregate merkle root
  async settleAggregateWithProof(params: { rpcUrl: string; chainId: number; privateKey: string; dealMarket: string; manifestRoot: string; merkleRoot: string; leaf: string; branch: string[]; index: number; provider: string; totalWei: number; gasPrice?: number; gasLimit?: number; nonce?: number; }): Promise<any> {
    const payload = JSON.stringify({
      rpcUrl: params.rpcUrl,
      chainId: params.chainId,
      privateKey: params.privateKey,
      dealMarket: params.dealMarket,
      manifestRoot: params.manifestRoot,
      merkleRoot: params.merkleRoot,
      leaf: params.leaf,
      branch: params.branch,
      index: params.index,
      provider: params.provider,
      totalWei: params.totalWei,
      gasPrice: params.gasPrice,
      gasLimit: params.gasLimit,
      nonce: params.nonce,
    });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/retrieval/settle`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`aggregate settle proof failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
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
    const headers: Record<string,string> = { 'Content-Type': contentType, 'X-Artha-Envelope': JSON.stringify(envelope) };
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, headers, body);
    if (resp.status < 200 || resp.status >= 300) throw new Error(`upload failed ${resp.status} ${resp.body.toString('utf8')}`);
    const json = JSON.parse(resp.body.toString('utf8'));
    if (!json || !json.cid) throw new Error('Malformed response');
    return json.cid as string;
    }

  async downloadToFile(cidUri: string, outPath: string, range?: {start?: number, end?: number}): Promise<{status:number, bytes:number}> {
    const cid = cidUri.replace(/^artha:\/\//, '');
    const headers: Record<string,string> = {};
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

  async createDeal(params: { cid: string; size: number; replicas: number; months: number; maxPrice: number; rpcUrl?: string; chainId?: number; privateKey?: string; dealMarket?: string; gasPrice?: number; gasLimit?: number; }): Promise<any> {
    const payload = JSON.stringify(params);
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

  // Batch verify via eth_call to ProofsV2
  async proofsV2BatchVerify(rpcUrl: string, proofsV2: string, dataHex: string): Promise<any> {
    const payload = JSON.stringify({ rpcUrl, proofsV2, data: dataHex });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/v2/batch/verify`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`batch verify failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Batch submit to DealMarket.streamPayoutV2Batch
  async proofsV2BatchSubmit(params: { rpcUrl: string; chainId: number; privateKey: string; gasPrice?: number; gasLimit?: number; dealMarket: string; dataHex: string; nonce?: number; }): Promise<any> {
    const payload = JSON.stringify({ rpcUrl: params.rpcUrl, chainId: params.chainId, privateKey: params.privateKey, gasPrice: params.gasPrice, gasLimit: params.gasLimit, dealMarket: params.dealMarket, data: params.dataHex, nonce: params.nonce });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/v2/batch/submit`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`batch submit failed ${resp.status} ${resp.body.toString('utf8')}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async buildMerkleBranch(cid: string, index: number): Promise<{root:string, leaf:string, branch:string[], index:number}> {
    const payload = JSON.stringify({ cid, index });
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/branch`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`branch failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async submitPayout(params: { rpcUrl: string; chainId: number; privateKey: string; nonce: number; gasPrice: number; gasLimit: number; dealMarket: string; root: string; leaf: string; index: number; branch: string[]; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/submit`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`submit failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Phase 4: Marketplace & SLA
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

  async reportLatency(params: { client: string; provider: string; root: string; latencyMs: number; rpcUrl: string; contract: string; privateKey: string; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/sla/report_latency`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`report latency failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Phase 4: PoRep GPU proving
  async porepProveSeal(params: { root: string; randomness: string; provider: string; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/porep/prove_seal`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`prove seal failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async porepChallenge(params: { commitment: string; rpcUrl: string; contract: string; privateKey: string; }): Promise<any> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/porep/challenge`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`issue challenge failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  // Phase 4: One-click AI
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

  // Phase 4: Analytics
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

// Simple CLI helper when run directly: node index.js upload <file>
// ESM-compatible "run if main" check
try {
  const isMain = import.meta && process.argv[1] && (import.meta.url === pathToFileURL(process.argv[1]).href);
  if (isMain) {
    const [,, cmd, ...rest] = process.argv;
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
} catch {}

// ============================================================================
// Identity & AI Extensions
// ============================================================================

export class ArthaID {
  private baseUrl: string;
  private rpcUrl: string;
  
  constructor(baseUrl: string, rpcUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.rpcUrl = rpcUrl;
  }

  async createDID(authKey: string, encKey: string, metaCid: string): Promise<{did: string, txHash: string}> {
    const payload = JSON.stringify({ authKey, encKey, metaCid });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/create`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`createDID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getDID(did: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/did/${encodeURIComponent(did)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getDID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async rotateKeys(did: string, newAuthKey: string, newEncKey: string): Promise<{txHash: string}> {
    const payload = JSON.stringify({ did, newAuthKey, newEncKey });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/rotate`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`rotateKeys failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async revokeDID(did: string): Promise<{txHash: string}> {
    const payload = JSON.stringify({ did });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/revoke`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`revokeDID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async verifySignature(did: string, messageHash: string, signature: string): Promise<{valid: boolean}> {
    const payload = JSON.stringify({ did, messageHash, signature });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/verify`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifySignature failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaVC {
  private baseUrl: string;
  
  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  async issueVC(issuerDid: string, subjectDid: string, claimHash: string, docCid: string, expiresAt: number): Promise<{vcHash: string, txHash: string}> {
    const payload = JSON.stringify({ issuerDid, subjectDid, claimHash, docCid, expiresAt });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/vc/issue`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`issueVC failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async revokeVC(vcHash: string): Promise<{txHash: string}> {
    const payload = JSON.stringify({ vcHash });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/vc/revoke`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`revokeVC failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async verifyVC(vcHash: string): Promise<{valid: boolean, vc: any}> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/${encodeURIComponent(vcHash)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifyVC failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getVCsBySubject(subjectDid: string): Promise<{vcs: any[]}> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/subject/${encodeURIComponent(subjectDid)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getVCsBySubject failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async hasClaimType(subjectDid: string, claimType: string): Promise<{has: boolean}> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/claim/${encodeURIComponent(subjectDid)}/${encodeURIComponent(claimType)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`hasClaimType failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

export class ArthaAIID {
  private baseUrl: string;
  
  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  async createAIID(ownerDid: string, modelCid: string, datasetId: string, codeHash: string, version: string): Promise<{aiid: string, txHash: string}> {
    const payload = JSON.stringify({ ownerDid, modelCid, datasetId, codeHash, version });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/aiid/create`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`createAIID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getAIID(aiid: string): Promise<any> {
    const resp = await requestRaw('GET', `${this.baseUrl}/identity/aiid/${encodeURIComponent(aiid)}`, {});
    if (resp.status < 200 || resp.status >= 300) throw new Error(`getAIID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async rotateAIID(aiid: string, newModelCid: string, newVersion: string): Promise<{newAiid: string, txHash: string}> {
    const payload = JSON.stringify({ aiid, newModelCid, newVersion });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/aiid/rotate`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`rotateAIID failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async linkOwner(aiid: string, ownerDid: string): Promise<{txHash: string}> {
    const payload = JSON.stringify({ aiid, ownerDid });
    const resp = await requestRaw('POST', `${this.baseUrl}/identity/aiid/link`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`linkOwner failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async getLineage(aiid: string): Promise<{lineage: string[]}> {
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

  async checkAccess(cid: string, did: string, sessionToken: string): Promise<{allowed: boolean, reason?: string}> {
    const payload = JSON.stringify({ cid, did, sessionToken });
    const resp = await requestRaw('POST', `${this.baseUrl}/policy/check`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`checkAccess failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async createSession(did: string, scope: string[]): Promise<{sessionId: string, token: string}> {
    const payload = JSON.stringify({ did, scope });
    const resp = await requestRaw('POST', `${this.baseUrl}/policy/session/create`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`createSession failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async revokeSession(sessionId: string): Promise<{success: boolean}> {
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

  async scoreVCRisk(vcInput: any): Promise<{risk: number, reasonCodes: string[], threshold: boolean}> {
    const payload = JSON.stringify(vcInput);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/risk/score`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`scoreVCRisk failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async detectAnomaly(nodeMetrics: any): Promise<{anomalyScore: number, suggestedAction: string, anomalies: string[]}> {
    const payload = JSON.stringify(nodeMetrics);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/anomaly/detect`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`detectAnomaly failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async scoreReputation(reputationInput: any): Promise<{arthaScore: number, flags: string[], riskLevel: string}> {
    const payload = JSON.stringify(reputationInput);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/reputation/score`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`scoreReputation failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }

  async verifyAuthenticity(receipt: any): Promise<{isAuthentic: boolean, confidence: number, provenance: string[]}> {
    const payload = JSON.stringify(receipt);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/authenticity/verify`, 
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`verifyAuthenticity failed ${resp.status}`);
    return JSON.parse(resp.body.toString('utf8'));
  }
}

// ============================================================================
// ArthaAIN v1 - AI Cloud Platform Classes
// ============================================================================

export class ArthaDataset {
  constructor(private baseUrl: string) {}

  async register(rootCid: string, licenseCid: string, tags: string[]): Promise<string> {
    const payload = JSON.stringify({ rootCid, licenseCid, tags });
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/dataset/register`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`register dataset failed ${resp.status}`);
    const result = JSON.parse(resp.body.toString('utf8'));
    return result.datasetId;
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
  constructor(private baseUrl: string) {}

  async register(params: {
    modelCid: string,
    architecture: string,
    baseModelId?: string,
    datasetId: string,
    codeHash: string,
    version: string,
    licenseCid?: string
  }): Promise<string> {
    const payload = JSON.stringify(params);
    const resp = await requestRaw('POST', `${this.baseUrl}/ai/model/register`,
      { 'Content-Type': 'application/json' }, Buffer.from(payload));
    if (resp.status < 200 || resp.status >= 300) throw new Error(`register model failed ${resp.status}`);
    const result = JSON.parse(resp.body.toString('utf8'));
    return result.modelId;
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
  constructor(private baseUrl: string) {}

  async submitTrain(params: {
    modelId: string,
    datasetId: string,
    submitterDid: string,
    epochs: number,
    batchSize: number,
    learningRate: number,
    optimizer: string,
    budget: number
  }): Promise<{jobId: string, status: string, estimatedCost: number, estimatedDurationSecs: number}> {
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
  }): Promise<{jobId: string, status: string}> {
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
  }): Promise<{jobId: string, status: string}> {
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
  constructor(private baseUrl: string) {}

  async startRound(params: {
    modelId: string,
    datasetIds: string[],
    rounds: number,
    dp: boolean,
    budget: number
  }): Promise<{fedId: string, status: string}> {
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
  constructor(private baseUrl: string) {}

  async start(params: {
    searchSpaceCid: string,
    population: number,
    generations: number,
    budget: number
  }): Promise<{evoId: string, status: string}> {
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
  constructor(private baseUrl: string) {}

  async deploy(params: {
    modelId: string,
    endpoint: string,
    replicas: number,
    maxTokens: number
  }): Promise<{deploymentId: string, endpointUrl: string}> {
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



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
  async uploadFile(filePath: string): Promise<string> {
    const data = readFileSync(filePath);
    const { body, contentType } = buildMultipart('file', filePath.split('/').pop() || 'data.bin', data);
    const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, { 'Content-Type': contentType }, body);
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



import { ethers } from 'ethers';
import http from 'http';
import https from 'https';
import { URL, pathToFileURL } from 'url';
import { readFileSync, writeFileSync } from 'fs';
function requestRaw(method, urlStr, headers, body) {
    return new Promise((resolve, reject) => {
        const url = new URL(urlStr);
        const isHttps = url.protocol === 'https:';
        const opts = {
            method,
            hostname: url.hostname,
            port: url.port || (isHttps ? 443 : 80),
            path: url.pathname + (url.search || ''),
            headers,
        };
        const client = isHttps ? https : http;
        const req = client.request(opts, (res) => {
            const chunks = [];
            res.on('data', (d) => chunks.push(Buffer.isBuffer(d) ? d : Buffer.from(d)));
            res.on('end', () => {
                const buf = Buffer.concat(chunks);
                const resHeaders = {};
                for (const [k, v] of Object.entries(res.headers)) {
                    if (v !== undefined)
                        resHeaders[k] = v;
                }
                resolve({ status: res.statusCode || 0, headers: resHeaders, body: buf });
            });
        });
        req.on('error', reject);
        if (body && body.length)
            req.write(body);
        req.end();
    });
}
function buildMultipart(fieldName, filename, data) {
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
    constructor(privateKey, rpcUrl) {
        this.provider = new ethers.JsonRpcProvider(rpcUrl);
        this.wallet = new ethers.Wallet(privateKey, this.provider);
    }
    async signAndSend(tx) {
        return await this.wallet.sendTransaction(tx);
    }
    async call(tx) {
        return await this.provider.call(tx);
    }
    getAddress() {
        return this.wallet.address;
    }
    async getNonce() {
        return await this.provider.getTransactionCount(this.wallet.address, 'pending');
    }
    async estimateGas(tx) {
        return await this.provider.estimateGas(tx);
    }
}
export class ArthaJS {
    constructor(baseUrl) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }
    async quote(provider, cidUri) {
        const payload = JSON.stringify({ provider, cid: cidUri });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/retrieval/quote`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`quote failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Settle a retrieval payment on-chain using local signing
     */
    async settle(params) {
        const iface = new ethers.Interface([
            'function settle(bytes32 manifestRoot, uint256 bytesServed, address provider, uint256 totalWei)'
        ]);
        const data = iface.encodeFunctionData('settle', [
            params.manifestRoot,
            params.bytesServed,
            params.provider,
            params.totalWei
        ]);
        const tx = {
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
    async settleAggregate(params) {
        const iface = new ethers.Interface([
            'function settleAggregate(bytes32 manifestRoot, bytes32 merkleRoot, address provider, uint256 totalWei)'
        ]);
        const data = iface.encodeFunctionData('settleAggregate', [
            params.manifestRoot,
            params.merkleRoot,
            params.provider,
            params.totalWei
        ]);
        const tx = {
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
    async settleAggregateWithProof(params) {
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
        const tx = {
            to: params.dealMarket,
            data,
            value: 0n,
            gasPrice: params.gasPrice,
            gasLimit: params.gasLimit
        };
        return await params.signer.signAndSend(tx);
    }
    async uploadFile(filePath) {
        const data = readFileSync(filePath);
        const { body, contentType } = buildMultipart('file', filePath.split('/').pop() || 'data.bin', data);
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, { 'Content-Type': contentType }, body);
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`upload failed ${resp.status} ${resp.body.toString('utf8')}`);
        const json = JSON.parse(resp.body.toString('utf8'));
        if (!json || !json.cid)
            throw new Error('Malformed response');
        return json.cid;
    }
    async uploadFileWithEnvelope(filePath, envelope) {
        const data = readFileSync(filePath);
        const { body, contentType } = buildMultipart('file', filePath.split('/').pop() || 'data.bin', data);
        const headers = { 'Content-Type': contentType, 'X-Artha-Envelope': JSON.stringify(envelope) };
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/upload`, headers, body);
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`upload failed ${resp.status} ${resp.body.toString('utf8')}`);
        const json = JSON.parse(resp.body.toString('utf8'));
        if (!json || !json.cid)
            throw new Error('Malformed response');
        return json.cid;
    }
    async downloadToFile(cidUri, outPath, range) {
        const cid = cidUri.replace(/^artha:\/\//, '');
        const headers = {};
        if (range) {
            if (range.start !== undefined && range.end !== undefined)
                headers['Range'] = `bytes=${range.start}-${range.end}`;
            else if (range.start !== undefined)
                headers['Range'] = `bytes=${range.start}-`;
            else if (range.end !== undefined)
                headers['Range'] = `bytes=0-${range.end}`;
        }
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/download/${cid}`, headers);
        if (!(resp.status === 200 || resp.status === 206))
            throw new Error(`download failed ${resp.status} ${resp.body.toString('utf8')}`);
        writeFileSync(outPath, resp.body);
        return { status: resp.status, bytes: resp.body.length };
    }
    async info(cidUri) {
        const cid = cidUri.replace(/^artha:\/\//, '');
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/info/${cid}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`info failed ${resp.status} ${resp.body.toString('utf8')}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async createDeal(params) {
        const payload = JSON.stringify({
            cid: params.cid,
            size: params.size,
            replicas: params.replicas,
            months: params.months,
            maxPrice: params.maxPrice
        });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/deals`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`deals failed ${resp.status} ${resp.body.toString('utf8')}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async setAccessPolicy(params) {
        const payload = JSON.stringify({ cid: params.cidUri, private: params.private, allowedDids: params.allowedDids || [], token: params.token });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/policy`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`access policy failed ${resp.status} ${resp.body.toString('utf8')}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async allowlistAdd(cidUri, did) {
        const payload = JSON.stringify({ cid: cidUri, did });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/allowlist/add`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`allowlist add failed ${resp.status} ${resp.body.toString('utf8')}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async allowlistRemove(cidUri, did) {
        const payload = JSON.stringify({ cid: cidUri, did });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/access/allowlist/remove`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`allowlist remove failed ${resp.status} ${resp.body.toString('utf8')}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async buildMerkleBranch(cid, index) {
        const payload = JSON.stringify({ cid, index });
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/proofs/branch`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`branch failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Submit proof payout using local signing
     */
    async submitPayout(params) {
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
        const tx = {
            to: params.dealMarket,
            data,
            value: 0n
        };
        return await params.signer.signAndSend(tx);
    }
    async getActiveProviders(rpcUrl, contract) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/providers?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get providers failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getProviderOffer(provider, rpcUrl, contract) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/offer/${provider}?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get offer failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getProviderReputation(provider, rpcUrl, contract) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/marketplace/reputation/${provider}?rpcUrl=${encodeURIComponent(rpcUrl)}&contract=${encodeURIComponent(contract)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get reputation failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Report latency using local signing
     */
    async reportLatency(params) {
        const iface = new ethers.Interface([
            'function reportLatency(address provider, bytes32 root, uint256 latencyMs)'
        ]);
        const data = iface.encodeFunctionData('reportLatency', [
            params.provider,
            params.root,
            params.latencyMs
        ]);
        const tx = {
            to: params.contract,
            data,
            value: 0n
        };
        return await params.signer.signAndSend(tx);
    }
    async porepProveSeal(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/porep/prove_seal`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`prove seal failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Issue PoRep challenge using local signing
     */
    async porepChallenge(params) {
        const iface = new ethers.Interface([
            'function issueChallenge(bytes32 commitment)'
        ]);
        const data = iface.encodeFunctionData('issueChallenge', [params.commitment]);
        const tx = {
            to: params.contract,
            data,
            value: 0n
        };
        return await params.signer.signAndSend(tx);
    }
    async aiTrain(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/ai/train`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`train failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async aiJobStatus(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/ai/job/${jobId}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`job status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async aiDeploy(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/ai/deploy`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`deploy failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async aiDeploymentStatus(deploymentId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/ai/deploy/${deploymentId}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`deployment status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async explorerProofs(cid) {
        const resp = await requestRaw('GET', `${this.baseUrl}/svdb/explorer/proofs/${encodeURIComponent(cid)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`explorer proofs failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async estimateCost(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/svdb/explorer/cost/estimate`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`cost estimate failed ${resp.status}`);
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
            }
            else if (cmd === 'info' && rest[0]) {
                const info = await sdk.info(rest[0]);
                console.log(JSON.stringify(info));
            }
            else if (cmd === 'download' && rest[0] && rest[1]) {
                const out = await sdk.downloadToFile(rest[0], rest[1]);
                console.log(JSON.stringify(out));
            }
            else {
                console.error('Usage: ARTHA_NODE=... node index.ts upload <file> | info <artha://cid> | download <artha://cid> <out>');
                process.exit(2);
            }
        })().catch((e) => { console.error(e); process.exit(1); });
    }
}
catch { }
export class ArthaID {
    constructor(baseUrl, signer) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.signer = signer;
    }
    /**
     * Create DID on-chain using local signing
     */
    async createDID(authKey, encKey, metaCid, contract) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
            txHash: receipt.hash
        };
    }
    async getDID(did) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/did/${encodeURIComponent(did)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`getDID failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Rotate DID keys using local signing
     */
    async rotateKeys(contract, newAuthKey, newEncKey) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
        return { txHash: receipt.hash };
    }
    /**
     * Revoke DID using local signing
     */
    async revokeDID(contract) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
        return { txHash: receipt.hash };
    }
    async verifySignature(did, messageHash, signature) {
        const payload = JSON.stringify({ did, messageHash, signature });
        const resp = await requestRaw('POST', `${this.baseUrl}/identity/did/verify`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`verifySignature failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaVC {
    constructor(baseUrl, signer) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.signer = signer;
    }
    /**
     * Issue VC on-chain using local signing
     */
    async issueVC(contract, subjectDid, claimHash, docCid, expiresAt) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
            txHash: receipt.hash
        };
    }
    /**
     * Revoke VC using local signing
     */
    async revokeVC(contract, vcHash) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
        return { txHash: receipt.hash };
    }
    async verifyVC(vcHash) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/${encodeURIComponent(vcHash)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`verifyVC failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getVCsBySubject(subjectDid) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/subject/${encodeURIComponent(subjectDid)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`getVCsBySubject failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async hasClaimType(subjectDid, claimType) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/vc/claim/${encodeURIComponent(subjectDid)}/${encodeURIComponent(claimType)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`hasClaimType failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaAIID {
    constructor(baseUrl, signer) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.signer = signer;
    }
    /**
     * Create AI ID on-chain using local signing
     */
    async createAIID(contract, modelCid, datasetId, codeHash, version) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
            txHash: receipt.hash
        };
    }
    async getAIID(aiid) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/aiid/${encodeURIComponent(aiid)}`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`getAIID failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    /**
     * Rotate AI ID using local signing
     */
    async rotateAIID(contract, aiid, newModelCid, newVersion) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
            txHash: receipt.hash
        };
    }
    async linkOwner(contract, aiid, ownerDid) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
        return { txHash: receipt.hash };
    }
    async getLineage(aiid) {
        const resp = await requestRaw('GET', `${this.baseUrl}/identity/aiid/${encodeURIComponent(aiid)}/lineage`, {});
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`getLineage failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaPolicy {
    constructor(baseUrl) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }
    async checkAccess(cid, did, sessionToken) {
        const payload = JSON.stringify({ cid, did, sessionToken });
        const resp = await requestRaw('POST', `${this.baseUrl}/policy/check`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`checkAccess failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async createSession(did, scope) {
        const payload = JSON.stringify({ did, scope });
        const resp = await requestRaw('POST', `${this.baseUrl}/policy/session/create`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`createSession failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async revokeSession(sessionId) {
        const payload = JSON.stringify({ sessionId });
        const resp = await requestRaw('POST', `${this.baseUrl}/policy/session/revoke`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`revokeSession failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaAI {
    constructor(baseUrl) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }
    async scoreVCRisk(vcInput) {
        const payload = JSON.stringify(vcInput);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/risk/score`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`scoreVCRisk failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async detectAnomaly(nodeMetrics) {
        const payload = JSON.stringify(nodeMetrics);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/anomaly/detect`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`detectAnomaly failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async scoreReputation(reputationInput) {
        const payload = JSON.stringify(reputationInput);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/reputation/score`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`scoreReputation failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async verifyAuthenticity(receipt) {
        const payload = JSON.stringify(receipt);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/authenticity/verify`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`verifyAuthenticity failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaDataset {
    constructor(baseUrl, signer) {
        this.baseUrl = baseUrl;
        this.signer = signer;
    }
    /**
     * Register dataset on-chain using local signing
     */
    async register(contract, rootCid, licenseCid, tags) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
    async list(owner) {
        const url = owner ? `${this.baseUrl}/ai/dataset/list?owner=${encodeURIComponent(owner)}` : `${this.baseUrl}/ai/dataset/list`;
        const resp = await requestRaw('GET', url, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`list datasets failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getInfo(datasetId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/dataset/${encodeURIComponent(datasetId)}`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get dataset info failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaModel {
    constructor(baseUrl, signer) {
        this.baseUrl = baseUrl;
        this.signer = signer;
    }
    /**
     * Register model on-chain using local signing
     */
    async register(contract, params) {
        if (!this.signer)
            throw new Error('Signer required for on-chain operations');
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
    async list(owner) {
        const url = owner ? `${this.baseUrl}/ai/model/list?owner=${encodeURIComponent(owner)}` : `${this.baseUrl}/ai/model/list`;
        const resp = await requestRaw('GET', url, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`list models failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getLineage(modelId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/lineage`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get model lineage failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async addCheckpoint(modelId, checkpointCid, metricsJsonCid, step) {
        const payload = JSON.stringify({ checkpointCid, metricsJsonCid, step });
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/checkpoint`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`add checkpoint failed ${resp.status}`);
    }
    async publish(modelId, checkpointCid) {
        const payload = JSON.stringify({ checkpointCid });
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/model/${encodeURIComponent(modelId)}/publish`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`publish model failed ${resp.status}`);
    }
}
export class ArthaJob {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }
    async submitTrain(params) {
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
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/train`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`submit train job failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async submitInfer(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/infer`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`submit infer job failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async submitAgent(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/agent`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`submit agent job failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getStatus(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/status`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get job status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getLogs(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/logs`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get job logs failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async cancel(jobId) {
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/cancel`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`cancel job failed ${resp.status}`);
    }
    async getArtifacts(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/job/${encodeURIComponent(jobId)}/artifacts`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get job artifacts failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getToolCalls(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/tool-calls`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get tool calls failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async recordToolCall(jobId, toolName, params, result) {
        const payload = JSON.stringify({ tool_name: toolName, params, result });
        const resp = await requestRaw('POST', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/tool-call`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`record tool call failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getAgentMemory(jobId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/memory`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get agent memory failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async updateAgentMemory(jobId, memoryCid) {
        const payload = JSON.stringify({ memory_cid: memoryCid });
        const resp = await requestRaw('POST', `${this.baseUrl}/agents/${encodeURIComponent(jobId)}/memory`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`update agent memory failed ${resp.status}`);
    }
}
export class ArthaFederated {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }
    async startRound(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/start`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`start federated failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getRoundStatus(fedId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/status`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get federated status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async submitGradient(fedId, weights, sampleCount) {
        const payload = JSON.stringify({ fed_id: fedId, weights, sample_count: sampleCount });
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/gradient`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`submit gradient failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async triggerAggregation(fedId) {
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/federated/${encodeURIComponent(fedId)}/aggregate`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`trigger aggregation failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaEvolution {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }
    async start(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/evolve/start`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`start evolution failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getStatus(evoId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/evolve/${encodeURIComponent(evoId)}/status`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get evolution status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getPopulation(evoId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/evolve/${encodeURIComponent(evoId)}/population`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get evolution population failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
}
export class ArthaDeployment {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }
    async deploy(params) {
        const payload = JSON.stringify(params);
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/deploy`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`deploy model failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async getStatus(deploymentId) {
        const resp = await requestRaw('GET', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}/status`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`get deployment status failed ${resp.status}`);
        return JSON.parse(resp.body.toString('utf8'));
    }
    async scale(deploymentId, replicas) {
        const payload = JSON.stringify({ replicas });
        const resp = await requestRaw('POST', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}/scale`, { 'Content-Type': 'application/json' }, Buffer.from(payload));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`scale deployment failed ${resp.status}`);
    }
    async undeploy(deploymentId) {
        const resp = await requestRaw('DELETE', `${this.baseUrl}/ai/deployment/${encodeURIComponent(deploymentId)}`, {}, Buffer.alloc(0));
        if (resp.status < 200 || resp.status >= 300)
            throw new Error(`undeploy failed ${resp.status}`);
    }
}
//# sourceMappingURL=index.js.map
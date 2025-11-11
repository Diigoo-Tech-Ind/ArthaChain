import json
import os
import urllib.request
import urllib.parse

def _request(method: str, url: str, headers=None, data: bytes | None = None):
    if headers is None:
        headers = {}
    req = urllib.request.Request(url=url, method=method, headers=headers, data=data)
    with urllib.request.urlopen(req) as resp:
        status = resp.status
        body = resp.read()
        return status, resp.headers, body

def _multipart(field: str, filename: str, data: bytes):
    boundary = f"----arthapy-{os.urandom(6).hex()}"
    pre = (f"--{boundary}\r\n"
           f"Content-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\n"
           f"Content-Type: application/octet-stream\r\n\r\n").encode()
    post = f"\r\n--{boundary}--\r\n".encode()
    body = pre + data + post
    return body, f"multipart/form-data; boundary={boundary}"

class ArthaPy:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')

    def upload_file(self, path: str) -> str:
        with open(path, 'rb') as f:
            data = f.read()
        body, content_type = _multipart('file', os.path.basename(path), data)
        status, _, body = _request('POST', f"{self.base_url}/svdb/upload", {'Content-Type': content_type}, body)
        if status < 200 or status >= 300:
            raise RuntimeError(f"upload failed {status}")
        return json.loads(body.decode())['cid']

    def upload_file_with_envelope(self, path: str, envelope: dict) -> str:
        with open(path, 'rb') as f:
            data = f.read()
        body, content_type = _multipart('file', os.path.basename(path), data)
        headers = {'Content-Type': content_type, 'X-Artha-Envelope': json.dumps(envelope)}
        status, _, body = _request('POST', f"{self.base_url}/svdb/upload", headers, body)
        if status < 200 or status >= 300:
            raise RuntimeError(f"upload failed {status}")
        return json.loads(body.decode())['cid']

    def download(self, cid_uri: str, out_path: str, start: int | None = None, end: int | None = None) -> tuple[int,int]:
        cid = cid_uri.replace('artha://', '')
        headers = {}
        if start is not None and end is not None:
            headers['Range'] = f"bytes={start}-{end}"
        elif start is not None:
            headers['Range'] = f"bytes={start}-"
        elif end is not None:
            headers['Range'] = f"bytes=0-{end}"
        status, _, body = _request('GET', f"{self.base_url}/svdb/download/{cid}", headers)
        if status not in (200, 206):
            raise RuntimeError(f"download failed {status}")
        with open(out_path, 'wb') as f:
            f.write(body)
        return status, len(body)

    def info(self, cid_uri: str) -> dict:
        cid = cid_uri.replace('artha://', '')
        status, _, body = _request('GET', f"{self.base_url}/svdb/info/{cid}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"info failed {status}")
        return json.loads(body.decode())

    def build_merkle_branch(self, cid_uri: str, index: int) -> dict:
        payload = json.dumps({'cid': cid_uri, 'index': index}).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/proofs/branch", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"branch failed {status}")
        return json.loads(body.decode())

    def submit_payout(self, *, rpc_url: str, chain_id: int, private_key: str, nonce: int, gas_price: int, gas_limit: int, deal_market: str, root: str, leaf: str, index: int, branch: list[str]) -> dict:
        payload = json.dumps({
            'rpcUrl': rpc_url,
            'chainId': chain_id,
            'privateKey': private_key,
            'nonce': nonce,
            'gasPrice': gas_price,
            'gasLimit': gas_limit,
            'dealMarket': deal_market,
            'root': root,
            'leaf': leaf,
            'index': index,
            'branch': branch,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/proofs/submit", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"submit failed {status}")
        return json.loads(body.decode())

    def proofs_v2_batch_verify(self, rpc_url: str, proofs_v2: str, data_hex: str) -> dict:
        payload = json.dumps({ 'rpcUrl': rpc_url, 'proofsV2': proofs_v2, 'data': data_hex }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/proofs/v2/batch/verify", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"batch verify failed {status}")
        return json.loads(body.decode())

    def proofs_v2_batch_submit(self, *, rpc_url: str, chain_id: int, private_key: str, deal_market: str, data_hex: str, gas_price: int | None = None, gas_limit: int | None = None, nonce: int | None = None) -> dict:
        payload = json.dumps({ 'rpcUrl': rpc_url, 'chainId': chain_id, 'privateKey': private_key, 'dealMarket': deal_market, 'data': data_hex, 'gasPrice': gas_price, 'gasLimit': gas_limit, 'nonce': nonce }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/proofs/v2/batch/submit", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"batch submit failed {status}")
        return json.loads(body.decode())

    def quote(self, provider: str, cid_uri: str) -> dict:
        payload = json.dumps({ 'provider': provider, 'cid': cid_uri }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/retrieval/quote", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"quote failed {status}")
        return json.loads(body.decode())

    def settle(self, *, rpc_url: str, chain_id: int, private_key: str, deal_market: str, manifest_root: str, bytes_served: int, provider: str, total_wei: int, gas_price: int | None = None, gas_limit: int | None = None, nonce: int | None = None) -> dict:
        payload = json.dumps({ 'rpcUrl': rpc_url, 'chainId': chain_id, 'privateKey': private_key, 'dealMarket': deal_market, 'manifestRoot': manifest_root, 'bytesServed': bytes_served, 'provider': provider, 'totalWei': total_wei, 'gasPrice': gas_price, 'gasLimit': gas_limit, 'nonce': nonce }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/retrieval/settle", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"settle failed {status}")
        return json.loads(body.decode())

    def settle_aggregate(self, *, rpc_url: str, chain_id: int, private_key: str, deal_market: str, manifest_root: str, merkle_root: str, provider: str, total_wei: int, gas_price: int | None = None, gas_limit: int | None = None, nonce: int | None = None) -> dict:
        payload = json.dumps({ 'rpcUrl': rpc_url, 'chainId': chain_id, 'privateKey': private_key, 'dealMarket': deal_market, 'manifestRoot': manifest_root, 'merkleRoot': merkle_root, 'provider': provider, 'totalWei': total_wei, 'gasPrice': gas_price, 'gasLimit': gas_limit, 'nonce': nonce }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/retrieval/settle", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"aggregate settle failed {status}")
        return json.loads(body.decode())

    def settle_aggregate_with_proof(self, *, rpc_url: str, chain_id: int, private_key: str, deal_market: str, manifest_root: str, merkle_root: str, leaf: str, branch: list[str], index: int, provider: str, total_wei: int, gas_price: int | None = None, gas_limit: int | None = None, nonce: int | None = None) -> dict:
        payload = json.dumps({ 'rpcUrl': rpc_url, 'chainId': chain_id, 'privateKey': private_key, 'dealMarket': deal_market, 'manifestRoot': manifest_root, 'merkleRoot': merkle_root, 'leaf': leaf, 'branch': branch, 'index': index, 'provider': provider, 'totalWei': total_wei, 'gasPrice': gas_price, 'GasLimit': gas_limit, 'nonce': nonce }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/retrieval/settle", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"aggregate settle proof failed {status}")
        return json.loads(body.decode())

    def set_access_policy(self, *, cid_uri: str, private: bool, allowed_dids: list[str] | None = None, token: str | None = None) -> dict:
        payload = json.dumps({ 'cid': cid_uri, 'private': private, 'allowedDids': allowed_dids or [], 'token': token }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/access/policy", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"access policy failed {status}")
        return json.loads(body.decode())

    def allowlist_add(self, *, cid_uri: str, did: str) -> dict:
        payload = json.dumps({ 'cid': cid_uri, 'did': did }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/access/allowlist/add", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"allowlist add failed {status}")
        return json.loads(body.decode())

    def allowlist_remove(self, *, cid_uri: str, did: str) -> dict:
        payload = json.dumps({ 'cid': cid_uri, 'did': did }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/access/allowlist/remove", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"allowlist remove failed {status}")
        return json.loads(body.decode())

    # Phase 4: Marketplace & SLA
    def get_active_providers(self, rpc_url: str, contract: str) -> dict:
        url = f"{self.base_url}/svdb/marketplace/providers?rpcUrl={urllib.parse.quote(rpc_url)}&contract={urllib.parse.quote(contract)}"
        status, _, body = _request('GET', url)
        if status < 200 or status >= 300:
            raise RuntimeError(f"get providers failed {status}")
        return json.loads(body.decode())

    def get_provider_offer(self, provider: str, rpc_url: str, contract: str) -> dict:
        url = f"{self.base_url}/svdb/marketplace/offer/{provider}?rpcUrl={urllib.parse.quote(rpc_url)}&contract={urllib.parse.quote(contract)}"
        status, _, body = _request('GET', url)
        if status < 200 or status >= 300:
            raise RuntimeError(f"get offer failed {status}")
        return json.loads(body.decode())

    def get_provider_reputation(self, provider: str, rpc_url: str, contract: str) -> dict:
        url = f"{self.base_url}/svdb/marketplace/reputation/{provider}?rpcUrl={urllib.parse.quote(rpc_url)}&contract={urllib.parse.quote(contract)}"
        status, _, body = _request('GET', url)
        if status < 200 or status >= 300:
            raise RuntimeError(f"get reputation failed {status}")
        return json.loads(body.decode())

    def report_latency(self, *, client: str, provider: str, root: str, latency_ms: int, rpc_url: str, contract: str, private_key: str) -> dict:
        payload = json.dumps({
            'client': client,
            'provider': provider,
            'root': root,
            'latencyMs': latency_ms,
            'rpcUrl': rpc_url,
            'contract': contract,
            'privateKey': private_key
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/sla/report_latency", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"report latency failed {status}")
        return json.loads(body.decode())

    # Phase 4: PoRep GPU proving
    def porep_prove_seal(self, *, root: str, randomness: str, provider: str) -> dict:
        payload = json.dumps({ 'root': root, 'randomness': randomness, 'provider': provider }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/porep/prove_seal", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"prove seal failed {status}")
        return json.loads(body.decode())

    def porep_challenge(self, *, commitment: str, rpc_url: str, contract: str, private_key: str) -> dict:
        payload = json.dumps({ 'commitment': commitment, 'rpcUrl': rpc_url, 'contract': contract, 'privateKey': private_key }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/porep/challenge", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"issue challenge failed {status}")
        return json.loads(body.decode())

    # Phase 4: One-click AI
    def ai_train(self, *, model_cid: str, dataset_cid: str, epochs: int | None = None, region: str | None = None, zk_enabled: bool | None = None, gpu_required: bool | None = None) -> dict:
        payload = json.dumps({
            'modelCid': model_cid,
            'datasetCid': dataset_cid,
            'epochs': epochs,
            'region': region,
            'zkEnabled': zk_enabled,
            'gpuRequired': gpu_required
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/ai/train", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"train failed {status}")
        return json.loads(body.decode())

    def ai_job_status(self, job_id: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/svdb/ai/job/{job_id}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"job status failed {status}")
        return json.loads(body.decode())

    def ai_deploy(self, *, model_cid: str, name: str | None = None, region: str | None = None, replicas: int | None = None) -> dict:
        payload = json.dumps({ 'modelCid': model_cid, 'name': name, 'region': region, 'replicas': replicas }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/ai/deploy", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"deploy failed {status}")
        return json.loads(body.decode())

    def ai_deployment_status(self, deployment_id: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/svdb/ai/deploy/{deployment_id}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"deployment status failed {status}")
        return json.loads(body.decode())

    # Phase 4: Analytics
    def explorer_proofs(self, cid: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/svdb/explorer/proofs/{urllib.parse.quote(cid)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"explorer proofs failed {status}")
        return json.loads(body.decode())

    def estimate_cost(self, *, size: int, replicas: int, months: int, rpc_url: str | None = None, price_oracle: str | None = None) -> dict:
        payload = json.dumps({ 'size': size, 'replicas': replicas, 'months': months, 'rpcUrl': rpc_url, 'priceOracle': price_oracle }).encode()
        status, _, body = _request('POST', f"{self.base_url}/svdb/explorer/cost/estimate", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"cost estimate failed {status}")
        return json.loads(body.decode())


# ============================================================================
# Identity & AI Extensions
# ============================================================================

class ArthaID:
    def __init__(self, base_url: str, rpc_url: str):
        self.base_url = base_url.rstrip('/')
        self.rpc_url = rpc_url

    def create_did(self, auth_key: str, enc_key: str, meta_cid: str) -> dict:
        payload = json.dumps({'authKey': auth_key, 'encKey': enc_key, 'metaCid': meta_cid}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/did/create", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"create_did failed {status}")
        return json.loads(body.decode())

    def get_did(self, did: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/did/{urllib.parse.quote(did)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get_did failed {status}")
        return json.loads(body.decode())

    def rotate_keys(self, did: str, new_auth_key: str, new_enc_key: str) -> dict:
        payload = json.dumps({'did': did, 'newAuthKey': new_auth_key, 'newEncKey': new_enc_key}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/did/rotate", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"rotate_keys failed {status}")
        return json.loads(body.decode())

    def revoke_did(self, did: str) -> dict:
        payload = json.dumps({'did': did}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/did/revoke", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"revoke_did failed {status}")
        return json.loads(body.decode())

    def verify_signature(self, did: str, message_hash: str, signature: str) -> dict:
        payload = json.dumps({'did': did, 'messageHash': message_hash, 'signature': signature}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/did/verify", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"verify_signature failed {status}")
        return json.loads(body.decode())


class ArthaVC:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')

    def issue_vc(self, issuer_did: str, subject_did: str, claim_hash: str, doc_cid: str, expires_at: int) -> dict:
        payload = json.dumps({'issuerDid': issuer_did, 'subjectDid': subject_did, 'claimHash': claim_hash, 'docCid': doc_cid, 'expiresAt': expires_at}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/vc/issue", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"issue_vc failed {status}")
        return json.loads(body.decode())

    def revoke_vc(self, vc_hash: str) -> dict:
        payload = json.dumps({'vcHash': vc_hash}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/vc/revoke", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"revoke_vc failed {status}")
        return json.loads(body.decode())

    def verify_vc(self, vc_hash: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/vc/{urllib.parse.quote(vc_hash)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"verify_vc failed {status}")
        return json.loads(body.decode())

    def get_vcs_by_subject(self, subject_did: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/vc/subject/{urllib.parse.quote(subject_did)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get_vcs_by_subject failed {status}")
        return json.loads(body.decode())

    def has_claim_type(self, subject_did: str, claim_type: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/vc/claim/{urllib.parse.quote(subject_did)}/{urllib.parse.quote(claim_type)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"has_claim_type failed {status}")
        return json.loads(body.decode())


class ArthaAIID:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')

    def create_aiid(self, owner_did: str, model_cid: str, dataset_id: str, code_hash: str, version: str) -> dict:
        payload = json.dumps({'ownerDid': owner_did, 'modelCid': model_cid, 'datasetId': dataset_id, 'codeHash': code_hash, 'version': version}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/aiid/create", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"create_aiid failed {status}")
        return json.loads(body.decode())

    def get_aiid(self, aiid: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/aiid/{urllib.parse.quote(aiid)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get_aiid failed {status}")
        return json.loads(body.decode())

    def rotate_aiid(self, aiid: str, new_model_cid: str, new_version: str) -> dict:
        payload = json.dumps({'aiid': aiid, 'newModelCid': new_model_cid, 'newVersion': new_version}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/aiid/rotate", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"rotate_aiid failed {status}")
        return json.loads(body.decode())

    def link_owner(self, aiid: str, owner_did: str) -> dict:
        payload = json.dumps({'aiid': aiid, 'ownerDid': owner_did}).encode()
        status, _, body = _request('POST', f"{self.base_url}/identity/aiid/link", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"link_owner failed {status}")
        return json.loads(body.decode())

    def get_lineage(self, aiid: str) -> dict:
        status, _, body = _request('GET', f"{self.base_url}/identity/aiid/{urllib.parse.quote(aiid)}/lineage")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get_lineage failed {status}")
        return json.loads(body.decode())


class ArthaPolicy:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')

    def check_access(self, cid: str, did: str, session_token: str) -> dict:
        payload = json.dumps({'cid': cid, 'did': did, 'sessionToken': session_token}).encode()
        status, _, body = _request('POST', f"{self.base_url}/policy/check", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"check_access failed {status}")
        return json.loads(body.decode())

    def create_session(self, did: str, scope: list[str]) -> dict:
        payload = json.dumps({'did': did, 'scope': scope}).encode()
        status, _, body = _request('POST', f"{self.base_url}/policy/session/create", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"create_session failed {status}")
        return json.loads(body.decode())

    def revoke_session(self, session_id: str) -> dict:
        payload = json.dumps({'sessionId': session_id}).encode()
        status, _, body = _request('POST', f"{self.base_url}/policy/session/revoke", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"revoke_session failed {status}")
        return json.loads(body.decode())


class ArthaAI:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')

    def score_vc_risk(self, vc_input: dict) -> dict:
        payload = json.dumps(vc_input).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/risk/score", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"score_vc_risk failed {status}")
        return json.loads(body.decode())

    def detect_anomaly(self, node_metrics: dict) -> dict:
        payload = json.dumps(node_metrics).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/anomaly/detect", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"detect_anomaly failed {status}")
        return json.loads(body.decode())

    def score_reputation(self, reputation_input: dict) -> dict:
        payload = json.dumps(reputation_input).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/reputation/score", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"score_reputation failed {status}")
        return json.loads(body.decode())

    def verify_authenticity(self, receipt: dict) -> dict:
        payload = json.dumps(receipt).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/authenticity/verify", {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"verify_authenticity failed {status}")
        return json.loads(body.decode())


# ============================================================================
# ArthaAIN v1 - AI Cloud Platform Classes
# ============================================================================

class ArthaDataset:
    """Dataset management for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def register(self, root_cid: str, license_cid: str, tags: list[str]) -> str:
        """Register a dataset on-chain"""
        payload = json.dumps({"rootCid": root_cid, "licenseCid": license_cid, "tags": tags}).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/dataset/register", 
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"register dataset failed {status}")
        return json.loads(body.decode())['dataset_id']
    
    def list(self, owner: str | None = None) -> list[dict]:
        """List datasets, optionally filtered by owner"""
        url = f"{self.base_url}/ai/dataset/list"
        if owner:
            url += f"?owner={urllib.parse.quote(owner)}"
        status, _, body = _request('GET', url)
        if status < 200 or status >= 300:
            raise RuntimeError(f"list datasets failed {status}")
        return json.loads(body.decode())
    
    def get_info(self, dataset_id: str) -> dict:
        """Get dataset information"""
        status, _, body = _request('GET', f"{self.base_url}/ai/dataset/{urllib.parse.quote(dataset_id)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get dataset info failed {status}")
        return json.loads(body.decode())


class ArthaModel:
    """Model management for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def register(self, model_cid: str, architecture: str, dataset_id: str, 
                 code_hash: str, version: str, base_model_id: str | None = None,
                 license_cid: str | None = None) -> str:
        """Register a model on-chain"""
        params = {
            "modelCid": model_cid,
            "architecture": architecture,
            "datasetId": dataset_id,
            "codeHash": code_hash,
            "version": version,
        }
        if base_model_id:
            params["baseModelId"] = base_model_id
        if license_cid:
            params["licenseCid"] = license_cid
        
        payload = json.dumps(params).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/model/register",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"register model failed {status}")
        return json.loads(body.decode())['model_id']
    
    def list(self, owner: str | None = None) -> list[dict]:
        """List models, optionally filtered by owner"""
        url = f"{self.base_url}/ai/model/list"
        if owner:
            url += f"?owner={urllib.parse.quote(owner)}"
        status, _, body = _request('GET', url)
        if status < 200 or status >= 300:
            raise RuntimeError(f"list models failed {status}")
        return json.loads(body.decode())
    
    def get_lineage(self, model_id: str) -> list[str]:
        """Get model lineage (parent chain)"""
        status, _, body = _request('GET', f"{self.base_url}/ai/model/{urllib.parse.quote(model_id)}/lineage")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get model lineage failed {status}")
        return json.loads(body.decode())
    
    def add_checkpoint(self, model_id: str, checkpoint_cid: str, metrics_json_cid: str, step: int):
        """Add checkpoint to model"""
        payload = json.dumps({"checkpointCid": checkpoint_cid, "metricsJsonCid": metrics_json_cid, "step": step}).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/model/{urllib.parse.quote(model_id)}/checkpoint",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"add checkpoint failed {status}")
    
    def publish(self, model_id: str, checkpoint_cid: str):
        """Publish model checkpoint"""
        payload = json.dumps({"checkpointCid": checkpoint_cid}).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/model/{urllib.parse.quote(model_id)}/publish",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"publish model failed {status}")


class ArthaJob:
    """Job management for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def submit_train(self, model_id: str, dataset_id: str, submitter_did: str,
                     epochs: int, batch_size: int, learning_rate: float,
                     optimizer: str, budget: int) -> dict:
        """Submit training job"""
        payload = json.dumps({
            "modelId": model_id,
            "datasetId": dataset_id,
            "submitterDid": submitter_did,
            "params": {
                "epochs": epochs,
                "batchSize": batch_size,
                "learningRate": learning_rate,
                "optimizer": optimizer,
                "checkpointInterval": 500,
            },
            "budget": budget,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/train",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"submit train job failed {status}")
        return json.loads(body.decode())
    
    def submit_infer(self, model_id: str, submitter_did: str, mode: str, budget: int,
                     input_cid: str | None = None, inline_input: str | None = None,
                     max_tokens: int | None = None) -> dict:
        """Submit inference job"""
        params = {
            "modelId": model_id,
            "submitterDid": submitter_did,
            "mode": mode,
            "budget": budget,
        }
        if input_cid:
            params["inputCid"] = input_cid
        if inline_input:
            params["inlineInput"] = inline_input
        if max_tokens:
            params["maxTokens"] = max_tokens
        
        payload = json.dumps(params).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/infer",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"submit infer job failed {status}")
        return json.loads(body.decode())
    
    def submit_agent(self, agent_spec_cid: str, submitter_did: str, goal: str,
                     tools: list[str], memory_policy: str, budget: int) -> dict:
        """Submit agent job"""
        payload = json.dumps({
            "agentSpecCid": agent_spec_cid,
            "submitterDid": submitter_did,
            "goal": goal,
            "tools": tools,
            "memoryPolicy": memory_policy,
            "budget": budget,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/agent",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"submit agent job failed {status}")
        return json.loads(body.decode())
    
    def get_status(self, job_id: str) -> dict:
        """Get job status"""
        status, _, body = _request('GET', f"{self.base_url}/ai/job/{urllib.parse.quote(job_id)}/status")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get job status failed {status}")
        return json.loads(body.decode())
    
    def get_logs(self, job_id: str) -> list[str]:
        """Get job logs"""
        status, _, body = _request('GET', f"{self.base_url}/ai/job/{urllib.parse.quote(job_id)}/logs")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get job logs failed {status}")
        return json.loads(body.decode())
    
    def cancel(self, job_id: str):
        """Cancel job"""
        status, _, _ = _request('POST', f"{self.base_url}/ai/job/{urllib.parse.quote(job_id)}/cancel")
        if status < 200 or status >= 300:
            raise RuntimeError(f"cancel job failed {status}")
    
    def get_artifacts(self, job_id: str) -> list[str]:
        """Get job artifacts (checkpoint CIDs)"""
        status, _, body = _request('GET', f"{self.base_url}/ai/job/{urllib.parse.quote(job_id)}/artifacts")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get job artifacts failed {status}")
        return json.loads(body.decode())
    
    def get_tool_calls(self, job_id: str) -> list[dict]:
        """Get agent tool calls for a job"""
        status, _, body = _request('GET', f"{self.base_url}/agents/{urllib.parse.quote(job_id)}/tool-calls")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get tool calls failed {status}")
        return json.loads(body.decode())
    
    def record_tool_call(self, job_id: str, tool_name: str, params: dict, result: dict) -> dict:
        """Record a tool call for an agent job"""
        payload = json.dumps({
            "tool_name": tool_name,
            "params": params,
            "result": result,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/agents/{urllib.parse.quote(job_id)}/tool-call",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"record tool call failed {status}")
        return json.loads(body.decode())
    
    def get_agent_memory(self, job_id: str) -> dict:
        """Get agent memory for a job"""
        status, _, body = _request('GET', f"{self.base_url}/agents/{urllib.parse.quote(job_id)}/memory")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get agent memory failed {status}")
        return json.loads(body.decode())
    
    def update_agent_memory(self, job_id: str, memory_cid: str):
        """Update agent memory for a job"""
        payload = json.dumps({"memory_cid": memory_cid}).encode()
        status, _, _ = _request('POST', f"{self.base_url}/agents/{urllib.parse.quote(job_id)}/memory",
                               {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"update agent memory failed {status}")


class ArthaFederated:
    """Federated learning for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def start_round(self, model_id: str, dataset_ids: list[str], rounds: int,
                    dp: bool, budget: int) -> dict:
        """Start federated learning round"""
        payload = json.dumps({
            "modelId": model_id,
            "datasetIds": dataset_ids,
            "rounds": rounds,
            "dp": dp,
            "budget": budget,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/federated/start",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"start federated failed {status}")
        return json.loads(body.decode())
    
    def get_round_status(self, fed_id: str) -> dict:
        """Get federated learning round status"""
        status, _, body = _request('GET', f"{self.base_url}/ai/federated/{urllib.parse.quote(fed_id)}/status")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get federated status failed {status}")
        return json.loads(body.decode())
    
    def submit_gradient(self, fed_id: str, weights: list[float], sample_count: int) -> dict:
        """Submit gradient update for federated learning"""
        payload = json.dumps({
            "fed_id": fed_id,
            "weights": weights,
            "sample_count": sample_count,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/federated/{urllib.parse.quote(fed_id)}/gradient",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"submit gradient failed {status}")
        return json.loads(body.decode())
    
    def trigger_aggregation(self, fed_id: str) -> dict:
        """Trigger aggregation for federated learning round"""
        status, _, body = _request('POST', f"{self.base_url}/ai/federated/{urllib.parse.quote(fed_id)}/aggregate")
        if status < 200 or status >= 300:
            raise RuntimeError(f"trigger aggregation failed {status}")
        return json.loads(body.decode())


class ArthaEvolution:
    """Evolutionary AI for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def start(self, search_space_cid: str, population: int, generations: int, budget: int) -> dict:
        """Start evolutionary search"""
        payload = json.dumps({
            "searchSpaceCid": search_space_cid,
            "population": population,
            "generations": generations,
            "budget": budget,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/evolve/start",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"start evolution failed {status}")
        return json.loads(body.decode())
    
    def get_status(self, evo_id: str) -> dict:
        """Get evolution status"""
        status, _, body = _request('GET', f"{self.base_url}/ai/evolve/{urllib.parse.quote(evo_id)}/status")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get evolution status failed {status}")
        return json.loads(body.decode())
    
    def get_population(self, evo_id: str) -> dict:
        """Get evolution population status"""
        status, _, body = _request('GET', f"{self.base_url}/ai/evolve/{urllib.parse.quote(evo_id)}/population")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get evolution population failed {status}")
        return json.loads(body.decode())


class ArthaDeployment:
    """Model deployment for ArthaAIN v1"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
    
    def deploy(self, model_id: str, endpoint: str, replicas: int, max_tokens: int) -> dict:
        """Deploy model for inference"""
        payload = json.dumps({
            "modelId": model_id,
            "endpoint": endpoint,
            "replicas": replicas,
            "maxTokens": max_tokens,
        }).encode()
        status, _, body = _request('POST', f"{self.base_url}/ai/deploy",
                                   {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"deploy model failed {status}")
        return json.loads(body.decode())
    
    def get_status(self, deployment_id: str) -> dict:
        """Get deployment status"""
        status, _, body = _request('GET', f"{self.base_url}/ai/deployment/{urllib.parse.quote(deployment_id)}/status")
        if status < 200 or status >= 300:
            raise RuntimeError(f"get deployment status failed {status}")
        return json.loads(body.decode())
    
    def scale(self, deployment_id: str, replicas: int):
        """Scale deployment"""
        payload = json.dumps({"replicas": replicas}).encode()
        status, _, _ = _request('POST', f"{self.base_url}/ai/deployment/{urllib.parse.quote(deployment_id)}/scale",
                               {'Content-Type': 'application/json'}, payload)
        if status < 200 or status >= 300:
            raise RuntimeError(f"scale deployment failed {status}")
    
    def undeploy(self, deployment_id: str):
        """Undeploy model"""
        status, _, _ = _request('DELETE', f"{self.base_url}/ai/deployment/{urllib.parse.quote(deployment_id)}")
        if status < 200 or status >= 300:
            raise RuntimeError(f"undeploy failed {status}")



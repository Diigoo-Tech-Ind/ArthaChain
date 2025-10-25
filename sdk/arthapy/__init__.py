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



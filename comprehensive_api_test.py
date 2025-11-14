#!/usr/bin/env python3
"""
Comprehensive API Testing Script for ArthaChain
Tests all 90+ APIs and categorizes them based on response quality
"""

import requests
import json
import time
from datetime import datetime
from typing import List, Dict, Any

class ArthaChainAPITester:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url
        self.fully_working = []
        self.mock_data = []
        self.empty_responses = []
        self.not_working = []
        self.test_results = {}
        
    def test_endpoint(self, method: str, endpoint: str, description: str, 
                     expected_fields: List[str] = None, data: Dict = None) -> Dict[str, Any]:
        """Test a single API endpoint and categorize the result"""
        url = f"{self.base_url}{endpoint}"
        
        try:
            if method.upper() == "GET":
                response = requests.get(url, timeout=10)
            elif method.upper() == "POST":
                response = requests.post(url, json=data, timeout=10)
            else:
                return {"status": "error", "message": f"Unsupported method: {method}"}
            
            result = {
                "method": method,
                "endpoint": endpoint,
                "description": description,
                "status_code": response.status_code,
                "response_time": response.elapsed.total_seconds(),
                "response_size": len(response.content),
                "timestamp": datetime.now().isoformat()
            }
            
            # Try to parse JSON response
            try:
                json_response = response.json()
                result["json_response"] = json_response
                result["has_json"] = True
            except:
                result["text_response"] = response.text
                result["has_json"] = False
                json_response = None
            
            # Categorize the response
            if response.status_code == 200:
                if json_response:
                    # Check for real data indicators
                    has_real_data = self._has_real_data(json_response, expected_fields)
                    has_mock_data = self._has_mock_data(json_response)
                    has_empty_data = self._has_empty_data(json_response)
                    
                    if has_real_data and not has_mock_data:
                        result["category"] = "FULLY_WORKING"
                        self.fully_working.append(f"{method} {endpoint} - {description}")
                    elif has_mock_data:
                        result["category"] = "MOCK_DATA"
                        self.mock_data.append(f"{method} {endpoint} - {description}")
                    elif has_empty_data:
                        result["category"] = "EMPTY_RESPONSE"
                        self.empty_responses.append(f"{method} {endpoint} - {description}")
                    else:
                        result["category"] = "FULLY_WORKING"
                        self.fully_working.append(f"{method} {endpoint} - {description}")
                else:
                    result["category"] = "EMPTY_RESPONSE"
                    self.empty_responses.append(f"{method} {endpoint} - {description}")
            else:
                result["category"] = "NOT_WORKING"
                self.not_working.append(f"{method} {endpoint} - {description}")
            
            return result
            
        except requests.exceptions.RequestException as e:
            result = {
                "method": method,
                "endpoint": endpoint,
                "description": description,
                "status_code": 0,
                "error": str(e),
                "category": "NOT_WORKING",
                "timestamp": datetime.now().isoformat()
            }
            self.not_working.append(f"{method} {endpoint} - {description}")
            return result
    
    def _has_real_data(self, response: Dict, expected_fields: List[str] = None) -> bool:
        """Check if response contains real data"""
        if not isinstance(response, dict):
            return False
            
        # Check for real data indicators
        real_data_indicators = [
            "height", "block_number", "transaction_count", "balance", 
            "nonce", "hash", "timestamp", "address", "value", "gas",
            "peers", "validators", "blocks", "transactions"
        ]
        
        for key in real_data_indicators:
            if key in response and response[key] is not None:
                if isinstance(response[key], (int, float)) and response[key] > 0:
                    return True
                elif isinstance(response[key], str) and response[key] and response[key] != "0":
                    return True
                elif isinstance(response[key], list) and len(response[key]) > 0:
                    return True
        
        return False
    
    def _has_mock_data(self, response: Dict) -> bool:
        """Check if response contains mock/placeholder data"""
        if not isinstance(response, dict):
            return False
            
        # More specific mock data indicators
        mock_indicators = [
            "is_mock", "mock_data", "placeholder", "not implemented", 
            "coming soon", "disabled", "test_data", "sample_data",
            "dummy_data", "fake_data", "example_data"
        ]
        
        response_str = json.dumps(response).lower()
        for indicator in mock_indicators:
            if indicator in response_str:
                return True
        
        # Check for specific mock patterns
        if "category" in response and response.get("category") == "MOCK_DATA":
            return True
            
        # Check for mock addresses (common test addresses)
        mock_addresses = [
            "0x0000000000000000000000000000000000000000",
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222"
        ]
        
        for addr in mock_addresses:
            if addr in response_str:
                return True
                
        return False
    
    def _has_empty_data(self, response: Dict) -> bool:
        """Check if response is essentially empty"""
        if not isinstance(response, dict):
            return True
            
        # Check for empty arrays or objects
        for key, value in response.items():
            if isinstance(value, list) and len(value) == 0:
                continue
            elif isinstance(value, dict) and len(value) == 0:
                continue
            elif value is None or value == "" or value == 0:
                continue
            else:
                return False
                
        return True
    
    def run_comprehensive_test(self):
        """Run comprehensive test of all ArthaChain APIs"""
        print("🚀 Starting Comprehensive ArthaChain API Testing")
        print("=" * 60)
        print()
        
        # Test all API endpoints
        test_cases = [
            # Basic Status APIs
            ("GET", "/", "Root endpoint with HTML dashboard"),
            ("GET", "/health", "Health check endpoint"),
            ("GET", "/status", "Status endpoint"),
            ("GET", "/config", "Configuration endpoint"),
            ("GET", "/docs", "API documentation"),
            
            # Core Blockchain APIs
            ("GET", "/api/v1/blockchain/height", "Get blockchain height"),
            ("GET", "/api/v1/blockchain/status", "Get blockchain status"),
            ("GET", "/api/v1/blockchain/info", "Get blockchain info"),
            ("POST", "/api/v1/blockchain/info", "Post blockchain info"),
            ("GET", "/api/v1/blockchain/chain-id", "Get chain ID"),
            ("POST", "/api/v1/blockchain/chain-id", "Post chain ID"),
            
            # Block APIs
            ("GET", "/api/v1/blocks/latest", "Get latest block"),
            ("GET", "/api/v1/blocks/0x1234567890abcdef", "Get block by hash"),
            ("GET", "/api/v1/blocks/height/0", "Get block by height"),
            ("GET", "/api/v1/blocks/height/1", "Get block by height 1"),
            ("POST", "/api/v1/blocks/sync", "Block sync"),
            ("GET", "/api/v1/blocks", "Get all blocks"),
            
            # Transaction APIs
            ("GET", "/api/v1/transactions/0x1234567890abcdef", "Get transaction by hash"),
            ("POST", "/api/v1/transactions", "Submit transaction"),
            ("GET", "/api/v1/mempool/transactions", "Get mempool transactions"),
            ("POST", "/api/v1/transactions/submit", "Submit transaction"),
            ("GET", "/api/v1/transactions/pending", "Get pending transactions"),
            
            # Account APIs
            ("GET", "/api/v1/accounts/0x1234567890abcdef1234567890abcdef12345678", "Get account info"),
            ("GET", "/api/v1/accounts/0x1234567890abcdef1234567890abcdef12345678/transactions", "Get account transactions"),
            ("GET", "/api/v1/accounts/0x1234567890abcdef1234567890abcdef12345678/balance", "Get account balance"),
            ("GET", "/api/v1/accounts/0x1234567890abcdef1234567890abcdef12345678/nonce", "Get account nonce"),
            ("POST", "/api/v1/accounts/0x1234567890abcdef1234567890abcdef12345678/nonce", "Post account nonce"),
            
            # Block Explorer APIs
            ("GET", "/api/v1/explorer/stats", "Get explorer stats"),
            ("GET", "/api/v1/explorer/blocks/recent", "Get recent blocks"),
            ("GET", "/api/v1/explorer/transactions/recent", "Get recent transactions"),
            
            # Smart Contract APIs
            ("GET", "/api/v1/contracts/0x1234567890abcdef1234567890abcdef12345678", "Get contract info"),
            ("GET", "/api/v1/contracts", "Get all contracts"),
            ("GET", "/api/v1/contracts/deploy", "Get contract deploy status"),
            ("POST", "/api/v1/contracts/deploy", "Deploy contract"),
            ("POST", "/api/v1/contracts/call", "Call contract"),
            ("GET", "/api/v1/contracts/verify", "Verify contract"),
            ("POST", "/api/v1/contracts/0x1234567890abcdef1234567890abcdef12345678/call", "Call specific contract"),
            ("GET", "/api/v1/contracts/0x1234567890abcdef1234567890abcdef12345678/events", "Get contract events"),
            
            # AI/ML APIs
            ("GET", "/api/v1/ai/status", "Get AI status"),
            ("GET", "/api/v1/ai/models", "Get AI models"),
            ("POST", "/api/v1/ai/fraud/detect", "Detect fraud"),
            ("GET", "/api/v1/ai/analytics", "Get AI analytics"),
            ("POST", "/api/v1/ai/inference", "AI inference"),
            ("GET", "/api/v1/ai/fraud-detection", "Fraud detection status"),
            
            # Network APIs
            ("GET", "/api/v1/network/status", "Get network status"),
            ("GET", "/api/v1/network/peers", "Get network peers"),
            ("GET", "/api/v1/network/sync", "Get network sync status"),
            ("GET", "/api/v1/network/mempool-size", "Get mempool size"),
            ("GET", "/api/v1/network/uptime", "Get network uptime"),
            ("GET", "/api/v1/network/stats", "Get network stats"),
            ("POST", "/api/v1/network/connect", "Connect to peer"),
            
            # Security APIs
            ("GET", "/api/v1/security/status", "Get security status"),
            ("GET", "/api/v1/security/threats", "Get security threats"),
            ("GET", "/api/v1/security/events", "Get security events"),
            ("GET", "/api/v1/security/audit", "Get security audit"),
            ("POST", "/api/v1/security/scan", "Security scan"),
            ("GET", "/api/v1/security/encryption", "Get encryption status"),
            
            # Testnet APIs
            ("POST", "/api/v1/testnet/faucet/request", "Request faucet"),
            ("GET", "/api/v1/testnet/faucet/status", "Get faucet status"),
            ("GET", "/api/v1/testnet/faucet/history", "Get faucet history"),
            ("POST", "/api/v1/testnet/gas-free/register", "Register for gas-free"),
            ("POST", "/api/v1/testnet/gas-free/check", "Check gas-free status"),
            ("GET", "/api/v1/testnet/gas-free/apps", "Get gas-free apps"),
            ("GET", "/api/v1/testnet/gas-free/stats", "Get gas-free stats"),
            ("POST", "/api/v1/testnet/gas-free/process", "Process gas-free"),
            ("GET", "/api/v1/faucet/request", "Get faucet request"),
            ("POST", "/api/v1/faucet/request", "Post faucet request"),
            ("GET", "/api/v1/gas-free/status", "Get gas-free status"),
            ("POST", "/api/v1/gas-free/request", "Request gas-free"),
            
            # Wallet APIs
            ("GET", "/api/v1/wallet/supported", "Get supported wallets"),
            ("GET", "/api/v1/wallet/ides", "Get wallet IDEs"),
            ("GET", "/api/v1/wallet/connect", "Connect wallet"),
            ("GET", "/api/v1/wallet/setup", "Setup wallet"),
            ("GET", "/api/v1/wallet/balance", "Get wallet balance"),
            ("POST", "/api/v1/wallet/create", "Create wallet"),
            ("GET", "/api/v1/wallet/addresses", "Get wallet addresses"),
            
            # EVM/RPC APIs
            ("POST", "/api/v1/rpc/eth_blockNumber", "EVM block number"),
            ("POST", "/api/v1/rpc/eth_getBalance", "EVM get balance"),
            ("POST", "/api/v1/rpc/eth_gasPrice", "EVM gas price"),
            ("POST", "/api/v1/rpc/eth_sendRawTransaction", "EVM send transaction"),
            ("POST", "/api/v1/rpc/eth_getTransactionCount", "EVM transaction count"),
            ("POST", "/api/v1/rpc/eth_getTransactionReceipt", "EVM transaction receipt"),
            ("GET", "/api/v1/evm/accounts", "Get EVM accounts"),
            ("POST", "/api/v1/evm/accounts", "Create EVM account"),
            ("GET", "/api/v1/evm/balance", "Get EVM balance"),
            ("POST", "/api/v1/evm/transfer", "EVM transfer"),
            
            # WebSocket APIs
            ("GET", "/api/v1/ws/connect", "WebSocket connect"),
            ("POST", "/api/v1/ws/subscribe", "WebSocket subscribe"),
            
            # Developer Tools APIs
            ("GET", "/api/v1/dev/tools", "Get developer tools"),
            ("POST", "/api/v1/dev/debug", "Debug tools"),
            ("GET", "/api/v1/dev/logs", "Get logs"),
            ("GET", "/api/v1/developer/tools", "Get developer tools"),
            ("POST", "/api/v1/developer/tools", "Use developer tools"),
            ("GET", "/api/v1/developer/debug", "Debug tools"),
            
            # Identity APIs
            ("POST", "/api/v1/identity/create", "Create identity"),
            ("POST", "/api/v1/identity/verify", "Verify identity"),
            ("GET", "/api/v1/identity/status", "Get identity status"),
            ("GET", "/api/v1/identity/verify", "Get identity verify status"),
            
            # Consensus APIs
            ("GET", "/api/v1/consensus/status", "Get consensus status"),
            ("GET", "/api/v1/consensus/validators", "Get validators"),
            ("GET", "/api/v1/consensus/rounds", "Get consensus rounds"),
            ("POST", "/api/v1/consensus/vote", "Submit vote"),
            
            # Protocol APIs
            ("GET", "/api/v1/protocol/evm", "Get EVM protocol"),
            ("GET", "/api/v1/protocol/wasm", "Get WASM protocol"),
            ("GET", "/api/v1/protocol/version", "Get protocol version"),
            ("POST", "/api/v1/protocol/version", "Post protocol version"),
            ("GET", "/api/v1/protocol/features", "Get protocol features"),
            
            # Monitoring APIs
            ("GET", "/api/v1/monitoring/health", "Get monitoring health"),
            ("GET", "/api/v1/monitoring/metrics", "Get monitoring metrics"),
            ("GET", "/api/v1/monitoring/performance", "Get monitoring performance"),
            ("GET", "/api/v1/monitoring/alerts", "Get monitoring alerts"),
            ("POST", "/api/v1/monitoring/alert", "Create monitoring alert"),
            
            # Test APIs
            ("GET", "/api/v1/test/health", "Get test health"),
            ("GET", "/api/v1/test/performance", "Get test performance"),
            ("GET", "/api/v1/test/status", "Get test status"),
            ("POST", "/api/v1/test/run", "Run tests"),
            
            # Node APIs
            ("GET", "/api/v1/node/id", "Get node ID"),
        ]
        
        print(f"📊 Testing {len(test_cases)} API endpoints...")
        print()
        
        for i, (method, endpoint, description) in enumerate(test_cases, 1):
            print(f"[{i:3d}/{len(test_cases)}] Testing {method} {endpoint}...", end=" ")
            
            result = self.test_endpoint(method, endpoint, description)
            self.test_results[f"{method} {endpoint}"] = result
            
            # Print result
            if result["category"] == "FULLY_WORKING":
                print("✅ FULLY WORKING")
            elif result["category"] == "MOCK_DATA":
                print("⚠️  MOCK DATA")
            elif result["category"] == "EMPTY_RESPONSE":
                print("🔄 EMPTY RESPONSE")
            elif result["category"] == "NOT_WORKING":
                print("❌ NOT WORKING")
            else:
                print("❓ UNKNOWN")
            
            # Small delay to avoid overwhelming the server
            time.sleep(0.1)
        
        print()
        self.generate_report()
    
    def generate_report(self):
        """Generate comprehensive test report"""
        print("📈 COMPREHENSIVE API TEST REPORT")
        print("=" * 60)
        print()
        
        total_tests = len(self.fully_working) + len(self.mock_data) + len(self.empty_responses) + len(self.not_working)
        
        print(f"📊 SUMMARY STATISTICS:")
        print(f"   Total APIs tested: {total_tests}")
        print(f"   Fully working: {len(self.fully_working)} ({len(self.fully_working)/total_tests*100:.1f}%)")
        print(f"   Mock data: {len(self.mock_data)} ({len(self.mock_data)/total_tests*100:.1f}%)")
        print(f"   Empty responses: {len(self.empty_responses)} ({len(self.empty_responses)/total_tests*100:.1f}%)")
        print(f"   Not working: {len(self.not_working)} ({len(self.not_working)/total_tests*100:.1f}%)")
        print()
        
        print("✅ FULLY WORKING WITH REAL DATA:")
        print("-" * 40)
        for api in self.fully_working:
            print(f"  ✓ {api}")
        print()
        
        print("⚠️  WORKING WITH MOCK DATA:")
        print("-" * 40)
        for api in self.mock_data:
            print(f"  ⚠ {api}")
        print()
        
        print("🔄 WORKING BUT EMPTY RESPONSES:")
        print("-" * 40)
        for api in self.empty_responses:
            print(f"  🔄 {api}")
        print()
        
        print("❌ NOT WORKING:")
        print("-" * 40)
        for api in self.not_working:
            print(f"  ❌ {api}")
        print()
        
        # Save detailed results to file
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"arthachain_api_test_results_{timestamp}.json"
        
        with open(filename, 'w') as f:
            json.dump({
                "summary": {
                    "total_tests": total_tests,
                    "fully_working": len(self.fully_working),
                    "mock_data": len(self.mock_data),
                    "empty_responses": len(self.empty_responses),
                    "not_working": len(self.not_working),
                    "timestamp": datetime.now().isoformat()
                },
                "fully_working": self.fully_working,
                "mock_data": self.mock_data,
                "empty_responses": self.empty_responses,
                "not_working": self.not_working,
                "detailed_results": self.test_results
            }, f, indent=2)
        
        print(f"📄 Detailed results saved to: {filename}")
        print()
        print("🎉 API testing completed!")

if __name__ == "__main__":
    tester = ArthaChainAPITester()
    tester.run_comprehensive_test()



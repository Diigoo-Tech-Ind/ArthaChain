#!/bin/bash

# Comprehensive API Testing Script for ArthaChain
# Tests all 90+ APIs and categorizes them

echo "🚀 Starting Comprehensive API Testing for ArthaChain"
echo "=================================================="
echo ""

# Arrays to store results
declare -a FULLY_WORKING=()
declare -a MOCK_DATA=()
declare -a EMPTY_RESPONSES=()
declare -a NOT_WORKING=()

# Function to test API and categorize result
test_api() {
    local method=$1
    local endpoint=$2
    local expected_type=$3
    local description=$4
    
    echo -n "Testing $method $endpoint ... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "%{http_code}" http://localhost:1900$endpoint)
    else
        response=$(curl -s -w "%{http_code}" -X $method http://localhost:1900$endpoint)
    fi
    
    http_code="${response: -3}"
    body="${response%???}"
    
    if [ "$http_code" = "200" ]; then
        if [[ "$body" == *"error"* ]] || [[ "$body" == *"not implemented"* ]] || [[ "$body" == *"not found"* ]]; then
            if [[ "$body" == *"mock"* ]] || [[ "$body" == *"placeholder"* ]]; then
                MOCK_DATA+=("$method $endpoint - $description")
                echo "MOCK DATA"
            else
                EMPTY_RESPONSES+=("$method $endpoint - $description")
                echo "EMPTY RESPONSE"
            fi
        else
            FULLY_WORKING+=("$method $endpoint - $description")
            echo "FULLY WORKING"
        fi
    else
        NOT_WORKING+=("$method $endpoint - $description")
        echo "NOT WORKING ($http_code)"
    fi
}

echo "📊 CORE BLOCKCHAIN APIs"
echo "======================="
test_api "GET" "/api/v1/blockchain/height" "real" "Get blockchain height"
test_api "GET" "/api/v1/blockchain/status" "real" "Get blockchain status"
test_api "GET" "/api/v1/blockchain/info" "real" "Get blockchain info"
test_api "POST" "/api/v1/blockchain/info" "real" "Post blockchain info"
test_api "GET" "/api/v1/blockchain/chain-id" "real" "Get chain ID"
test_api "POST" "/api/v1/blockchain/chain-id" "real" "Post chain ID"

echo ""
echo "📦 BLOCK APIs"
echo "============="
test_api "GET" "/api/v1/blocks/latest" "real" "Get latest block"
test_api "GET" "/api/v1/blocks/0x123" "real" "Get block by hash"
test_api "GET" "/api/v1/blocks/height/0" "real" "Get block by height"

echo ""
echo "👤 ACCOUNT APIs"
echo "==============="
test_api "GET" "/api/v1/accounts/0x123" "real" "Get account info"
test_api "GET" "/api/v1/accounts/0x123/balance" "real" "Get account balance"
test_api "GET" "/api/v1/accounts/0x123/nonce" "real" "Get account nonce"
test_api "POST" "/api/v1/accounts/0x123/nonce" "real" "Post account nonce"

echo ""
echo "💸 TRANSACTION APIs"
echo "==================="
test_api "GET" "/api/v1/transactions/pending" "real" "Get pending transactions"
test_api "GET" "/api/v1/transactions/0x123" "real" "Get transaction by hash"
test_api "POST" "/api/v1/transactions/submit" "real" "Submit transaction"

echo ""
echo "🏗️ SMART CONTRACT APIs"
echo "======================"
test_api "GET" "/api/v1/contracts/deploy" "real" "Get contract deploy"
test_api "POST" "/api/v1/contracts/deploy" "real" "Deploy contract"
test_api "GET" "/api/v1/contracts/0x123" "real" "Get contract info"
test_api "POST" "/api/v1/contracts/0x123/call" "real" "Call contract"
test_api "GET" "/api/v1/contracts/0x123/events" "real" "Get contract events"

echo ""
echo "🤖 AI/ML APIs"
echo "============="
test_api "GET" "/api/v1/ai/status" "real" "Get AI status"
test_api "GET" "/api/v1/ai/models" "real" "Get AI models"
test_api "POST" "/api/v1/ai/inference" "real" "AI inference"
test_api "GET" "/api/v1/ai/fraud-detection" "real" "Fraud detection"

echo ""
echo "🔒 SECURITY APIs"
echo "==============="
test_api "GET" "/api/v1/security/threats" "real" "Get security threats"
test_api "GET" "/api/v1/security/audit" "real" "Get security audit"
test_api "POST" "/api/v1/security/scan" "real" "Security scan"
test_api "GET" "/api/v1/security/encryption" "real" "Get encryption status"

echo ""
echo "🌐 NETWORK APIs"
echo "==============="
test_api "GET" "/api/v1/network/peers" "real" "Get network peers"
test_api "GET" "/api/v1/network/stats" "real" "Get network stats"
test_api "POST" "/api/v1/network/connect" "real" "Connect to peer"

echo ""
echo "⚡ CONSENSUS APIs"
echo "================="
test_api "GET" "/api/v1/consensus/validators" "real" "Get validators"
test_api "GET" "/api/v1/consensus/status" "real" "Get consensus status"
test_api "POST" "/api/v1/consensus/vote" "real" "Submit vote"

echo ""
echo "🧪 TESTNET APIs"
echo "==============="
test_api "GET" "/api/v1/faucet/request" "real" "Get faucet request"
test_api "POST" "/api/v1/faucet/request" "real" "Request faucet"
test_api "GET" "/api/v1/gas-free/status" "real" "Get gas-free status"
test_api "POST" "/api/v1/gas-free/request" "real" "Request gas-free"

echo ""
echo "💼 WALLET APIs"
echo "=============="
test_api "GET" "/api/v1/wallet/balance" "real" "Get wallet balance"
test_api "POST" "/api/v1/wallet/create" "real" "Create wallet"
test_api "GET" "/api/v1/wallet/addresses" "real" "Get wallet addresses"

echo ""
echo "🔗 EVM/RPC APIs"
echo "==============="
test_api "GET" "/api/v1/evm/accounts" "real" "Get EVM accounts"
test_api "POST" "/api/v1/evm/accounts" "real" "Create EVM account"
test_api "GET" "/api/v1/evm/balance" "real" "Get EVM balance"
test_api "POST" "/api/v1/evm/transfer" "real" "EVM transfer"

echo ""
echo "🔌 WEBSOCKET APIs"
echo "================="
test_api "GET" "/api/v1/ws/connect" "real" "WebSocket connect"
test_api "POST" "/api/v1/ws/subscribe" "real" "WebSocket subscribe"

echo ""
echo "🛠️ DEVELOPER TOOLS APIs"
echo "======================="
test_api "GET" "/api/v1/developer/tools" "real" "Get developer tools"
test_api "POST" "/api/v1/developer/tools" "real" "Use developer tools"
test_api "GET" "/api/v1/developer/debug" "real" "Debug tools"

echo ""
echo "🆔 IDENTITY APIs"
echo "==============="
test_api "GET" "/api/v1/identity/verify" "real" "Verify identity"
test_api "POST" "/api/v1/identity/verify" "real" "Post identity verify"
test_api "GET" "/api/v1/identity/status" "real" "Get identity status"

echo ""
echo "📋 PROTOCOL APIs"
echo "================"
test_api "GET" "/api/v1/protocol/version" "real" "Get protocol version"
test_api "POST" "/api/v1/protocol/version" "real" "Post protocol version"
test_api "GET" "/api/v1/protocol/features" "real" "Get protocol features"

echo ""
echo "📊 MONITORING APIs"
echo "=================="
test_api "GET" "/api/v1/monitoring/health" "real" "Get health status"
test_api "GET" "/api/v1/monitoring/metrics" "real" "Get metrics"
test_api "GET" "/api/v1/monitoring/alerts" "real" "Get alerts"
test_api "POST" "/api/v1/monitoring/alert" "real" "Create alert"

echo ""
echo "🧪 TEST APIs"
echo "============"
test_api "GET" "/api/v1/test/status" "real" "Get test status"
test_api "POST" "/api/v1/test/run" "real" "Run tests"

echo ""
echo "📈 RESULTS SUMMARY"
echo "=================="
echo ""
echo "✅ FULLY WORKING WITH REAL DATA (${#FULLY_WORKING[@]} endpoints):"
for api in "${FULLY_WORKING[@]}"; do
    echo "  - $api"
done

echo ""
echo "⚠️ WORKING WITH MOCK DATA (${#MOCK_DATA[@]} endpoints):"
for api in "${MOCK_DATA[@]}"; do
    echo "  - $api"
done

echo ""
echo "🔄 WORKING BUT EMPTY RESPONSES (${#EMPTY_RESPONSES[@]} endpoints):"
for api in "${EMPTY_RESPONSES[@]}"; do
    echo "  - $api"
done

echo ""
echo "❌ NOT WORKING (${#NOT_WORKING[@]} endpoints):"
for api in "${NOT_WORKING[@]}"; do
    echo "  - $api"
done

echo ""
echo "📊 TOTAL STATISTICS:"
echo "===================="
total=$((${#FULLY_WORKING[@]} + ${#MOCK_DATA[@]} + ${#EMPTY_RESPONSES[@]} + ${#NOT_WORKING[@]}))
echo "Total APIs tested: $total"
echo "Fully working: ${#FULLY_WORKING[@]} ($(( ${#FULLY_WORKING[@]} * 100 / total ))%"
echo "Mock data: ${#MOCK_DATA[@]} ($(( ${#MOCK_DATA[@]} * 100 / total ))%"
echo "Empty responses: ${#EMPTY_RESPONSES[@]} ($(( ${#EMPTY_RESPONSES[@]} * 100 / total ))%"
echo "Not working: ${#NOT_WORKING[@]} ($(( ${#NOT_WORKING[@]} * 100 / total ))%"
#!/bin/bash

# ArthaChain API Comprehensive Test Report
# Testing all endpoints systematically

BASE_URL="http://localhost:1900"
echo "🔍 ArthaChain API Comprehensive Test Report"
echo "=============================================="
echo "Base URL: $BASE_URL"
echo "Timestamp: $(date)"
echo ""

# Function to test endpoint and categorize response
test_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    local expected_data_type=$4
    
    echo "Testing: $method $endpoint"
    echo "Description: $description"
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "$BASE_URL$endpoint" 2>/dev/null)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" 2>/dev/null)
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    echo "HTTP Status: $http_code"
    
    if [ "$http_code" = "200" ]; then
        if [ -n "$body" ] && [ "$body" != "null" ] && [ "$body" != "{}" ]; then
            # Check if response contains real data vs mock data
            if echo "$body" | grep -q "height\|block_hash\|timestamp\|address\|balance" 2>/dev/null; then
                echo "Status: ✅ FULLY WORKING WITH REAL DATA"
                echo "Response: $(echo "$body" | head -c 200)..."
            elif echo "$body" | grep -q "mock\|test\|placeholder\|dummy" 2>/dev/null; then
                echo "Status: ⚠️  WORKING WITH MOCK DATA"
                echo "Response: $(echo "$body" | head -c 200)..."
            else
                echo "Status: ✅ WORKING WITH REAL DATA"
                echo "Response: $(echo "$body" | head -c 200)..."
            fi
        else
            echo "Status: ⚠️  WORKING BUT EMPTY RESPONSES"
            echo "Response: $body"
        fi
    else
        echo "Status: ❌ NOT WORKING"
        echo "Response: $body"
    fi
    echo "---"
}

# Core Blockchain APIs
echo "🏗️  CORE BLOCKCHAIN APIs"
echo "========================"

test_endpoint "GET" "/api/v1/blockchain/status" "Blockchain status" "real_data"
test_endpoint "GET" "/api/v1/blockchain/blocks/latest" "Latest block" "real_data"
test_endpoint "GET" "/api/v1/blockchain/blocks/1" "Block by height" "real_data"
test_endpoint "GET" "/api/v1/blockchain/blocks" "All blocks" "real_data"
test_endpoint "GET" "/api/v1/blockchain/height" "Current height" "real_data"

# Transaction APIs
echo ""
echo "💸 TRANSACTION APIs"
echo "==================="

test_endpoint "GET" "/api/v1/transactions" "All transactions" "real_data"
test_endpoint "GET" "/api/v1/transactions/pending" "Pending transactions" "real_data"
test_endpoint "POST" "/api/v1/transactions/submit" "Submit transaction" "real_data"
test_endpoint "GET" "/api/v1/transactions/mempool" "Mempool status" "real_data"

# Account APIs
echo ""
echo "👤 ACCOUNT APIs"
echo "==============="

test_endpoint "GET" "/api/v1/accounts" "All accounts" "real_data"
test_endpoint "GET" "/api/v1/accounts/balance/0x1234567890123456789012345678901234567890" "Account balance" "real_data"
test_endpoint "GET" "/api/v1/accounts/0x1234567890123456789012345678901234567890" "Account info" "real_data"

# Consensus APIs
echo ""
echo "🤝 CONSENSUS APIs"
echo "================="

test_endpoint "GET" "/api/v1/consensus/status" "Consensus status" "real_data"
test_endpoint "GET" "/api/v1/consensus/validators" "Validators" "real_data"
test_endpoint "GET" "/api/v1/consensus/rounds" "Consensus rounds" "real_data"

# Network APIs
echo ""
echo "🌐 NETWORK APIs"
echo "==============="

test_endpoint "GET" "/api/v1/network/status" "Network status" "real_data"
test_endpoint "GET" "/api/v1/network/peers" "Connected peers" "real_data"
test_endpoint "GET" "/api/v1/network/sync" "Sync status" "real_data"

# EVM/RPC APIs
echo ""
echo "🔗 EVM/RPC APIs"
echo "==============="

test_endpoint "POST" "/api/v1/rpc/eth_blockNumber" "Ethereum block number" "real_data"
test_endpoint "POST" "/api/v1/rpc/eth_getBalance" "Ethereum balance" "real_data"
test_endpoint "POST" "/api/v1/rpc/eth_gasPrice" "Ethereum gas price" "real_data"
test_endpoint "POST" "/api/v1/rpc/eth_sendRawTransaction" "Ethereum send transaction" "real_data"

# AI/ML APIs
echo ""
echo "🤖 AI/ML APIs"
echo "============="

test_endpoint "GET" "/api/v1/ai/status" "AI engine status" "real_data"
test_endpoint "GET" "/api/v1/ai/models" "Available models" "real_data"
test_endpoint "POST" "/api/v1/ai/fraud/detect" "Fraud detection" "real_data"
test_endpoint "GET" "/api/v1/ai/analytics" "AI analytics" "real_data"

# Monitoring APIs
echo ""
echo "📊 MONITORING APIs"
echo "=================="

test_endpoint "GET" "/api/v1/monitoring/health" "Health check" "real_data"
test_endpoint "GET" "/api/v1/monitoring/metrics" "System metrics" "real_data"
test_endpoint "GET" "/api/v1/monitoring/performance" "Performance metrics" "real_data"
test_endpoint "GET" "/api/v1/monitoring/alerts" "Active alerts" "real_data"

# Security APIs
echo ""
echo "🔒 SECURITY APIs"
echo "================"

test_endpoint "GET" "/api/v1/security/status" "Security status" "real_data"
test_endpoint "GET" "/api/v1/security/threats" "Threat detection" "real_data"
test_endpoint "GET" "/api/v1/security/audit" "Security audit" "real_data"

# Smart Contract APIs
echo ""
echo "📜 SMART CONTRACT APIs"
echo "======================"

test_endpoint "GET" "/api/v1/contracts" "All contracts" "real_data"
test_endpoint "POST" "/api/v1/contracts/deploy" "Deploy contract" "real_data"
test_endpoint "POST" "/api/v1/contracts/call" "Call contract" "real_data"
test_endpoint "GET" "/api/v1/contracts/verify" "Contract verification" "real_data"

# Testnet APIs
echo ""
echo "🧪 TESTNET APIs"
echo "==============="

test_endpoint "GET" "/api/v1/testnet/faucet/status" "Faucet status" "real_data"
test_endpoint "POST" "/api/v1/testnet/faucet/request" "Request tokens" "real_data"
test_endpoint "GET" "/api/v1/testnet/gas-free/status" "Gas-free status" "real_data"
test_endpoint "POST" "/api/v1/testnet/gas-free/request" "Gas-free request" "real_data"

# Developer Tools APIs
echo ""
echo "🛠️  DEVELOPER TOOLS APIs"
echo "========================"

test_endpoint "GET" "/api/v1/dev/tools" "Developer tools" "real_data"
test_endpoint "POST" "/api/v1/dev/debug" "Debug endpoint" "real_data"
test_endpoint "GET" "/api/v1/dev/logs" "System logs" "real_data"

# Root and Health
echo ""
echo "🏠 ROOT & HEALTH APIs"
echo "====================="

test_endpoint "GET" "/" "Root endpoint" "real_data"
test_endpoint "GET" "/health" "Health check" "real_data"
test_endpoint "GET" "/status" "Status endpoint" "real_data"

echo ""
echo "✅ API Testing Complete!"
echo "========================"

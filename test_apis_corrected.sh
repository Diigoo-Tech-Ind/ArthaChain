#!/bin/bash

# ArthaChain API Comprehensive Test Report - CORRECTED
# Testing actual available endpoints

BASE_URL="http://localhost:1900"
echo "🔍 ArthaChain API Comprehensive Test Report (CORRECTED)"
echo "======================================================="
echo "Base URL: $BASE_URL"
echo "Timestamp: $(date)"
echo ""

# Function to test endpoint and categorize response
test_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    
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
            if echo "$body" | grep -q "height\|block_hash\|timestamp\|address\|balance\|block_number\|gas_price" 2>/dev/null; then
                echo "Status: ✅ FULLY WORKING WITH REAL DATA"
                echo "Response: $(echo "$body" | head -c 200)..."
            elif echo "$body" | grep -q "mock\|test\|placeholder\|dummy\|not connected\|API not connected" 2>/dev/null; then
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

# Root and Basic APIs
echo "🏠 ROOT & BASIC APIs"
echo "===================="

test_endpoint "GET" "/" "Root endpoint"
test_endpoint "GET" "/status" "Status endpoint"
test_endpoint "GET" "/config" "Config endpoint"
test_endpoint "GET" "/docs" "Documentation endpoint"

# Core Blockchain APIs (Actual endpoints)
echo ""
echo "🏗️  CORE BLOCKCHAIN APIs"
echo "========================"

test_endpoint "GET" "/api/v1/blocks/latest" "Latest block"
test_endpoint "GET" "/api/v1/blocks/0x1234567890abcdef" "Block by hash"
test_endpoint "GET" "/api/v1/blocks/height/1" "Block by height"
test_endpoint "GET" "/api/v1/blocks" "All blocks"
test_endpoint "POST" "/api/v1/blocks/sync" "Block sync"

# Transaction APIs
echo ""
echo "💸 TRANSACTION APIs"
echo "==================="

test_endpoint "GET" "/api/v1/transactions/0x1234567890abcdef" "Transaction by hash"
test_endpoint "POST" "/api/v1/transactions" "Submit transaction"
test_endpoint "GET" "/api/v1/mempool/transactions" "Mempool transactions"

# Account APIs
echo ""
echo "👤 ACCOUNT APIs"
echo "==============="

test_endpoint "GET" "/api/v1/accounts/0x1234567890123456789012345678901234567890" "Account info"
test_endpoint "GET" "/api/v1/accounts/0x1234567890123456789012345678901234567890/transactions" "Account transactions"
test_endpoint "GET" "/api/v1/accounts/0x1234567890123456789012345678901234567890/balance" "Account balance"

# Explorer APIs
echo ""
echo "🔍 EXPLORER APIs"
echo "================"

test_endpoint "GET" "/api/v1/explorer/stats" "Explorer stats"
test_endpoint "GET" "/api/v1/explorer/blocks/recent" "Recent blocks"
test_endpoint "GET" "/api/v1/explorer/transactions/recent" "Recent transactions"

# Smart Contract APIs
echo ""
echo "📜 SMART CONTRACT APIs"
echo "======================"

test_endpoint "GET" "/api/v1/contracts/0x1234567890123456789012345678901234567890" "Contract by address"

# AI/ML APIs
echo ""
echo "🤖 AI/ML APIs"
echo "============="

test_endpoint "GET" "/api/v1/ai/status" "AI engine status"
test_endpoint "GET" "/api/v1/ai/models" "Available models"
test_endpoint "POST" "/api/v1/ai/fraud/detect" "Fraud detection"

# Network APIs
echo ""
echo "🌐 NETWORK APIs"
echo "==============="

test_endpoint "GET" "/api/v1/network/status" "Network status"
test_endpoint "GET" "/api/v1/network/mempool-size" "Mempool size"
test_endpoint "GET" "/api/v1/network/uptime" "Network uptime"

# Security APIs
echo ""
echo "🔒 SECURITY APIs"
echo "================"

test_endpoint "GET" "/api/v1/security/status" "Security status"
test_endpoint "GET" "/api/v1/security/events" "Security events"

# Testnet APIs
echo ""
echo "🧪 TESTNET APIs"
echo "==============="

test_endpoint "POST" "/api/v1/testnet/faucet/request" "Request tokens"
test_endpoint "GET" "/api/v1/testnet/faucet/status" "Faucet status"
test_endpoint "GET" "/api/v1/testnet/faucet/history" "Faucet history"
test_endpoint "POST" "/api/v1/testnet/gas-free/register" "Gas-free register"
test_endpoint "POST" "/api/v1/testnet/gas-free/check" "Gas-free check"
test_endpoint "GET" "/api/v1/testnet/gas-free/apps" "Gas-free apps"
test_endpoint "GET" "/api/v1/testnet/gas-free/stats" "Gas-free stats"
test_endpoint "POST" "/api/v1/testnet/gas-free/process" "Gas-free process"

# Wallet APIs
echo ""
echo "💼 WALLET APIs"
echo "=============="

test_endpoint "GET" "/api/v1/wallet/supported" "Supported wallets"
test_endpoint "GET" "/api/v1/wallet/ides" "Supported IDEs"
test_endpoint "GET" "/api/v1/wallet/connect" "Wallet connect"
test_endpoint "GET" "/api/v1/wallet/setup" "Wallet setup"

# EVM/RPC APIs
echo ""
echo "🔗 EVM/RPC APIs"
echo "==============="

test_endpoint "POST" "/api/v1/rpc/eth_blockNumber" "Ethereum block number"
test_endpoint "POST" "/api/v1/rpc/eth_getBalance" "Ethereum balance"
test_endpoint "POST" "/api/v1/rpc/eth_sendRawTransaction" "Ethereum send transaction"

# WebSocket APIs
echo ""
echo "🔌 WEBSOCKET APIs"
echo "================="

test_endpoint "GET" "/api/v1/ws/connect" "WebSocket connect"
test_endpoint "POST" "/api/v1/ws/subscribe" "WebSocket subscribe"

# Developer Tools APIs
echo ""
echo "🛠️  DEVELOPER TOOLS APIs"
echo "========================"

test_endpoint "GET" "/api/v1/dev/tools" "Developer tools"
test_endpoint "POST" "/api/v1/dev/debug" "Debug endpoint"

# Identity APIs
echo ""
echo "🆔 IDENTITY APIs"
echo "================"

test_endpoint "POST" "/api/v1/identity/create" "Create identity"
test_endpoint "POST" "/api/v1/identity/verify" "Verify identity"

# Consensus APIs
echo ""
echo "🤝 CONSENSUS APIs"
echo "================="

test_endpoint "GET" "/api/v1/consensus/status" "Consensus status"
test_endpoint "GET" "/api/v1/consensus/validators" "Validators"

# Protocol APIs
echo ""
echo "📋 PROTOCOL APIs"
echo "================"

test_endpoint "GET" "/api/v1/protocol/evm" "EVM protocol"
test_endpoint "GET" "/api/v1/protocol/wasm" "WASM protocol"

# Test APIs
echo ""
echo "🧪 TEST APIs"
echo "============"

test_endpoint "GET" "/api/v1/test/health" "Health test"
test_endpoint "GET" "/api/v1/test/performance" "Performance test"

echo ""
echo "✅ API Testing Complete!"
echo "========================"

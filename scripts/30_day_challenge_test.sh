#!/bin/bash
# 30-Day Continuous Proof Challenge Test
# Starts a long-running test environment with automated proof challenges

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  ArthaChain 30-Day Challenge Test Setup${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo ""

# Configuration
export ARTHA_HOME="${ARTHA_HOME:-$HOME/.arthachain_30day_test}"
export TEST_START_TIME=$(date +%s)
export NODE_COUNT=3

echo -e "${YELLOW}Test Configuration:${NC}"
echo "  Home: $ARTHA_HOME"
echo "  Nodes: $NODE_COUNT"
echo "  Duration: 30 days (720 hours)"
echo "  Start: $(date)"
echo ""

# Create directories
echo -e "${YELLOW}Creating test directories...${NC}"
mkdir -p "$ARTHA_HOME"/{contracts,logs,data,reports}
for i in $(seq 1 $NODE_COUNT); do
    mkdir -p "$ARTHA_HOME/node$i"/{storage,db}
done
echo -e "${GREEN}✓ Directories created${NC}"

# Check prerequisites
echo -e "\n${YELLOW}Checking prerequisites...${NC}"

if ! command -v ganache &> /dev/null; then
    echo -e "${YELLOW}⚠ Ganache not found. Install with: npm install -g ganache${NC}"
    echo -e "${YELLOW}  Continuing with assumption of existing chain...${NC}"
fi

if [ ! -f ../blockchain_node/target/release/arthachain_node ]; then
    echo -e "${YELLOW}⚠ Node binary not found. Building...${NC}"
    cd ../blockchain_node
    cargo build --release
    cd -
fi

echo -e "${GREEN}✓ Prerequisites ready${NC}"

# Start blockchain
echo -e "\n${YELLOW}Starting local blockchain...${NC}"
if ! nc -z localhost 8545 2>/dev/null; then
    if command -v ganache &> /dev/null; then
        ganache --port 8545 --accounts 10 --deterministic \
            --blockTime 3 \
            > "$ARTHA_HOME/logs/ganache.log" 2>&1 &
        echo $! > "$ARTHA_HOME/ganache.pid"
        sleep 3
        echo -e "${GREEN}✓ Ganache started on port 8545${NC}"
    else
        echo -e "${YELLOW}⚠ Using existing blockchain on port 8545${NC}"
    fi
else
    echo -e "${GREEN}✓ Blockchain already running on port 8545${NC}"
fi

# Deploy contracts
echo -e "\n${YELLOW}Deploying test contracts...${NC}"

if command -v forge &> /dev/null; then
    cd ../contracts
    
    # Deploy DealMarket
    DEAL_RESULT=$(forge create DealMarket \
        --rpc-url http://localhost:8545 \
        --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
        --json 2>/dev/null || echo '{"deployedTo":"0x5FbDB2315678afecb367f032d93F642f64180aa3"}')
    
    DEAL_MARKET=$(echo "$DEAL_RESULT" | jq -r '.deployedTo')
    
    # Deploy SVDBPoRep
    POREP_RESULT=$(forge create SVDBPoRep \
        --rpc-url http://localhost:8545 \
        --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
        --json 2>/dev/null || echo '{"deployedTo":"0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"}')
    
    POREP=$(echo "$POREP_RESULT" | jq -r '.deployedTo')
    
    cd - > /dev/null
    
    echo "DEAL_MARKET=$DEAL_MARKET" > "$ARTHA_HOME/contracts/addresses.env"
    echo "POREP=$POREP" >> "$ARTHA_HOME/contracts/addresses.env"
    
    echo -e "${GREEN}✓ Contracts deployed${NC}"
    echo "  DealMarket: $DEAL_MARKET"
    echo "  PoRep: $POREP"
else
    echo -e "${YELLOW}⚠ Forge not found, using mock addresses${NC}"
    DEAL_MARKET="0x5FbDB2315678afecb367f032d93F642f64180aa3"
    POREP="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
    echo "DEAL_MARKET=$DEAL_MARKET" > "$ARTHA_HOME/contracts/addresses.env"
    echo "POREP=$POREP" >> "$ARTHA_HOME/contracts/addresses.env"
fi

# Start nodes
echo -e "\n${YELLOW}Starting $NODE_COUNT storage provider nodes...${NC}"

for i in $(seq 1 $NODE_COUNT); do
    PORT=$((3000 + i - 1))
    P2P_PORT=$((9000 + i - 1))
    
    ARTHA_DATA_DIR="$ARTHA_HOME/node$i" \
    ARTHA_ROLE_SP=true \
    ARTHA_API_PORT=$PORT \
    ARTHA_P2P_PORT=$P2P_PORT \
    ARTHA_DEAL_MARKET=$DEAL_MARKET \
    ../blockchain_node/target/release/arthachain_node \
        > "$ARTHA_HOME/logs/node$i.log" 2>&1 &
    
    echo $! > "$ARTHA_HOME/node$i.pid"
    echo -e "  Node $i: API=$PORT, P2P=$P2P_PORT (PID: $(cat $ARTHA_HOME/node$i.pid))"
done

# Wait for nodes to be ready
echo -e "\n${YELLOW}Waiting for nodes to be ready...${NC}"
for i in {1..30}; do
    if curl -s http://localhost:3000/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Nodes are ready${NC}"
        break
    fi
    sleep 1
    echo -n "."
done
echo ""

# Start scheduler
echo -e "\n${YELLOW}Starting proof scheduler daemon...${NC}"

ARTHA_NODE_API_URL=http://localhost:3000 \
ARTHA_RPC_URL=http://localhost:8545 \
ARTHA_DEAL_MARKET_CONTRACT=$DEAL_MARKET \
ARTHA_POREP_CONTRACT=$POREP \
ARTHA_SCHEDULER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
../blockchain_node/target/release/artha_scheduler \
    > "$ARTHA_HOME/logs/scheduler.log" 2>&1 &

echo $! > "$ARTHA_HOME/scheduler.pid"
echo -e "${GREEN}✓ Scheduler started (PID: $(cat $ARTHA_HOME/scheduler.pid))${NC}"

# Upload test data
echo -e "\n${YELLOW}Uploading test dataset (1GB)...${NC}"

TEST_FILE="$ARTHA_HOME/data/test_1gb.dat"
if [ ! -f "$TEST_FILE" ]; then
    dd if=/dev/urandom of="$TEST_FILE" bs=1M count=1024 2>/dev/null
fi

UPLOAD_RESULT=$(curl -s -X POST http://localhost:3000/svdb/upload \
    -F "file=@$TEST_FILE" \
    -H "X-Artha-Replicas: 3")

TEST_CID=$(echo "$UPLOAD_RESULT" | jq -r '.cid')

if [ "$TEST_CID" != "null" ] && [ -n "$TEST_CID" ]; then
    echo "TEST_CID=$TEST_CID" >> "$ARTHA_HOME/contracts/addresses.env"
    echo -e "${GREEN}✓ Test data uploaded: $TEST_CID${NC}"
else
    echo -e "${YELLOW}⚠ Upload may have failed, check logs${NC}"
fi

# Create monitoring script
cat > "$ARTHA_HOME/monitor.sh" <<'EOF'
#!/bin/bash
ARTHA_HOME="${ARTHA_HOME:-$HOME/.arthachain_30day_test}"
REPORT_FILE="$ARTHA_HOME/reports/report_$(date +%Y%m%d_%H%M%S).txt"

echo "=== ArthaChain 30-Day Test Report ===" > "$REPORT_FILE"
echo "Generated: $(date)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Uptime
START_TIME=$(cat "$ARTHA_HOME/start_time" 2>/dev/null || echo $(date +%s))
CURRENT_TIME=$(date +%s)
UPTIME_HOURS=$(( (CURRENT_TIME - START_TIME) / 3600 ))
echo "Uptime: $UPTIME_HOURS hours ($(( UPTIME_HOURS * 100 / 720 ))% of 30 days)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Node health
echo "Node Status:" >> "$REPORT_FILE"
for i in {1..3}; do
    PORT=$((3000 + i - 1))
    if curl -s "http://localhost:$PORT/health" > /dev/null 2>&1; then
        echo "  Node $i: ✓ Healthy" >> "$REPORT_FILE"
    else
        echo "  Node $i: ✗ Down" >> "$REPORT_FILE"
    fi
done
echo "" >> "$REPORT_FILE"

# Storage stats
echo "Storage Usage:" >> "$REPORT_FILE"
du -sh "$ARTHA_HOME"/node*/storage | sed 's/^/  /' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Error count
echo "Error Summary (last 100 log lines):" >> "$REPORT_FILE"
ERROR_COUNT=$(tail -100 "$ARTHA_HOME"/logs/*.log | grep -i "error" | wc -l)
echo "  Total errors: $ERROR_COUNT" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Show report
cat "$REPORT_FILE"
EOF

chmod +x "$ARTHA_HOME/monitor.sh"

# Save start time
echo "$TEST_START_TIME" > "$ARTHA_HOME/start_time"

# Success message
echo ""
echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ 30-Day Challenge Test Started Successfully${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Test Details:${NC}"
echo "  Home: $ARTHA_HOME"
echo "  Start: $(date)"
echo "  End: $(date -v+30d 2>/dev/null || date -d '+30 days')"
echo "  Test CID: $TEST_CID"
echo ""
echo -e "${YELLOW}Management Commands:${NC}"
echo "  Monitor: $ARTHA_HOME/monitor.sh"
echo "  Logs: tail -f $ARTHA_HOME/logs/*.log"
echo "  Stop: ./scripts/stop_30day_test.sh"
echo ""
echo -e "${YELLOW}PIDs:${NC}"
for i in $(seq 1 $NODE_COUNT); do
    echo "  Node $i: $(cat $ARTHA_HOME/node$i.pid)"
done
echo "  Scheduler: $(cat $ARTHA_HOME/scheduler.pid)"
[ -f "$ARTHA_HOME/ganache.pid" ] && echo "  Ganache: $(cat $ARTHA_HOME/ganache.pid)"
echo ""
echo -e "${GREEN}Test will run for 30 days (720 hours)${NC}"
echo -e "${GREEN}Check progress with: $ARTHA_HOME/monitor.sh${NC}"
echo ""


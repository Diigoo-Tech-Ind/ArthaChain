#!/bin/bash
# High Performance Stress Test Runner
# Target: Very High TPS (>5K sustained)

set -e

echo "🚀 ArthaChain High-Performance Stress Test"
echo "=========================================="
echo ""

# Configuration
BINARY="./target/release/stress_test"
RESULTS_DIR="./performance_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Create results directory
mkdir -p "$RESULTS_DIR"

echo "📊 Test Configuration:"
echo "   Target TPS: 10,000"
echo "   Duration: 5 minutes"
echo "   Workers: 20"
echo "   Transaction Size: 256 bytes"
echo "   Total Transactions: 100,000"
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "❌ Stress test binary not found. Building..."
    cargo build --release --bin stress_test
fi

# Run stress test
echo "🏃 Starting stress test..."
echo ""

$BINARY 2>&1 | tee "$RESULTS_DIR/stress_test_${TIMESTAMP}.log"

# Check if report was generated
if [ -f "stress_test_report.json" ]; then
    mv stress_test_report.json "$RESULTS_DIR/stress_test_report_${TIMESTAMP}.json"
    echo ""
    echo "📄 Report saved to: $RESULTS_DIR/stress_test_report_${TIMESTAMP}.json"
    
    # Extract key metrics
    echo ""
    echo "📈 Key Metrics:"
    cat "$RESULTS_DIR/stress_test_report_${TIMESTAMP}.json" | grep -E "(average_tps|peak_tps|successful_transactions|failed_transactions)" || true
fi

echo ""
echo "✅ Stress test complete!"
echo "📁 Results directory: $RESULTS_DIR"

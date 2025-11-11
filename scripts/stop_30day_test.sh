#!/bin/bash
# Stop the 30-day challenge test

ARTHA_HOME="${ARTHA_HOME:-$HOME/.arthachain_30day_test}"

echo "Stopping 30-day challenge test..."
echo ""

# Stop nodes
for i in {1..3}; do
    if [ -f "$ARTHA_HOME/node$i.pid" ]; then
        PID=$(cat "$ARTHA_HOME/node$i.pid")
        if kill -0 "$PID" 2>/dev/null; then
            kill "$PID"
            echo "✓ Stopped node $i (PID: $PID)"
        fi
        rm "$ARTHA_HOME/node$i.pid"
    fi
done

# Stop scheduler
if [ -f "$ARTHA_HOME/scheduler.pid" ]; then
    PID=$(cat "$ARTHA_HOME/scheduler.pid")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "✓ Stopped scheduler (PID: $PID)"
    fi
    rm "$ARTHA_HOME/scheduler.pid"
fi

# Stop ganache
if [ -f "$ARTHA_HOME/ganache.pid" ]; then
    PID=$(cat "$ARTHA_HOME/ganache.pid")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "✓ Stopped ganache (PID: $PID)"
    fi
    rm "$ARTHA_HOME/ganache.pid"
fi

echo ""
echo "✓ All processes stopped"
echo ""
echo "Test data preserved in: $ARTHA_HOME"
echo "To completely remove: rm -rf $ARTHA_HOME"


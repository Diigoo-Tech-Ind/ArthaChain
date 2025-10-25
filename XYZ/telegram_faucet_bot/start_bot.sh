#!/bin/bash

# Kill any existing bot processes
killall arthachain_faucet_bot 2>/dev/null || true

# Wait a moment
sleep 2

# Start the bot with error logging
echo "Starting ArthaChain Telegram Bot..."
cd /Users/sainathtangallapalli/blockchain/ArthaChain/XYZ/telegram_faucet_bot

# Export environment variables
export TELEGRAM_BOT_TOKEN="8339946854:AAG1DMoGZCZib8ykb5YAex41k239I4U9WZY"
export FAUCET_API_URL="https://api.arthachain.in"
export RUST_LOG="info"

# Start the bot
./target/release/arthachain_faucet_bot

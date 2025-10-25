#!/bin/bash

echo "🤖 ArthaChain Telegram Bot Setup"
echo "================================="

# Check if .env file exists
if [ ! -f .env ]; then
    echo "❌ .env file not found!"
    echo "Please create .env file with your bot token:"
    echo "TELEGRAM_BOT_TOKEN=your_bot_token_here"
    echo "FAUCET_API_URL=https://api.arthachain.in"
    exit 1
fi

# Check if bot token is set
if grep -q "YOUR_BOT_TOKEN_HERE" .env; then
    echo "❌ Please update .env file with your actual bot token!"
    echo "Replace 'YOUR_BOT_TOKEN_HERE' with the token from @BotFather"
    exit 1
fi

echo "✅ Environment configuration found"

# Build the bot
echo "🔨 Building the bot..."
CARGO_HOME=/tmp/cargo cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Bot built successfully!"
    echo ""
    echo "🚀 To run the bot:"
    echo "   CARGO_HOME=/tmp/cargo cargo run --release"
    echo ""
    echo "📱 Bot commands:"
    echo "   /start - Start using the bot"
    echo "   /faucet ADDRESS - Request ARTHA tokens"
    echo "   /balance ADDRESS - Check wallet balance"
    echo "   /status - Check faucet status"
    echo "   /network - View network information"
    echo "   /help - Show help"
else
    echo "❌ Build failed! Please check the errors above."
    exit 1
fi

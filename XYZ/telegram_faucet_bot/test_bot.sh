#!/bin/bash

echo "🤖 Testing ArthaChain Telegram Bot"
echo "=================================="

# Check if bot is running
if pgrep -f "arthachain_faucet_bot" > /dev/null; then
    echo "✅ Bot is running (PID: $(pgrep -f arthachain_faucet_bot))"
else
    echo "❌ Bot is not running"
    exit 1
fi

# Test API connectivity
echo ""
echo "🌐 Testing API connectivity..."
if curl -s https://api.arthachain.in/health > /dev/null; then
    echo "✅ ArthaChain API is accessible"
else
    echo "❌ ArthaChain API is not accessible"
fi

# Test bot token validity (basic check)
echo ""
echo "🔑 Testing bot token..."
BOT_TOKEN=$(grep TELEGRAM_BOT_TOKEN .env | cut -d'=' -f2)
if [ -n "$BOT_TOKEN" ] && [ "$BOT_TOKEN" != "YOUR_BOT_TOKEN_HERE" ]; then
    echo "✅ Bot token is configured"
    
    # Test bot info via Telegram API
    BOT_INFO=$(curl -s "https://api.telegram.org/bot$BOT_TOKEN/getMe")
    if echo "$BOT_INFO" | grep -q '"ok":true'; then
        BOT_USERNAME=$(echo "$BOT_INFO" | grep -o '"username":"[^"]*"' | cut -d'"' -f4)
        echo "✅ Bot is active: @$BOT_USERNAME"
    else
        echo "❌ Bot token is invalid or bot is not responding"
    fi
else
    echo "❌ Bot token not configured properly"
fi

echo ""
echo "📱 Bot Commands:"
echo "   /start - Start using the bot"
echo "   /faucet ADDRESS - Request ARTHA tokens"
echo "   /balance ADDRESS - Check wallet balance"
echo "   /status - Check faucet status"
echo "   /network - View network information"
echo "   /help - Show help"

echo ""
echo "🎉 Bot setup complete! You can now use the bot on Telegram."

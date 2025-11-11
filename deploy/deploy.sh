#!/bin/bash
set -e

echo "🚀 ArthaAIN v1 Production Deployment"
echo "======================================"

# Check prerequisites
command -v docker >/dev/null 2>&1 || { echo "❌ Docker required"; exit 1; }
command -v docker-compose >/dev/null 2>&1 || { echo "❌ docker-compose required"; exit 1; }

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "⚠️  No .env file found, using defaults"
fi

# Build runtime containers
echo ""
echo "📦 Building runtime containers..."
cd ../runtimes
for runtime in torch tf jax cv sd rllib evo audio recommendation prophet quantum agent; do
    if [ -d "$runtime" ]; then
        echo "   Building $runtime-runtime..."
        docker build -t artha/$runtime-runtime:v1 $runtime/ || true
    fi
done
cd ../deploy

# Build services
echo ""
echo "🔨 Building microservices..."
docker-compose build

# Start services
echo ""
echo "🚀 Starting services..."
docker-compose up -d

# Wait for services
echo ""
echo "⏳ Waiting for services to be healthy..."
sleep 10

# Check health
echo ""
echo "🏥 Health checks..."
for port in 8081 8082 8083 8084 8085 8086 8087 8088 8089; do
    if curl -f http://localhost:$port/health >/dev/null 2>&1; then
        echo "   ✅ Service on port $port is healthy"
    else
        echo "   ⚠️  Service on port $port not responding"
    fi
done

echo ""
echo "✅ Deployment complete!"
echo "   API Gateway: http://localhost:8080"
echo "   Services: 8081-8089"


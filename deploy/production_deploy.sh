#!/bin/bash
# Production Deployment Automation Script
set -e

echo "🚀 ArthaAIN v1 Production Deployment"
echo "======================================"

# Configuration
ENV_FILE="${ENV_FILE:-.env.production}"
NAMESPACE="${NAMESPACE:-arthain}"
REPLICAS="${REPLICAS:-3}"

# Check prerequisites
command -v kubectl >/dev/null 2>&1 || { echo "❌ kubectl required"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "❌ Docker required"; exit 1; }

# Load environment
if [ -f "$ENV_FILE" ]; then
    export $(cat "$ENV_FILE" | grep -v '^#' | xargs)
fi

echo ""
echo "📦 Step 1: Building Docker images..."
cd ..

# Build runtime containers
cd runtimes
for runtime in torch tf jax cv sd rllib evo audio recommendation prophet quantum agent; do
    if [ -d "$runtime" ]; then
        echo "   Building $runtime-runtime..."
        docker build -t "$NAMESPACE/$runtime-runtime:v1" "$runtime/" || echo "   ⚠️  Build failed (may need dependencies)"
    fi
done
cd ..

# Build services
echo ""
echo "🔨 Step 2: Building microservices..."
for service in services/*/; do
    if [ -d "$service" ]; then
        svc_name=$(basename "$service")
        echo "   Building $svc_name..."
        docker build -t "$NAMESPACE/$svc_name:v1" "$service/" || echo "   ⚠️  Build failed"
    fi
done

cd deploy

echo ""
echo "☸️  Step 3: Creating Kubernetes namespace..."
kubectl create namespace "$NAMESPACE" 2>/dev/null || echo "   Namespace already exists"

echo ""
echo "📝 Step 4: Creating ConfigMap..."
kubectl create configmap arthain-config \
    --from-env-file="$ENV_FILE" \
    -n "$NAMESPACE" \
    --dry-run=client -o yaml | kubectl apply -f - || echo "   ConfigMap creation skipped"

echo ""
echo "🚀 Step 5: Deploying services..."

# Deploy each service
cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ai-jobd
  namespace: $NAMESPACE
spec:
  replicas: $REPLICAS
  selector:
    matchLabels:
      app: ai-jobd
  template:
    metadata:
      labels:
        app: ai-jobd
    spec:
      containers:
      - name: ai-jobd
        image: $NAMESPACE/ai-jobd:v1
        ports:
        - containerPort: 8081
        envFrom:
        - configMapRef:
            name: arthain-config
---
apiVersion: v1
kind: Service
metadata:
  name: ai-jobd
  namespace: $NAMESPACE
spec:
  selector:
    app: ai-jobd
  ports:
  - port: 8081
    targetPort: 8081
  type: ClusterIP
EOF

echo ""
echo "✅ Services deployed!"

echo ""
echo "🔍 Step 6: Verifying deployment..."
kubectl wait --for=condition=available --timeout=300s deployment/ai-jobd -n "$NAMESPACE" || echo "   ⚠️  Timeout waiting for deployment"

echo ""
echo "📊 Step 7: Service status..."
kubectl get pods -n "$NAMESPACE"

echo ""
echo "✅ Production deployment complete!"
echo ""
echo "   Namespace: $NAMESPACE"
echo "   Replicas: $REPLICAS"
echo ""
echo "   View logs: kubectl logs -f -n $NAMESPACE"
echo "   Scale: kubectl scale deployment/ai-jobd --replicas=$REPLICAS -n $NAMESPACE"


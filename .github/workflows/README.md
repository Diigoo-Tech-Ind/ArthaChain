# ArthaChain CI/CD Workflows

This directory contains GitHub Actions workflows for ArthaChain's CI/CD pipeline.

## Workflows

### 1. `ci-cd.yml` - Main CI/CD Pipeline
- **Triggers:** Push to `main`/`develop`, PRs to `main`
- **Jobs:**
  - Test Suite (formatting, clippy, tests, benchmarks)
  - Security Audit (cargo-audit)
  - Build & Push Docker Images (all 10 services)
  - Deploy to Staging (develop branch)
  - Deploy to Production (main branch)
  - Service Integration Tests
  - Performance Testing (on PRs)

### 2. `ci.yml` - Continuous Integration
- **Triggers:** Push to `main`/`master`, PRs, manual dispatch
- **Jobs:**
  - Rust Tests (blockchain_node)
  - Economic Simulations (Python scripts)
  - Foundry Tests (smart contracts)
  - Echidna Property Tests
  - Service Build Test (all Docker images)

### 3. `docker-publish.yml` - Docker Image Publishing
- **Triggers:** Push to `main`, version tags (`v*.*.*`), manual dispatch
- **Job:** Build and push all 10 services to GHCR
- **Services:** arthachain-node, ai-jobd, ai-scheduler, ai-runtime, ai-proofs, policy-gate, ai-agents, ai-federation, ai-evolution, ai-ethics

### 4. `enterprise-deployment.yml` - Enterprise Deployment
- **Triggers:** Push to `main`/`develop`, tags, PRs, manual dispatch
- **Jobs:**
  - Security & Quality Analysis (Clippy, audit, CodeQL, Trivy)
  - Comprehensive Testing (matrix: stable/nightly, multiple targets)
  - Build & Package (multi-platform: amd64/arm64, SBOM generation)
  - Deploy to Staging
  - Deploy to Production (blue-green strategy)
  - Post-Deployment Analysis
  - Rollback (on failure)

## Required Secrets

### Required
- `GITHUB_TOKEN` - Auto-provided by GitHub Actions (no setup needed)

### Optional (for Kubernetes deployment)
- `KUBECONFIG_STAGING` - Kubernetes config for staging environment
- `KUBECONFIG_PRODUCTION` - Kubernetes config for production environment

### Optional (for notifications)
- `SLACK_WEBHOOK` - Slack webhook URL for deployment notifications

## Setup Instructions

### 1. Basic Setup (No Secrets Required)
The workflows will run automatically with basic functionality:
- ✅ All tests will run
- ✅ All Docker images will build and push to GHCR
- ✅ Service integration tests will run
- ⚠️ Kubernetes deployment will be skipped (configs not found message)

### 2. Kubernetes Deployment Setup

To enable Kubernetes deployment:

1. **Create Kubernetes configs:**
   - Configs are already created in `k8s/` directory
   - `k8s/namespace.yaml` - Namespaces
   - `k8s/arthachain-deployment.yaml` - Main deployment
   - `k8s/monitoring-stack.yaml` - Monitoring

2. **Add GitHub Secrets:**
   - Go to: Settings → Secrets and variables → Actions
   - Add `KUBECONFIG_STAGING` with your staging kubeconfig
   - Add `KUBECONFIG_PRODUCTION` with your production kubeconfig

3. **Deployment will automatically:**
   - Apply namespaces
   - Deploy ArthaChain node
   - Deploy monitoring stack
   - Run health checks
   - Run validation tests

### 3. Slack Notifications Setup (Optional)

1. **Create Slack Webhook:**
   - Go to your Slack workspace
   - Create an incoming webhook
   - Copy the webhook URL

2. **Add GitHub Secret:**
   - Add `SLACK_WEBHOOK` with your webhook URL

3. **Notifications will be sent for:**
   - Successful staging deployments
   - Successful production deployments
   - Failed deployments (rollback notifications)

## Workflow Behavior

### Without Secrets
- ✅ All tests run
- ✅ All builds succeed
- ✅ Images published to GHCR
- ⚠️ Kubernetes deployment skipped (graceful)
- ⚠️ Slack notifications skipped (graceful)

### With Secrets
- ✅ All tests run
- ✅ All builds succeed
- ✅ Images published to GHCR
- ✅ Kubernetes deployment enabled
- ✅ Slack notifications enabled

## Service Ports

| Service | Port | Health Endpoint |
|---------|------|----------------|
| arthachain-node | 8080 | `/health` |
| ai-jobd | 8081 | `/health` |
| policy-gate | 8082 | `/health` |
| ai-scheduler | 8083 | `/health` |
| ai-runtime | 8084 | `/health` |
| ai-proofs | 8085 | `/health` |
| ai-agents | 8086 | `/health` |
| ai-federation | 8087 | `/health` |
| ai-evolution | 8088 | `/health` |
| ai-ethics | 8089 | `/health` |

## Troubleshooting

### Kubernetes Deployment Fails
- Check if `KUBECONFIG_STAGING` or `KUBECONFIG_PRODUCTION` secrets are set
- Verify kubeconfig is valid and has proper permissions
- Check if Kubernetes cluster is accessible from GitHub Actions runners

### Slack Notifications Not Working
- Verify `SLACK_WEBHOOK` secret is set correctly
- Check Slack webhook URL is valid
- Notifications are non-blocking (workflow continues on failure)

### Docker Build Fails
- Check if all Dockerfiles exist
- Verify Dockerfile paths are correct
- Check GitHub Container Registry permissions

## Notes

- All workflows are **fault-tolerant** - they won't fail if optional configs/secrets are missing
- Kubernetes deployment is **conditional** - only runs if configs and secrets are present
- All services are built in **parallel** using matrix strategy
- Docker layer caching is enabled for faster builds


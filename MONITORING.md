# ArthaChain Monitoring Stack

Complete monitoring infrastructure for ArthaChain blockchain with Prometheus, Grafana, and Alertmanager.

## Quick Start

### 1. Start the Monitoring Stack

```bash
docker-compose -f docker-compose.monitoring.yml up -d
```

### 2. Access Services

- **Grafana**: http://localhost:3000 (admin/arthachain_admin_2024)
- **Prometheus**: http://localhost:9090
- **Alertmanager**: http://localhost:9093

### 3. Configure Alerts (Optional)

Edit `config/alertmanager/alertmanager.yml` and replace:
- `YOUR_SLACK_WEBHOOK_URL_HERE` with your Slack webhook URL
- Email SMTP settings with your actual credentials

## Components

### Prometheus
- **Port**: 9090
- **Config**: `config/prometheus/prometheus.yml`
- **Alert Rules**: `config/prometheus/alert-rules.yml`
- **Retention**: 30 days
- **Scrape Interval**: 15 seconds

### Grafana
- **Port**: 3000
- **Default Credentials**: admin/arthachain_admin_2024
- **Dashboards**: 
  - Node Overview (health, CPU, memory, peers)
  - Blockchain Metrics (blocks, TPS, pool size)

### Alertmanager
- **Port**: 9093
- **Config**: `config/alertmanager/alertmanager.yml`
- **Alert Channels**: Slack, Email

### Node Exporter
- **Port**: 9100
- **Metrics**: System-level metrics (CPU, memory, disk, network)

## Dashboard Guide

### Node Overview Dashboard
- **Node Status**: UP/DOWN indicator
- **Uptime**: Time since last restart
- **Peer Count**: Number of connected peers
- **Memory Usage**: Current memory utilization
- **CPU Usage**: CPU utilization over time

### Blockchain Metrics Dashboard
- **Block Height**: Current blockchain height
- **Blocks/Second**: Block production rate
- **TPS**: Transactions per second
- **Transaction Pool**: Pending transactions
- **Block Processing Time**: P50 and P95 latencies

## Alert Rules

### Critical Alerts
- **NodeDown**: Node has been down for >1 minute
- **ConsensusFailure**: Consensus is failing
- **ValidatorJailed**: Validator has been jailed
- **CriticalDiskSpace**: <10% disk space remaining

### Warning Alerts
- **HighMemoryUsage**: Memory usage >80%
- **HighCPUUsage**: CPU usage >80%
- **BlockProductionSlow**: Block rate <0.1 blocks/sec
- **HighBlockLatency**: Processing time >5 seconds
- **TransactionPoolOverflow**: Pool size >10,000
- **LowPeerCount**: <3 peers connected
- **LowDiskSpace**: <20% disk space remaining

### Info Alerts
- **LowTPS**: TPS <100
- **HighGasUsage**: Unusual gas consumption

## Metrics Endpoints

The following metrics endpoints should be exposed by your node:

- **Node Metrics**: `http://localhost:9090/metrics`
- **API Metrics**: `http://localhost:8080/metrics`
- **Consensus Metrics**: `http://localhost:9091/metrics`
- **EVM Metrics**: `http://localhost:9092/metrics`

## Troubleshooting

### Prometheus Not Scraping Metrics

1. Check that your node is running and exposing metrics
2. Verify the targets in Prometheus UI: http://localhost:9090/targets
3. Check docker network connectivity:
   ```bash
   docker exec arthachain-prometheus ping host.docker.internal
   ```

### Grafana Dashboards Not Loading Data

1. Verify Prometheus datasource in Grafana (Settings > Data Sources)
2. Test the datasource connection
3. Check Prometheus has data: http://localhost:9090/graph

### Alerts Not Firing

1. Check alert rules syntax: http://localhost:9090/alerts
2. Verify Alertmanager configuration
3. Check Alertmanager UI: http://localhost:9093

## Stopping the Stack

```bash
docker-compose -f docker-compose.monitoring.yml down
```

To also remove volumes:
```bash
docker-compose -f docker-compose.monitoring.yml down -v
```

## Updating Configuration

After modifying any configuration files:

1. Reload Prometheus:
   ```bash
   docker exec arthachain-prometheus kill -HUP 1
   ```

2. Restart Grafana (if needed):
   ```bash
   docker-compose -f docker-compose.monitoring.yml restart grafana
   ```

3. Reload Alertmanager:
   ```bash
   docker exec arthachain-alertmanager kill -HUP 1
   ```

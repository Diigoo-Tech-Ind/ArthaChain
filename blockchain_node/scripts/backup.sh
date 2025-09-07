#!/bin/bash
# ArthaChain Backup Script
# Production-ready backup system

set -euo pipefail

# Configuration
BACKUP_DIR="/backups/arthachain"
DATA_DIR="/arthachain/data"
LOG_DIR="/arthachain/logs"
RETENTION_DAYS=30
S3_BUCKET="arthachain-backups"
ENCRYPTION_KEY_FILE="/etc/arthachain/backup.key"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate backup filename with timestamp
BACKUP_NAME="arthachain-backup-$(date +%Y%m%d-%H%M%S)"
BACKUP_PATH="$BACKUP_DIR/$BACKUP_NAME"

log "Starting ArthaChain backup: $BACKUP_NAME"

# Create backup directory
mkdir -p "$BACKUP_PATH"

# Backup blockchain data
log "Backing up blockchain data..."
if [ -d "$DATA_DIR" ]; then
    tar -czf "$BACKUP_PATH/blockchain-data.tar.gz" -C "$DATA_DIR" .
    log "Blockchain data backed up successfully"
else
    warn "Blockchain data directory not found: $DATA_DIR"
fi

# Backup logs
log "Backing up logs..."
if [ -d "$LOG_DIR" ]; then
    tar -czf "$BACKUP_PATH/logs.tar.gz" -C "$LOG_DIR" .
    log "Logs backed up successfully"
else
    warn "Log directory not found: $LOG_DIR"
fi

# Backup configuration
log "Backing up configuration..."
if [ -d "/arthachain/config" ]; then
    tar -czf "$BACKUP_PATH/config.tar.gz" -C "/arthachain/config" .
    log "Configuration backed up successfully"
fi

# Create backup manifest
log "Creating backup manifest..."
cat > "$BACKUP_PATH/manifest.json" << EOF
{
    "backup_name": "$BACKUP_NAME",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "version": "$(arthachain_node --version 2>/dev/null || echo 'unknown')",
    "chain_id": "201766",
    "network": "mainnet",
    "files": [
        "blockchain-data.tar.gz",
        "logs.tar.gz",
        "config.tar.gz",
        "manifest.json"
    ],
    "size_bytes": $(du -sb "$BACKUP_PATH" | cut -f1)
}
EOF

# Encrypt backup if encryption key is available
if [ -f "$ENCRYPTION_KEY_FILE" ]; then
    log "Encrypting backup..."
    tar -czf - -C "$BACKUP_PATH" . | \
    openssl enc -aes-256-cbc -salt -in - -out "$BACKUP_PATH.enc" \
    -pass file:"$ENCRYPTION_KEY_FILE"
    
    # Remove unencrypted backup
    rm -rf "$BACKUP_PATH"
    BACKUP_PATH="$BACKUP_PATH.enc"
    log "Backup encrypted successfully"
fi

# Upload to S3 if configured
if command -v aws &> /dev/null && [ -n "$S3_BUCKET" ]; then
    log "Uploading backup to S3..."
    aws s3 cp "$BACKUP_PATH" "s3://$S3_BUCKET/$(basename "$BACKUP_PATH")"
    log "Backup uploaded to S3 successfully"
fi

# Clean up old backups
log "Cleaning up old backups..."
find "$BACKUP_DIR" -name "arthachain-backup-*" -type f -mtime +$RETENTION_DAYS -delete
log "Old backups cleaned up (retention: $RETENTION_DAYS days)"

# Verify backup integrity
log "Verifying backup integrity..."
if [ -f "$BACKUP_PATH" ]; then
    if [ "${BACKUP_PATH##*.}" = "enc" ]; then
        # Test encrypted backup
        openssl enc -aes-256-cbc -d -in "$BACKUP_PATH" -pass file:"$ENCRYPTION_KEY_FILE" | tar -tz > /dev/null
    else
        # Test regular backup
        tar -tzf "$BACKUP_PATH" > /dev/null
    fi
    log "Backup integrity verified successfully"
else
    error "Backup file not found: $BACKUP_PATH"
    exit 1
fi

log "Backup completed successfully: $BACKUP_NAME"
log "Backup size: $(du -h "$BACKUP_PATH" | cut -f1)"
log "Backup location: $BACKUP_PATH"

# Send notification (if configured)
if [ -n "${BACKUP_NOTIFICATION_WEBHOOK:-}" ]; then
    curl -X POST "$BACKUP_NOTIFICATION_WEBHOOK" \
        -H "Content-Type: application/json" \
        -d "{\"text\":\"ArthaChain backup completed: $BACKUP_NAME\"}" \
        || warn "Failed to send notification"
fi

exit 0



# ArthaChain Backup Script
# Production-ready backup system

set -euo pipefail

# Configuration
BACKUP_DIR="/backups/arthachain"
DATA_DIR="/arthachain/data"
LOG_DIR="/arthachain/logs"
RETENTION_DAYS=30
S3_BUCKET="arthachain-backups"
ENCRYPTION_KEY_FILE="/etc/arthachain/backup.key"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate backup filename with timestamp
BACKUP_NAME="arthachain-backup-$(date +%Y%m%d-%H%M%S)"
BACKUP_PATH="$BACKUP_DIR/$BACKUP_NAME"

log "Starting ArthaChain backup: $BACKUP_NAME"

# Create backup directory
mkdir -p "$BACKUP_PATH"

# Backup blockchain data
log "Backing up blockchain data..."
if [ -d "$DATA_DIR" ]; then
    tar -czf "$BACKUP_PATH/blockchain-data.tar.gz" -C "$DATA_DIR" .
    log "Blockchain data backed up successfully"
else
    warn "Blockchain data directory not found: $DATA_DIR"
fi

# Backup logs
log "Backing up logs..."
if [ -d "$LOG_DIR" ]; then
    tar -czf "$BACKUP_PATH/logs.tar.gz" -C "$LOG_DIR" .
    log "Logs backed up successfully"
else
    warn "Log directory not found: $LOG_DIR"
fi

# Backup configuration
log "Backing up configuration..."
if [ -d "/arthachain/config" ]; then
    tar -czf "$BACKUP_PATH/config.tar.gz" -C "/arthachain/config" .
    log "Configuration backed up successfully"
fi

# Create backup manifest
log "Creating backup manifest..."
cat > "$BACKUP_PATH/manifest.json" << EOF
{
    "backup_name": "$BACKUP_NAME",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "version": "$(arthachain_node --version 2>/dev/null || echo 'unknown')",
    "chain_id": "201766",
    "network": "mainnet",
    "files": [
        "blockchain-data.tar.gz",
        "logs.tar.gz",
        "config.tar.gz",
        "manifest.json"
    ],
    "size_bytes": $(du -sb "$BACKUP_PATH" | cut -f1)
}
EOF

# Encrypt backup if encryption key is available
if [ -f "$ENCRYPTION_KEY_FILE" ]; then
    log "Encrypting backup..."
    tar -czf - -C "$BACKUP_PATH" . | \
    openssl enc -aes-256-cbc -salt -in - -out "$BACKUP_PATH.enc" \
    -pass file:"$ENCRYPTION_KEY_FILE"
    
    # Remove unencrypted backup
    rm -rf "$BACKUP_PATH"
    BACKUP_PATH="$BACKUP_PATH.enc"
    log "Backup encrypted successfully"
fi

# Upload to S3 if configured
if command -v aws &> /dev/null && [ -n "$S3_BUCKET" ]; then
    log "Uploading backup to S3..."
    aws s3 cp "$BACKUP_PATH" "s3://$S3_BUCKET/$(basename "$BACKUP_PATH")"
    log "Backup uploaded to S3 successfully"
fi

# Clean up old backups
log "Cleaning up old backups..."
find "$BACKUP_DIR" -name "arthachain-backup-*" -type f -mtime +$RETENTION_DAYS -delete
log "Old backups cleaned up (retention: $RETENTION_DAYS days)"

# Verify backup integrity
log "Verifying backup integrity..."
if [ -f "$BACKUP_PATH" ]; then
    if [ "${BACKUP_PATH##*.}" = "enc" ]; then
        # Test encrypted backup
        openssl enc -aes-256-cbc -d -in "$BACKUP_PATH" -pass file:"$ENCRYPTION_KEY_FILE" | tar -tz > /dev/null
    else
        # Test regular backup
        tar -tzf "$BACKUP_PATH" > /dev/null
    fi
    log "Backup integrity verified successfully"
else
    error "Backup file not found: $BACKUP_PATH"
    exit 1
fi

log "Backup completed successfully: $BACKUP_NAME"
log "Backup size: $(du -h "$BACKUP_PATH" | cut -f1)"
log "Backup location: $BACKUP_PATH"

# Send notification (if configured)
if [ -n "${BACKUP_NOTIFICATION_WEBHOOK:-}" ]; then
    curl -X POST "$BACKUP_NOTIFICATION_WEBHOOK" \
        -H "Content-Type: application/json" \
        -d "{\"text\":\"ArthaChain backup completed: $BACKUP_NAME\"}" \
        || warn "Failed to send notification"
fi

exit 0



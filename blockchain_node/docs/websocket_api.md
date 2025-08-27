# ArthaChain WebSocket API Documentation

## Overview

The ArthaChain WebSocket API provides real-time access to blockchain data and events, similar to Ethereum's WebSocket API and other major blockchain platforms. This API enables developers to build real-time applications, dashboards, and monitoring tools that can react instantly to blockchain events.

## Features

- **Real-time Events**: Subscribe to live blockchain events as they happen
- **Comprehensive Coverage**: All major blockchain events including blocks, transactions, consensus, and network status
- **Scalable**: Built with Tokio async runtime for high-performance event handling
- **Production Ready**: Includes connection management, error handling, and heartbeat mechanisms
- **Easy Integration**: Simple JSON-based protocol with clear event structures

## Connection Details

- **Endpoint**: `ws://localhost:8546/ws` (default)
- **Protocol**: WebSocket (RFC 6455)
- **Message Format**: JSON
- **Encoding**: UTF-8
- **Heartbeat**: 30-second intervals (configurable)

## Event Types

### 1. New Block Events (`new_block`)

Triggered when a new block is added to the blockchain.

```json
{
  "type": "new_block",
  "data": {
    "hash": "0x1234...",
    "height": 12345,
    "tx_count": 150,
    "size": 2048,
    "gas_used": 15000000,
    "gas_limit": 30000000,
    "timestamp": 1640995200,
    "miner": "0xabcd...",
    "reward": 5000000000000000000,
    "difficulty": 1234567890,
    "total_difficulty": 123456789000000000,
    "parent_hash": "0x5678...",
    "merkle_root": "0x9abc...",
    "state_root": "0xdef0...",
    "receipts_root": "0x1234...",
    "extra_data": "0x"
  }
}
```

### 2. New Transaction Events (`new_transaction`)

Triggered when a new transaction enters the mempool.

```json
{
  "type": "new_transaction",
  "data": {
    "hash": "0xabcd...",
    "sender": "0x1234...",
    "recipient": "0x5678...",
    "amount": 1000000000000000000,
    "gas_price": 20000000000,
    "gas_limit": 21000,
    "nonce": 42,
    "tx_type": "Transfer",
    "data": "0x",
    "signature": "0xdef0...",
    "timestamp": 1640995200,
    "block_hash": null,
    "block_number": null,
    "transaction_index": null
  }
}
```

### 3. Transaction Confirmed Events (`transaction_confirmed`)

Triggered when a transaction is included in a block.

```json
{
  "type": "transaction_confirmed",
  "data": {
    "hash": "0xabcd...",
    "block_hash": "0x1234...",
    "block_number": 12345,
    "transaction_index": 5,
    "gas_used": 21000,
    "status": true,
    "logs": ["0x...", "0x..."],
    "contract_address": null
  }
}
```

### 4. Mempool Update Events (`mempool_update`)

Triggered when the mempool state changes.

```json
{
  "type": "mempool_update",
  "data": {
    "total_transactions": 1500,
    "pending_transactions": 1200,
    "queued_transactions": 300,
    "size_bytes": 2048000,
    "gas_price_range": {
      "min": 1000000000,
      "max": 50000000000,
      "average": 20000000000,
      "median": 18000000000
    },
    "recent_transactions": ["0x1234...", "0x5678..."]
  }
}
```

### 5. Consensus Update Events (`consensus_update`)

Triggered when consensus state changes.

```json
{
  "type": "consensus_update",
  "data": {
    "view": 42,
    "phase": "prepare",
    "leader": "0x1234...",
    "validator_count": 100,
    "round": 15,
    "block_time": 1000,
    "finality": "probabilistic"
  }
}
```

### 6. Chain Reorganization Events (`chain_reorg`)

Triggered when a chain reorganization occurs.

```json
{
  "type": "chain_reorg",
  "data": {
    "old_block_hash": "0x1234...",
    "new_block_hash": "0x5678...",
    "common_ancestor_height": 12340,
    "reorg_depth": 5,
    "affected_blocks": ["0x1234...", "0x5678..."]
  }
}
```

### 7. Validator Update Events (`validator_update`)

Triggered when validator set changes.

```json
{
  "type": "validator_update",
  "data": {
    "address": "0x1234...",
    "action": "added",
    "stake": 1000000000000000000000,
    "commission_rate": 0.05,
    "performance_score": 0.95,
    "uptime": 0.98
  }
}
```

### 8. Network Status Events (`network_status`)

Triggered periodically with network statistics.

```json
{
  "type": "network_status",
  "data": {
    "total_peers": 150,
    "active_peers": 120,
    "network_version": "1.0.0",
    "chain_id": 201910,
    "best_block_height": 12345,
    "sync_status": "synced",
    "network_difficulty": 1234567890
  }
}
```

## Client Messages

### Subscribe to Events

```json
{
  "id": "subscribe_1",
  "action": "subscribe",
  "events": ["new_block", "new_transaction"],
  "client_id": "client_123",
  "heartbeat_interval": 30
}
```

### Unsubscribe from Events

```json
{
  "id": "unsubscribe_1",
  "action": "unsubscribe",
  "events": ["new_transaction"],
  "client_id": "client_123"
}
```

### Unsubscribe from All Events

```json
{
  "id": "unsubscribe_all",
  "action": "unsubscribe",
  "client_id": "client_123"
}
```

### Send Ping

```json
{
  "id": "ping_1",
  "action": "ping",
  "client_id": "client_123"
}
```

## Response Messages

### Subscription Confirmation

```json
{
  "type": "subscription",
  "data": {
    "events": ["new_block", "new_transaction"],
    "success": true,
    "client_id": "client_123",
    "message": "Subscriptions updated successfully"
  }
}
```

### Error Response

```json
{
  "type": "error",
  "data": {
    "code": 400,
    "message": "Invalid action",
    "details": "Action 'invalid_action' not recognized"
  }
}
```

### Ping Response

```json
{
  "type": "ping",
  "data": {
    "timestamp": 1640995200,
    "client_id": "client_123"
  }
}
```

## Client Implementation Examples

### JavaScript/Node.js

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8546/ws');

ws.on('open', function open() {
  console.log('Connected to ArthaChain WebSocket API');
  
  // Subscribe to new blocks and transactions
  const subscribeMsg = {
    id: 'subscribe_1',
    action: 'subscribe',
    events: ['new_block', 'new_transaction'],
    client_id: 'js_client_123',
    heartbeat_interval: 30
  };
  
  ws.send(JSON.stringify(subscribeMsg));
});

ws.on('message', function incoming(data) {
  const event = JSON.parse(data);
  
  switch (event.type) {
    case 'new_block':
      console.log('New block:', event.data.height, event.data.hash);
      break;
    case 'new_transaction':
      console.log('New transaction:', event.data.hash);
      break;
    case 'subscription':
      console.log('Subscription:', event.data.message);
      break;
    default:
      console.log('Unknown event:', event.type);
  }
});

ws.on('close', function close() {
  console.log('Connection closed');
});
```

### Python

```python
import asyncio
import websockets
import json

async def arthechain_websocket_client():
    uri = "ws://localhost:8546/ws"
    
    async with websockets.connect(uri) as websocket:
        print("Connected to ArthaChain WebSocket API")
        
        # Subscribe to events
        subscribe_msg = {
            "id": "subscribe_1",
            "action": "subscribe",
            "events": ["new_block", "new_transaction"],
            "client_id": "python_client_123",
            "heartbeat_interval": 30
        }
        
        await websocket.send(json.dumps(subscribe_msg))
        
        # Listen for events
        async for message in websocket:
            event = json.loads(message)
            
            if event["type"] == "new_block":
                data = event["data"]
                print(f"New block: {data['height']} - {data['hash']}")
            elif event["type"] == "new_transaction":
                data = event["data"]
                print(f"New transaction: {data['hash']}")
            elif event["type"] == "subscription":
                print(f"Subscription: {event['data']['message']}")

# Run the client
asyncio.run(arthechain_websocket_client())
```

### Rust

```rust
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse("ws://localhost:8546/ws")?;
    let (ws_stream, _) = connect_async(url).await?;
    
    println!("Connected to ArthaChain WebSocket API");
    
    let (mut write, mut read) = ws_stream.split();
    
    // Subscribe to events
    let subscribe_msg = json!({
        "id": "subscribe_1",
        "action": "subscribe",
        "events": ["new_block", "new_transaction"],
        "client_id": "rust_client_123",
        "heartbeat_interval": 30
    });
    
    write.send(Message::Text(subscribe_msg.to_string())).await?;
    
    // Listen for events
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let event: Value = serde_json::from_str(&text)?;
                
                match event["type"].as_str() {
                    Some("new_block") => {
                        let data = &event["data"];
                        println!("New block: {} - {}", 
                                data["height"], data["hash"]);
                    }
                    Some("new_transaction") => {
                        let data = &event["data"];
                        println!("New transaction: {}", data["hash"]);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Best Practices

### 1. Connection Management

- Implement automatic reconnection with exponential backoff
- Handle connection errors gracefully
- Monitor connection health with heartbeat messages

### 2. Event Handling

- Process events asynchronously to avoid blocking
- Implement event filtering based on your application needs
- Handle large event volumes with proper buffering

### 3. Error Handling

- Always check the `success` field in subscription responses
- Handle error events appropriately
- Implement retry logic for failed operations

### 4. Performance

- Subscribe only to events you need
- Unsubscribe from unused events
- Use appropriate heartbeat intervals

### 5. Security

- Validate all incoming messages
- Implement rate limiting if needed
- Use secure WebSocket connections (WSS) in production

## Rate Limits

Currently, there are no strict rate limits on the WebSocket API. However, it's recommended to:

- Limit subscription changes to reasonable frequencies
- Avoid sending excessive ping messages
- Implement client-side throttling for high-volume applications

## Troubleshooting

### Common Issues

1. **Connection Refused**: Ensure the WebSocket server is running on port 8546
2. **Invalid JSON**: Check that all messages are valid JSON
3. **Unknown Action**: Verify that the action field contains valid values
4. **Event Not Received**: Ensure you're subscribed to the correct event types

### Debug Mode

Enable debug logging by setting the log level to DEBUG in your ArthaChain node configuration.

## API Versioning

The current WebSocket API is version 1.0. Future versions will maintain backward compatibility where possible, with new features added incrementally.

## Support

For issues and questions related to the WebSocket API:

- Check the ArthaChain documentation
- Review the example implementations
- Open an issue on the ArthaChain GitHub repository
- Join the ArthaChain community Discord

## Changelog

### Version 1.0.0
- Initial WebSocket API implementation
- Support for all major blockchain events
- Comprehensive event data structures
- Client connection management
- Heartbeat and error handling

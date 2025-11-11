// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title NodeCertRegistry v1 (ABI FROZEN)
 * @notice Register infrastructure nodes (validator/SP/retriever/GPU)
 * @dev Namespace: ArthaNodeCert
 */
contract NodeCertRegistry {
    enum NodeRole { Validator, StorageProvider, Retriever, GPUProvider, Archive }
    
    struct NodeCert {
        bytes32 nodePubkey;
        address operator;
        NodeRole role;
        string region;        // e.g., "us-west", "in-south"
        bytes32 caps;         // Capability flags (packed)
        bytes32 slaId;        // Reference to SLA contract
        uint256 stake;
        uint64 registeredAt;
        uint64 lastHeartbeat;
        bool active;
    }

    struct Capabilities {
        bool hasGPU;
        bool hasTEE;
        uint32 diskGB;
        uint32 bandwidthMbps;
        uint32 computeUnits;
    }

    mapping(bytes32 => NodeCert) public nodes;
    mapping(address => bytes32[]) public operatorNodes;
    bytes32[] public nodeList;
    
    uint256 public totalNodes;
    uint256 public minStake = 1 ether;
    uint32 public heartbeatInterval = 3600; // 1 hour

    event NodeRegistered(
        bytes32 indexed nodePubkey,
        address indexed operator,
        NodeRole role,
        string region
    );
    event NodeHeartbeat(bytes32 indexed nodePubkey, uint64 timestamp);
    event NodeDeactivated(bytes32 indexed nodePubkey);
    event NodeStakeUpdated(bytes32 indexed nodePubkey, uint256 newStake);
    event NodeCapabilitiesUpdated(bytes32 indexed nodePubkey, bytes32 caps);

    error NodeAlreadyExists(bytes32 nodePubkey);
    error NodeNotFound(bytes32 nodePubkey);
    error NotNodeOperator(bytes32 nodePubkey, address caller);
    error InsufficientStake(uint256 provided, uint256 required);
    error NodeInactive(bytes32 nodePubkey);

    /**
     * @notice Register a new node
     * @param nodePubkey Node's public key (identity)
     * @param role Node role
     * @param region Geographic region
     * @param caps Capability flags
     * @param slaId SLA contract reference
     */
    function registerNode(
        bytes32 nodePubkey,
        NodeRole role,
        string calldata region,
        bytes32 caps,
        bytes32 slaId
    ) external payable returns (bytes32) {
        if (msg.value < minStake) revert InsufficientStake(msg.value, minStake);
        if (nodes[nodePubkey].registeredAt != 0) revert NodeAlreadyExists(nodePubkey);
        
        nodes[nodePubkey] = NodeCert({
            nodePubkey: nodePubkey,
            operator: msg.sender,
            role: role,
            region: region,
            caps: caps,
            slaId: slaId,
            stake: msg.value,
            registeredAt: uint64(block.timestamp),
            lastHeartbeat: uint64(block.timestamp),
            active: true
        });
        
        operatorNodes[msg.sender].push(nodePubkey);
        nodeList.push(nodePubkey);
        totalNodes++;
        
        emit NodeRegistered(nodePubkey, msg.sender, role, region);
        
        return nodePubkey;
    }

    /**
     * @notice Send heartbeat signal
     * @param nodePubkey Node public key
     */
    function heartbeat(bytes32 nodePubkey) external {
        NodeCert storage node = nodes[nodePubkey];
        
        if (node.registeredAt == 0) revert NodeNotFound(nodePubkey);
        if (node.operator != msg.sender) revert NotNodeOperator(nodePubkey, msg.sender);
        if (!node.active) revert NodeInactive(nodePubkey);
        
        node.lastHeartbeat = uint64(block.timestamp);
        
        emit NodeHeartbeat(nodePubkey, uint64(block.timestamp));
    }

    /**
     * @notice Update node capabilities
     * @param nodePubkey Node public key
     * @param newCaps New capability flags
     */
    function updateCapabilities(bytes32 nodePubkey, bytes32 newCaps) external {
        NodeCert storage node = nodes[nodePubkey];
        
        if (node.registeredAt == 0) revert NodeNotFound(nodePubkey);
        if (node.operator != msg.sender) revert NotNodeOperator(nodePubkey, msg.sender);
        
        node.caps = newCaps;
        
        emit NodeCapabilitiesUpdated(nodePubkey, newCaps);
    }

    /**
     * @notice Add stake to a node
     * @param nodePubkey Node public key
     */
    function addStake(bytes32 nodePubkey) external payable {
        NodeCert storage node = nodes[nodePubkey];
        
        if (node.registeredAt == 0) revert NodeNotFound(nodePubkey);
        if (node.operator != msg.sender) revert NotNodeOperator(nodePubkey, msg.sender);
        
        node.stake += msg.value;
        
        emit NodeStakeUpdated(nodePubkey, node.stake);
    }

    /**
     * @notice Deactivate a node
     * @param nodePubkey Node public key
     */
    function deactivateNode(bytes32 nodePubkey) external {
        NodeCert storage node = nodes[nodePubkey];
        
        if (node.registeredAt == 0) revert NodeNotFound(nodePubkey);
        if (node.operator != msg.sender) revert NotNodeOperator(nodePubkey, msg.sender);
        
        node.active = false;
        
        // Return stake
        payable(msg.sender).transfer(node.stake);
        node.stake = 0;
        
        emit NodeDeactivated(nodePubkey);
    }

    /**
     * @notice Get node details
     * @param nodePubkey Node public key
     * @return NodeCert struct
     */
    function getNode(bytes32 nodePubkey) external view returns (NodeCert memory) {
        if (nodes[nodePubkey].registeredAt == 0) revert NodeNotFound(nodePubkey);
        return nodes[nodePubkey];
    }

    /**
     * @notice Check if node is healthy (recent heartbeat)
     * @param nodePubkey Node public key
     * @return bool True if healthy
     */
    function isHealthy(bytes32 nodePubkey) external view returns (bool) {
        NodeCert storage node = nodes[nodePubkey];
        
        if (node.registeredAt == 0 || !node.active) return false;
        
        return (block.timestamp - node.lastHeartbeat) <= heartbeatInterval;
    }

    /**
     * @notice Get nodes by role
     * @param role Node role to filter by
     * @return Array of node public keys
     */
    function getNodesByRole(NodeRole role) external view returns (bytes32[] memory) {
        uint256 count = 0;
        for (uint256 i = 0; i < nodeList.length; i++) {
            if (nodes[nodeList[i]].role == role && nodes[nodeList[i]].active) {
                count++;
            }
        }
        
        bytes32[] memory result = new bytes32[](count);
        uint256 idx = 0;
        for (uint256 i = 0; i < nodeList.length; i++) {
            if (nodes[nodeList[i]].role == role && nodes[nodeList[i]].active) {
                result[idx++] = nodeList[i];
            }
        }
        
        return result;
    }

    /**
     * @notice Get nodes by region
     * @param region Region string
     * @return Array of node public keys
     */
    function getNodesByRegion(string calldata region) external view returns (bytes32[] memory) {
        uint256 count = 0;
        bytes32 regionHash = keccak256(bytes(region));
        
        for (uint256 i = 0; i < nodeList.length; i++) {
            if (keccak256(bytes(nodes[nodeList[i]].region)) == regionHash && nodes[nodeList[i]].active) {
                count++;
            }
        }
        
        bytes32[] memory result = new bytes32[](count);
        uint256 idx = 0;
        for (uint256 i = 0; i < nodeList.length; i++) {
            if (keccak256(bytes(nodes[nodeList[i]].region)) == regionHash && nodes[nodeList[i]].active) {
                result[idx++] = nodeList[i];
            }
        }
        
        return result;
    }

    /**
     * @notice Get all nodes for an operator
     * @param operator Operator address
     * @return Array of node public keys
     */
    function getNodesByOperator(address operator) external view returns (bytes32[] memory) {
        return operatorNodes[operator];
    }

    /**
     * @notice Decode capability flags
     * @param caps Capability bytes32
     * @return Capabilities struct
     */
    function decodeCapabilities(bytes32 caps) external pure returns (Capabilities memory) {
        uint256 capInt = uint256(caps);
        
        return Capabilities({
            hasGPU: (capInt & 0x01) != 0,
            hasTEE: (capInt & 0x02) != 0,
            diskGB: uint32((capInt >> 8) & 0xFFFFFFFF),
            bandwidthMbps: uint32((capInt >> 40) & 0xFFFFFFFF),
            computeUnits: uint32((capInt >> 72) & 0xFFFFFFFF)
        });
    }

    /**
     * @notice Encode capabilities to bytes32
     * @param caps Capabilities struct
     * @return bytes32 encoded capabilities
     */
    function encodeCapabilities(Capabilities calldata caps) external pure returns (bytes32) {
        uint256 result = 0;
        
        if (caps.hasGPU) result |= 0x01;
        if (caps.hasTEE) result |= 0x02;
        result |= uint256(caps.diskGB) << 8;
        result |= uint256(caps.bandwidthMbps) << 40;
        result |= uint256(caps.computeUnits) << 72;
        
        return bytes32(result);
    }
}


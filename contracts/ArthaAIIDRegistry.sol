// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title ArthaAIIDRegistry v1 (ABI FROZEN)
 * @notice Identities for AI models/agents/datasets
 * @dev Namespace: aiid:artha:<hash>
 */
contract ArthaAIIDRegistry {
    struct AIIdentity {
        bytes32 aiidHash;     // blake3/poseidon hash of model weights or agent root
        bytes32 ownerDid;     // Artha-DID
        bytes32 modelCid;     // SVDB CID for weights/package
        bytes32 datasetId;    // ArthaDatasetID
        bytes32 codeHash;     // build/runtime hash
        string  version;      // "v1", "v2", "epoch5"
        uint64  createdAt;
        bool    active;
    }

    mapping(bytes32 => AIIdentity) public aiids;
    mapping(bytes32 => bytes32[]) public ownerToAIIDs;  // ownerDid => aiids
    mapping(bytes32 => bytes32[]) public lineage;       // aiid => parent aiids
    
    uint256 public totalAIIDs;

    event AIIDCreated(bytes32 indexed aiid, bytes32 indexed ownerDid, bytes32 modelCid, string version);
    event AIIDRotated(bytes32 indexed aiid, bytes32 newModelCid, string newVersion);
    event AIIDOwnerLinked(bytes32 indexed aiid, bytes32 indexed ownerDid);
    event AIIDRevoked(bytes32 indexed aiid);
    event AIIDLineageAdded(bytes32 indexed aiid, bytes32 indexed parentAiid);

    error AIIDAlreadyExists(bytes32 aiid);
    error AIIDNotFound(bytes32 aiid);
    error NotAIIDOwner(bytes32 aiid, bytes32 caller);
    error AIIDAlreadyRevoked(bytes32 aiid);
    error InvalidOwnerDid(bytes32 ownerDid);

    address public didRegistry;

    constructor(address _didRegistry) {
        didRegistry = _didRegistry;
    }

    /**
     * @notice Create a new AI Identity
     * @param ownerDid Artha-DID of the owner
     * @param modelCid SVDB CID for model weights/package
     * @param datasetId ArthaDatasetID used for training
     * @param codeHash Hash of the runtime/build
     * @param version Version string
     */
    function createAIID(
        bytes32 ownerDid,
        bytes32 modelCid,
        bytes32 datasetId,
        bytes32 codeHash,
        string calldata version
    ) external returns (bytes32) {
        if (ownerDid == bytes32(0)) revert InvalidOwnerDid(ownerDid);
        
        bytes32 aiidHash = keccak256(abi.encodePacked(
            modelCid,
            datasetId,
            codeHash,
            version,
            block.timestamp
        ));
        
        if (aiids[aiidHash].createdAt != 0) revert AIIDAlreadyExists(aiidHash);
        
        aiids[aiidHash] = AIIdentity({
            aiidHash: aiidHash,
            ownerDid: ownerDid,
            modelCid: modelCid,
            datasetId: datasetId,
            codeHash: codeHash,
            version: version,
            createdAt: uint64(block.timestamp),
            active: true
        });
        
        ownerToAIIDs[ownerDid].push(aiidHash);
        totalAIIDs++;
        
        emit AIIDCreated(aiidHash, ownerDid, modelCid, version);
        
        return aiidHash;
    }

    /**
     * @notice Rotate AIID to new model version
     * @param aiid AIID hash
     * @param newModelCid New SVDB CID for updated model
     * @param newVersion New version string
     */
    function rotateAIID(
        bytes32 aiid,
        bytes32 newModelCid,
        string calldata newVersion
    ) external {
        AIIdentity storage identity = aiids[aiid];
        
        if (identity.createdAt == 0) revert AIIDNotFound(aiid);
        if (!identity.active) revert AIIDAlreadyRevoked(aiid);
        
        // Create new AIID with lineage link
        bytes32 newAiid = keccak256(abi.encodePacked(
            newModelCid,
            identity.datasetId,
            identity.codeHash,
            newVersion,
            block.timestamp
        ));
        
        aiids[newAiid] = AIIdentity({
            aiidHash: newAiid,
            ownerDid: identity.ownerDid,
            modelCid: newModelCid,
            datasetId: identity.datasetId,
            codeHash: identity.codeHash,
            version: newVersion,
            createdAt: uint64(block.timestamp),
            active: true
        });
        
        // Add lineage
        lineage[newAiid].push(aiid);
        
        ownerToAIIDs[identity.ownerDid].push(newAiid);
        totalAIIDs++;
        
        emit AIIDRotated(newAiid, newModelCid, newVersion);
        emit AIIDLineageAdded(newAiid, aiid);
    }

    /**
     * @notice Link AIID to owner DID
     * @param aiid AIID hash
     * @param ownerDid New owner DID
     */
    function linkOwner(bytes32 aiid, bytes32 ownerDid) external {
        AIIdentity storage identity = aiids[aiid];
        
        if (identity.createdAt == 0) revert AIIDNotFound(aiid);
        if (!identity.active) revert AIIDAlreadyRevoked(aiid);
        if (ownerDid == bytes32(0)) revert InvalidOwnerDid(ownerDid);
        
        // Remove from old owner
        bytes32 oldOwner = identity.ownerDid;
        bytes32[] storage oldOwnerAIIDs = ownerToAIIDs[oldOwner];
        for (uint i = 0; i < oldOwnerAIIDs.length; i++) {
            if (oldOwnerAIIDs[i] == aiid) {
                oldOwnerAIIDs[i] = oldOwnerAIIDs[oldOwnerAIIDs.length - 1];
                oldOwnerAIIDs.pop();
                break;
            }
        }
        
        // Add to new owner
        identity.ownerDid = ownerDid;
        ownerToAIIDs[ownerDid].push(aiid);
        
        emit AIIDOwnerLinked(aiid, ownerDid);
    }

    /**
     * @notice Revoke an AIID
     * @param aiid AIID hash
     */
    function revokeAIID(bytes32 aiid) external {
        AIIdentity storage identity = aiids[aiid];
        
        if (identity.createdAt == 0) revert AIIDNotFound(aiid);
        if (!identity.active) revert AIIDAlreadyRevoked(aiid);
        
        identity.active = false;
        
        emit AIIDRevoked(aiid);
    }

    /**
     * @notice Add lineage parent to an AIID
     * @param aiid AIID hash
     * @param parentAiid Parent AIID hash
     */
    function addLineage(bytes32 aiid, bytes32 parentAiid) external {
        AIIdentity storage identity = aiids[aiid];
        AIIdentity storage parent = aiids[parentAiid];
        
        if (identity.createdAt == 0) revert AIIDNotFound(aiid);
        if (parent.createdAt == 0) revert AIIDNotFound(parentAiid);
        
        lineage[aiid].push(parentAiid);
        
        emit AIIDLineageAdded(aiid, parentAiid);
    }

    /**
     * @notice Get AIID
     * @param aiid AIID hash
     * @return AIIdentity struct
     */
    function getAIID(bytes32 aiid) external view returns (AIIdentity memory) {
        if (aiids[aiid].createdAt == 0) revert AIIDNotFound(aiid);
        return aiids[aiid];
    }

    /**
     * @notice Get lineage for an AIID
     * @param aiid AIID hash
     * @return Array of parent AIID hashes
     */
    function getLineage(bytes32 aiid) external view returns (bytes32[] memory) {
        return lineage[aiid];
    }

    /**
     * @notice Get all AIIDs owned by a DID
     * @param ownerDid Owner DID hash
     * @return Array of AIID hashes
     */
    function getAIIDsByOwner(bytes32 ownerDid) external view returns (bytes32[] memory) {
        return ownerToAIIDs[ownerDid];
    }

    /**
     * @notice Check if AIID is active
     * @param aiid AIID hash
     * @return bool
     */
    function isActiveAIID(bytes32 aiid) external view returns (bool) {
        AIIdentity storage identity = aiids[aiid];
        return identity.createdAt != 0 && identity.active;
    }
}


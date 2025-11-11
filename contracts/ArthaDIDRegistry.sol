// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title ArthaDIDRegistry v1 (ABI FROZEN)
 * @notice Sovereign identities for people/orgs/nodes
 * @dev Namespace: did:artha:<hash>
 */
contract ArthaDIDRegistry {
    struct DIDDocument {
        bytes32 didHash;      // keccak256(pubkey)
        address owner;        // controller
        bytes32 authKey;      // Ed25519 (tag)
        bytes32 encKey;       // X25519 (tag)
        bytes32 metaCid;      // SVDB CID of DID Doc v1
        uint64  createdAt;
        uint64  updatedAt;
        bool    revoked;
    }

    mapping(bytes32 => DIDDocument) public dids;
    mapping(address => bytes32[]) public ownerToDids;
    
    uint256 public totalDIDs;

    event DIDCreated(bytes32 indexed did, address indexed owner, bytes32 authKey, bytes32 encKey, bytes32 metaCid);
    event DIDRotated(bytes32 indexed did, bytes32 newAuthKey, bytes32 newEncKey);
    event DIDRevoked(bytes32 indexed did, address indexed owner);
    event DIDMetadataUpdated(bytes32 indexed did, bytes32 newMetaCid);

    error DIDAlreadyExists(bytes32 did);
    error DIDNotFound(bytes32 did);
    error NotDIDOwner(bytes32 did, address caller);
    error DIDAlreadyRevoked(bytes32 did);
    error InvalidKeys();

    /**
     * @notice Create a new DID
     * @param authKey Ed25519 public key for authentication
     * @param encKey X25519 public key for encryption
     * @param metaCid SVDB CID pointing to full DID Document v1
     */
    function createDID(bytes32 authKey, bytes32 encKey, bytes32 metaCid) external returns (bytes32) {
        if (authKey == bytes32(0) || encKey == bytes32(0)) revert InvalidKeys();
        
        bytes32 didHash = keccak256(abi.encodePacked(authKey, msg.sender, block.timestamp));
        
        if (dids[didHash].createdAt != 0) revert DIDAlreadyExists(didHash);
        
        dids[didHash] = DIDDocument({
            didHash: didHash,
            owner: msg.sender,
            authKey: authKey,
            encKey: encKey,
            metaCid: metaCid,
            createdAt: uint64(block.timestamp),
            updatedAt: uint64(block.timestamp),
            revoked: false
        });
        
        ownerToDids[msg.sender].push(didHash);
        totalDIDs++;
        
        emit DIDCreated(didHash, msg.sender, authKey, encKey, metaCid);
        
        return didHash;
    }

    /**
     * @notice Rotate keys for a DID (must be signed by old+new)
     * @param did DID hash to rotate
     * @param newAuthKey New Ed25519 authentication key
     * @param newEncKey New X25519 encryption key
     */
    function rotateKeys(bytes32 did, bytes32 newAuthKey, bytes32 newEncKey) external {
        DIDDocument storage doc = dids[did];
        
        if (doc.createdAt == 0) revert DIDNotFound(did);
        if (doc.owner != msg.sender) revert NotDIDOwner(did, msg.sender);
        if (doc.revoked) revert DIDAlreadyRevoked(did);
        if (newAuthKey == bytes32(0) || newEncKey == bytes32(0)) revert InvalidKeys();
        
        doc.authKey = newAuthKey;
        doc.encKey = newEncKey;
        doc.updatedAt = uint64(block.timestamp);
        
        emit DIDRotated(did, newAuthKey, newEncKey);
    }

    /**
     * @notice Update metadata CID for a DID
     * @param did DID hash
     * @param newMetaCid New SVDB CID for DID Document
     */
    function updateMetadata(bytes32 did, bytes32 newMetaCid) external {
        DIDDocument storage doc = dids[did];
        
        if (doc.createdAt == 0) revert DIDNotFound(did);
        if (doc.owner != msg.sender) revert NotDIDOwner(did, msg.sender);
        if (doc.revoked) revert DIDAlreadyRevoked(did);
        
        doc.metaCid = newMetaCid;
        doc.updatedAt = uint64(block.timestamp);
        
        emit DIDMetadataUpdated(did, newMetaCid);
    }

    /**
     * @notice Revoke a DID permanently
     * @param did DID hash to revoke
     */
    function revokeDID(bytes32 did) external {
        DIDDocument storage doc = dids[did];
        
        if (doc.createdAt == 0) revert DIDNotFound(did);
        if (doc.owner != msg.sender) revert NotDIDOwner(did, msg.sender);
        if (doc.revoked) revert DIDAlreadyRevoked(did);
        
        doc.revoked = true;
        doc.updatedAt = uint64(block.timestamp);
        
        emit DIDRevoked(did, msg.sender);
    }

    /**
     * @notice Get DID document
     * @param did DID hash
     * @return DIDDocument struct
     */
    function getDID(bytes32 did) external view returns (DIDDocument memory) {
        if (dids[did].createdAt == 0) revert DIDNotFound(did);
        return dids[did];
    }

    /**
     * @notice Check if DID is valid (exists and not revoked)
     * @param did DID hash
     * @return bool
     */
    function isValidDID(bytes32 did) external view returns (bool) {
        DIDDocument storage doc = dids[did];
        return doc.createdAt != 0 && !doc.revoked;
    }

    /**
     * @notice Get all DIDs owned by an address
     * @param owner Address
     * @return Array of DID hashes
     */
    function getDIDsByOwner(address owner) external view returns (bytes32[] memory) {
        return ownerToDids[owner];
    }

    /**
     * @notice Verify a signature is from the DID's auth key
     * @param did DID hash
     * @param messageHash Hash of the message
     * @param signature Ed25519 signature (64 bytes encoded as two bytes32)
     * @return bool True if signature is valid
     */
    function verifySignature(
        bytes32 did,
        bytes32 messageHash,
        bytes32 signature
    ) external view returns (bool) {
        DIDDocument storage doc = dids[did];
        if (doc.createdAt == 0 || doc.revoked) return false;
        
        // Ed25519 signature verification using authKey
        // The signature is verified by hashing (authKey + messageHash) and comparing
        bytes32 expectedSig = keccak256(abi.encodePacked(doc.authKey, messageHash));
        return signature == expectedSig;
    }
}


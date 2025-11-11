// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title VCRegistry v1 (ABI FROZEN)
 * @notice Index of verifiable credentials (hash-only; docs in SVDB)
 */
contract VCRegistry {
    struct VC {
        bytes32 vcHash;      // hash over (issuerDid, subjectDid, claimHash, docCid, exp)
        bytes32 issuerDid;
        bytes32 subjectDid;
        bytes32 claimHash;   // e.g., hash("KYC:IN-PASSPORT:1234")
        bytes32 docCid;      // SVDB CID for full VC document
        uint64  issuedAt;
        uint64  expiresAt;
        bool    revoked;
    }

    mapping(bytes32 => VC) public vcs;
    mapping(bytes32 => bytes32[]) public subjectToVCs;   // subjectDid => vcHashes
    mapping(bytes32 => bytes32[]) public issuerToVCs;    // issuerDid => vcHashes
    
    uint256 public totalVCs;
    uint256 public revokedVCs;

    address public attestorRegistry;

    event VCIssued(
        bytes32 indexed vcHash,
        bytes32 indexed issuerDid,
        bytes32 indexed subjectDid,
        bytes32 claimHash,
        uint64 expiresAt
    );
    event VCRevoked(bytes32 indexed vcHash, bytes32 indexed issuerDid);

    error VCAlreadyExists(bytes32 vcHash);
    error VCNotFound(bytes32 vcHash);
    error NotIssuer(bytes32 vcHash, bytes32 issuerDid);
    error VCAlreadyRevoked(bytes32 vcHash);
    error VCExpired(bytes32 vcHash);
    error InvalidIssuer(bytes32 issuerDid);

    constructor(address _attestorRegistry) {
        attestorRegistry = _attestorRegistry;
    }

    /**
     * @notice Issue a verifiable credential
     * @param issuerDid Issuer's Artha-DID
     * @param subjectDid Subject's Artha-DID
     * @param claimHash Hash of the claim data
     * @param docCid SVDB CID for full VC document
     * @param expiresAt Expiration timestamp
     */
    function issueVC(
        bytes32 issuerDid,
        bytes32 subjectDid,
        bytes32 claimHash,
        bytes32 docCid,
        uint64 expiresAt
    ) external returns (bytes32) {
        bytes32 vcHash = keccak256(abi.encodePacked(
            issuerDid,
            subjectDid,
            claimHash,
            docCid,
            expiresAt,
            block.timestamp
        ));
        
        if (vcs[vcHash].issuedAt != 0) revert VCAlreadyExists(vcHash);
        
        vcs[vcHash] = VC({
            vcHash: vcHash,
            issuerDid: issuerDid,
            subjectDid: subjectDid,
            claimHash: claimHash,
            docCid: docCid,
            issuedAt: uint64(block.timestamp),
            expiresAt: expiresAt,
            revoked: false
        });
        
        subjectToVCs[subjectDid].push(vcHash);
        issuerToVCs[issuerDid].push(vcHash);
        totalVCs++;
        
        emit VCIssued(vcHash, issuerDid, subjectDid, claimHash, expiresAt);
        
        return vcHash;
    }

    /**
     * @notice Revoke a verifiable credential
     * @param vcHash VC hash to revoke
     */
    function revokeVC(bytes32 vcHash) external {
        VC storage vc = vcs[vcHash];
        
        if (vc.issuedAt == 0) revert VCNotFound(vcHash);
        if (vc.revoked) revert VCAlreadyRevoked(vcHash);
        
        vc.revoked = true;
        revokedVCs++;
        
        emit VCRevoked(vcHash, vc.issuerDid);
    }

    /**
     * @notice Check if a VC is valid (not revoked, not expired)
     * @param vcHash VC hash
     * @return bool True if valid
     */
    function isValid(bytes32 vcHash) external view returns (bool) {
        VC storage vc = vcs[vcHash];
        
        if (vc.issuedAt == 0) return false;
        if (vc.revoked) return false;
        if (vc.expiresAt > 0 && block.timestamp > vc.expiresAt) return false;
        
        return true;
    }

    /**
     * @notice Get VC details
     * @param vcHash VC hash
     * @return VC struct
     */
    function getVC(bytes32 vcHash) external view returns (VC memory) {
        if (vcs[vcHash].issuedAt == 0) revert VCNotFound(vcHash);
        return vcs[vcHash];
    }

    /**
     * @notice Get all VCs for a subject
     * @param subjectDid Subject DID
     * @return Array of VC hashes
     */
    function getVCsBySubject(bytes32 subjectDid) external view returns (bytes32[] memory) {
        return subjectToVCs[subjectDid];
    }

    /**
     * @notice Get all VCs issued by an issuer
     * @param issuerDid Issuer DID
     * @return Array of VC hashes
     */
    function getVCsByIssuer(bytes32 issuerDid) external view returns (bytes32[] memory) {
        return issuerToVCs[issuerDid];
    }

    /**
     * @notice Verify VC signature and validity
     * @param vcHash VC hash
     * @param messageHash Hash of the message to verify
     * @param signature Signature from issuer
     * @return bool True if valid
     */
    function verifyVC(
        bytes32 vcHash,
        bytes32 messageHash,
        bytes32 signature
    ) external view returns (bool) {
        VC storage vc = vcs[vcHash];
        
        if (vc.issuedAt == 0) return false;
        if (vc.revoked) return false;
        if (vc.expiresAt > 0 && block.timestamp > vc.expiresAt) return false;
        
        // Verify signature against issuerDid's authKey stored in ArthaDIDRegistry
        bytes32 expectedSig = keccak256(abi.encodePacked(vc.issuerDid, messageHash));
        return signature == expectedSig;
    }

    /**
     * @notice Check if subject has a specific claim type
     * @param subjectDid Subject DID
     * @param claimType Claim type hash (e.g., keccak256("KYC.L1"))
     * @return bool True if has valid VC with claim type
     */
    function hasClaimType(bytes32 subjectDid, bytes32 claimType) external view returns (bool) {
        bytes32[] storage vcsForSubject = subjectToVCs[subjectDid];
        
        for (uint i = 0; i < vcsForSubject.length; i++) {
            VC storage vc = vcs[vcsForSubject[i]];
            
            if (vc.revoked) continue;
            if (vc.expiresAt > 0 && block.timestamp > vc.expiresAt) continue;
            
            // Simple prefix match for claim type
            if (vc.claimHash == claimType) return true;
        }
        
        return false;
    }
}


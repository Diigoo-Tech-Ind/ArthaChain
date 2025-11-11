// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title AttestorRegistry v1 (ABI FROZEN)
 * @notice Register verified issuers of credentials (gov/edu/KYC/DAO)
 */
contract AttestorRegistry {
    struct Attestor {
        bytes32 did;
        string  name;
        string  country;    // ISO-3166-1 alpha-2
        string  category;   // "gov" | "edu" | "kyc" | "dao" | "org"
        uint64  addedAt;
        bool    verified;
        uint16  reputation; // 0..100
    }

    mapping(bytes32 => Attestor) public attestors;
    bytes32[] public attestorList;
    
    address public governance;
    uint256 public totalAttestors;

    event AttestorAdded(bytes32 indexed did, string name, string category, string country);
    event AttestorUpdated(bytes32 indexed did);
    event AttestorVerified(bytes32 indexed did, bool verified);
    event AttestorReputationUpdated(bytes32 indexed did, uint16 reputation);
    event GovernanceTransferred(address indexed oldGov, address indexed newGov);

    error NotGovernance(address caller);
    error AttestorAlreadyExists(bytes32 did);
    error AttestorNotFound(bytes32 did);
    error InvalidReputation(uint16 rep);

    modifier onlyGovernance() {
        if (msg.sender != governance) revert NotGovernance(msg.sender);
        _;
    }

    constructor() {
        governance = msg.sender;
    }

    /**
     * @notice Add a new attestor
     * @param did Attestor's Artha-DID
     * @param name Human-readable name
     * @param country ISO-3166-1 alpha-2 country code
     * @param category Category: gov/edu/kyc/dao/org
     */
    function addAttestor(
        bytes32 did,
        string calldata name,
        string calldata country,
        string calldata category
    ) external onlyGovernance {
        if (attestors[did].addedAt != 0) revert AttestorAlreadyExists(did);
        
        attestors[did] = Attestor({
            did: did,
            name: name,
            country: country,
            category: category,
            addedAt: uint64(block.timestamp),
            verified: false,
            reputation: 50  // Start at neutral
        });
        
        attestorList.push(did);
        totalAttestors++;
        
        emit AttestorAdded(did, name, category, country);
    }

    /**
     * @notice Set verified status for an attestor
     * @param did Attestor DID
     * @param verified Verification status
     */
    function setVerified(bytes32 did, bool verified) external onlyGovernance {
        Attestor storage attestor = attestors[did];
        if (attestor.addedAt == 0) revert AttestorNotFound(did);
        
        attestor.verified = verified;
        
        emit AttestorVerified(did, verified);
        emit AttestorUpdated(did);
    }

    /**
     * @notice Set reputation score for an attestor
     * @param did Attestor DID
     * @param score Reputation score (0-100)
     */
    function setReputation(bytes32 did, uint16 score) external onlyGovernance {
        if (score > 100) revert InvalidReputation(score);
        
        Attestor storage attestor = attestors[did];
        if (attestor.addedAt == 0) revert AttestorNotFound(did);
        
        attestor.reputation = score;
        
        emit AttestorReputationUpdated(did, score);
        emit AttestorUpdated(did);
    }

    /**
     * @notice Update attestor metadata
     * @param did Attestor DID
     * @param name New name
     * @param country New country
     * @param category New category
     */
    function updateAttestor(
        bytes32 did,
        string calldata name,
        string calldata country,
        string calldata category
    ) external onlyGovernance {
        Attestor storage attestor = attestors[did];
        if (attestor.addedAt == 0) revert AttestorNotFound(did);
        
        attestor.name = name;
        attestor.country = country;
        attestor.category = category;
        
        emit AttestorUpdated(did);
    }

    /**
     * @notice Check if a DID is a verified attestor
     * @param did DID to check
     * @return isAttestor True if exists
     * @return attestor Attestor struct
     */
    function isAttestor(bytes32 did) external view returns (bool isAttestor, Attestor memory attestor) {
        attestor = attestors[did];
        isAttestor = attestor.addedAt != 0;
    }

    /**
     * @notice Get attestor details
     * @param did Attestor DID
     * @return Attestor struct
     */
    function getAttestor(bytes32 did) external view returns (Attestor memory) {
        if (attestors[did].addedAt == 0) revert AttestorNotFound(did);
        return attestors[did];
    }

    /**
     * @notice Get all attestors
     * @return Array of attestor DIDs
     */
    function getAllAttestors() external view returns (bytes32[] memory) {
        return attestorList;
    }

    /**
     * @notice Get attestors by category
     * @param category Category to filter by
     * @return Array of matching attestor DIDs
     */
    function getAttestorsByCategory(string calldata category) external view returns (bytes32[] memory) {
        uint256 count = 0;
        for (uint256 i = 0; i < attestorList.length; i++) {
            if (keccak256(bytes(attestors[attestorList[i]].category)) == keccak256(bytes(category))) {
                count++;
            }
        }
        
        bytes32[] memory result = new bytes32[](count);
        uint256 idx = 0;
        for (uint256 i = 0; i < attestorList.length; i++) {
            if (keccak256(bytes(attestors[attestorList[i]].category)) == keccak256(bytes(category))) {
                result[idx++] = attestorList[i];
            }
        }
        
        return result;
    }

    /**
     * @notice Transfer governance
     * @param newGovernance New governance address
     */
    function transferGovernance(address newGovernance) external onlyGovernance {
        require(newGovernance != address(0), "Invalid address");
        address oldGov = governance;
        governance = newGovernance;
        emit GovernanceTransferred(oldGov, newGovernance);
    }
}


// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title VersionRegistry v1 (ABI FROZEN)
 * @notice Manage schema versions and deprecation policy (10-year stability)
 */
contract VersionRegistry {
    struct SchemaVersion {
        string name;           // e.g., "DIDDoc", "AIIDDoc", "VC"
        string version;        // e.g., "v1", "v2"
        bytes32 schemaCid;     // SVDB CID for schema document
        uint64 activatedAt;
        uint64 deprecatedAt;   // 0 if not deprecated
        uint64 sunsetEpoch;    // When this version will be removed
        bool active;
    }

    mapping(bytes32 => SchemaVersion) public schemas;  // keccak256(name, version) => schema
    mapping(string => bytes32) public activeVersions;  // name => current active version hash
    bytes32[] public schemaList;
    
    address public governance;
    uint32 public minDeprecationWindow = 730 days;  // 24 months minimum

    event SchemaActivated(string indexed name, string version, bytes32 schemaCid);
    event SchemaDeprecated(string indexed name, string version, uint64 sunsetEpoch);
    event ActiveSchemaSet(string indexed name, string version);
    event GovernanceTransferred(address indexed oldGov, address indexed newGov);

    error NotGovernance(address caller);
    error SchemaAlreadyExists(string name, string version);
    error SchemaNotFound(string name, string version);
    error DeprecationWindowTooShort(uint64 provided, uint32 required);
    error SchemaAlreadyDeprecated(string name, string version);

    modifier onlyGovernance() {
        if (msg.sender != governance) revert NotGovernance(msg.sender);
        _;
    }

    constructor() {
        governance = msg.sender;
    }

    /**
     * @notice Activate a new schema version
     * @param name Schema name
     * @param version Version string
     * @param schemaCid SVDB CID for schema document
     */
    function activateSchema(
        string calldata name,
        string calldata version,
        bytes32 schemaCid
    ) external onlyGovernance returns (bytes32) {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        
        if (schemas[schemaHash].activatedAt != 0) {
            revert SchemaAlreadyExists(name, version);
        }
        
        schemas[schemaHash] = SchemaVersion({
            name: name,
            version: version,
            schemaCid: schemaCid,
            activatedAt: uint64(block.timestamp),
            deprecatedAt: 0,
            sunsetEpoch: 0,
            active: true
        });
        
        schemaList.push(schemaHash);
        
        emit SchemaActivated(name, version, schemaCid);
        
        return schemaHash;
    }

    /**
     * @notice Set the active version for a schema name
     * @param name Schema name
     * @param version Version to set as active
     */
    function setActiveSchema(string calldata name, string calldata version) external onlyGovernance {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        
        if (schemas[schemaHash].activatedAt == 0) {
            revert SchemaNotFound(name, version);
        }
        
        activeVersions[name] = schemaHash;
        
        emit ActiveSchemaSet(name, version);
    }

    /**
     * @notice Announce deprecation of a schema version
     * @param name Schema name
     * @param oldVersion Version to deprecate
     * @param sunsetEpoch When this version will be removed (must be >= 24 months)
     */
    function announceDeprecation(
        string calldata name,
        string calldata oldVersion,
        uint64 sunsetEpoch
    ) external onlyGovernance {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, oldVersion));
        SchemaVersion storage schema = schemas[schemaHash];
        
        if (schema.activatedAt == 0) {
            revert SchemaNotFound(name, oldVersion);
        }
        
        if (schema.deprecatedAt != 0) {
            revert SchemaAlreadyDeprecated(name, oldVersion);
        }
        
        uint64 deprecationWindow = sunsetEpoch - uint64(block.timestamp);
        if (deprecationWindow < minDeprecationWindow) {
            revert DeprecationWindowTooShort(deprecationWindow, minDeprecationWindow);
        }
        
        schema.deprecatedAt = uint64(block.timestamp);
        schema.sunsetEpoch = sunsetEpoch;
        
        emit SchemaDeprecated(name, oldVersion, sunsetEpoch);
    }

    /**
     * @notice Get schema details
     * @param name Schema name
     * @param version Schema version
     * @return SchemaVersion struct
     */
    function getSchema(string calldata name, string calldata version) 
        external 
        view 
        returns (SchemaVersion memory) 
    {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        if (schemas[schemaHash].activatedAt == 0) {
            revert SchemaNotFound(name, version);
        }
        return schemas[schemaHash];
    }

    /**
     * @notice Get active schema for a name
     * @param name Schema name
     * @return SchemaVersion struct of active version
     */
    function getActiveSchema(string calldata name) external view returns (SchemaVersion memory) {
        bytes32 schemaHash = activeVersions[name];
        if (schemaHash == bytes32(0)) {
            revert SchemaNotFound(name, "active");
        }
        return schemas[schemaHash];
    }

    /**
     * @notice List all registered schemas
     * @return Array of schema hashes
     */
    function listSchemas() external view returns (bytes32[] memory) {
        return schemaList;
    }

    /**
     * @notice Check if a schema version is deprecated
     * @param name Schema name
     * @param version Schema version
     * @return bool True if deprecated
     */
    function isDeprecated(string calldata name, string calldata version) external view returns (bool) {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        SchemaVersion storage schema = schemas[schemaHash];
        
        if (schema.activatedAt == 0) return false;
        return schema.deprecatedAt != 0;
    }

    /**
     * @notice Check if a schema version is sunset (removed)
     * @param name Schema name
     * @param version Schema version
     * @return bool True if sunset
     */
    function isSunset(string calldata name, string calldata version) external view returns (bool) {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        SchemaVersion storage schema = schemas[schemaHash];
        
        if (schema.activatedAt == 0) return false;
        if (schema.sunsetEpoch == 0) return false;
        
        return block.timestamp >= schema.sunsetEpoch;
    }

    /**
     * @notice Get deprecation info for a schema
     * @param name Schema name
     * @param version Schema version
     * @return deprecatedAt Deprecation timestamp (0 if not deprecated)
     * @return sunsetEpoch Sunset timestamp (0 if not set)
     * @return remainingDays Days until sunset
     */
    function getDeprecationInfo(string calldata name, string calldata version) 
        external 
        view 
        returns (uint64 deprecatedAt, uint64 sunsetEpoch, uint256 remainingDays) 
    {
        bytes32 schemaHash = keccak256(abi.encodePacked(name, version));
        SchemaVersion storage schema = schemas[schemaHash];
        
        if (schema.activatedAt == 0) {
            revert SchemaNotFound(name, version);
        }
        
        deprecatedAt = schema.deprecatedAt;
        sunsetEpoch = schema.sunsetEpoch;
        
        if (sunsetEpoch > block.timestamp) {
            remainingDays = (sunsetEpoch - block.timestamp) / 1 days;
        } else {
            remainingDays = 0;
        }
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


// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ModelRegistry
/// @notice On-chain registry for models linked to datasets and code hash
contract ModelRegistry {
    struct Model {
        address owner;
        bytes32 modelCidRoot;
        bytes32 datasetCidRoot;
        bytes32 codeHash; // arbitrary code hash (keccak256 or blake3 bridged)
        string version;
        bytes32[] lineage; // list of previous model roots
        uint256 registeredAt;
        uint256 updatedAt;
        bool exists;
    }

    mapping(bytes32 => Model) private models; // key: modelCidRoot

    event ModelRegistered(bytes32 indexed modelCidRoot, address indexed owner, bytes32 datasetCidRoot, string version);
    event ModelUpdated(bytes32 indexed modelCidRoot, string version);
    event LineageUpdated(bytes32 indexed modelCidRoot, bytes32[] lineage);
    event ModelOwnershipTransferred(bytes32 indexed modelCidRoot, address indexed previousOwner, address indexed newOwner);

    function registerModel(
        bytes32 modelCidRoot,
        bytes32 datasetCidRoot,
        bytes32 codeHash,
        string calldata version,
        bytes32[] calldata lineage
    ) external {
        require(modelCidRoot != bytes32(0), "model");
        Model storage m = models[modelCidRoot];
        require(!m.exists, "exists");
        m.owner = msg.sender;
        m.modelCidRoot = modelCidRoot;
        m.datasetCidRoot = datasetCidRoot;
        m.codeHash = codeHash;
        m.version = version;
        for (uint256 i = 0; i < lineage.length; i++) {
            m.lineage.push(lineage[i]);
        }
        m.registeredAt = block.timestamp;
        m.updatedAt = block.timestamp;
        m.exists = true;
        emit ModelRegistered(modelCidRoot, msg.sender, datasetCidRoot, version);
        if (lineage.length > 0) emit LineageUpdated(modelCidRoot, lineage);
    }

    function updateVersion(bytes32 modelCidRoot, string calldata newVersion) external {
        Model storage m = models[modelCidRoot];
        require(m.exists, "notfound");
        require(m.owner == msg.sender, "forbidden");
        m.version = newVersion;
        m.updatedAt = block.timestamp;
        emit ModelUpdated(modelCidRoot, newVersion);
    }

    function setLineage(bytes32 modelCidRoot, bytes32[] calldata lineage) external {
        Model storage m = models[modelCidRoot];
        require(m.exists, "notfound");
        require(m.owner == msg.sender, "forbidden");
        delete m.lineage;
        for (uint256 i = 0; i < lineage.length; i++) {
            m.lineage.push(lineage[i]);
        }
        m.updatedAt = block.timestamp;
        emit LineageUpdated(modelCidRoot, lineage);
    }

    function transferModelOwnership(bytes32 modelCidRoot, address newOwner) external {
        Model storage m = models[modelCidRoot];
        require(m.exists, "notfound");
        require(m.owner == msg.sender, "forbidden");
        require(newOwner != address(0), "zero");
        address prev = m.owner;
        m.owner = newOwner;
        m.updatedAt = block.timestamp;
        emit ModelOwnershipTransferred(modelCidRoot, prev, newOwner);
    }

    function getModel(bytes32 modelCidRoot) external view returns (
        address owner,
        bytes32 datasetCidRoot,
        bytes32 codeHash,
        string memory version,
        bytes32[] memory lineage,
        uint256 registeredAt,
        uint256 updatedAt,
        bool exists
    ) {
        Model storage m = models[modelCidRoot];
        return (m.owner, m.datasetCidRoot, m.codeHash, m.version, m.lineage, m.registeredAt, m.updatedAt, m.exists);
    }
}



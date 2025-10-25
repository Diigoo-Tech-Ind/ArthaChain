// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title DatasetRegistry
/// @notice On-chain registry for datasets referenced by artha:// CIDs (stored as bytes32 roots)
contract DatasetRegistry {
    struct Dataset {
        address owner;
        bytes32 cidRoot; // manifest root or canonical CID root (32 bytes)
        uint64 size;
        string licenseInfo;
        string[] tags;
        uint256 registeredAt;
        uint256 updatedAt;
        bool exists;
    }

    mapping(bytes32 => Dataset) private datasets; // key: cidRoot

    event DatasetRegistered(bytes32 indexed cidRoot, address indexed owner, uint64 size, string licenseInfo);
    event DatasetUpdated(bytes32 indexed cidRoot, string licenseInfo);
    event DatasetTagsUpdated(bytes32 indexed cidRoot, string[] tags);
    event DatasetOwnershipTransferred(bytes32 indexed cidRoot, address indexed previousOwner, address indexed newOwner);

    /// @notice Register a dataset
    function registerDataset(bytes32 cidRoot, uint64 size, string calldata licenseInfo, string[] calldata tags) external {
        require(cidRoot != bytes32(0), "cidRoot");
        Dataset storage d = datasets[cidRoot];
        require(!d.exists, "exists");
        d.owner = msg.sender;
        d.cidRoot = cidRoot;
        d.size = size;
        d.licenseInfo = licenseInfo;
        for (uint256 i = 0; i < tags.length; i++) {
            d.tags.push(tags[i]);
        }
        d.registeredAt = block.timestamp;
        d.updatedAt = block.timestamp;
        d.exists = true;
        emit DatasetRegistered(cidRoot, msg.sender, size, licenseInfo);
        if (tags.length > 0) {
            emit DatasetTagsUpdated(cidRoot, tags);
        }
    }

    /// @notice Update license info; only owner
    function updateLicense(bytes32 cidRoot, string calldata newLicense) external {
        Dataset storage d = datasets[cidRoot];
        require(d.exists, "notfound");
        require(d.owner == msg.sender, "forbidden");
        d.licenseInfo = newLicense;
        d.updatedAt = block.timestamp;
        emit DatasetUpdated(cidRoot, newLicense);
    }

    /// @notice Replace tags; only owner
    function setTags(bytes32 cidRoot, string[] calldata tags) external {
        Dataset storage d = datasets[cidRoot];
        require(d.exists, "notfound");
        require(d.owner == msg.sender, "forbidden");
        delete d.tags;
        for (uint256 i = 0; i < tags.length; i++) {
            d.tags.push(tags[i]);
        }
        d.updatedAt = block.timestamp;
        emit DatasetTagsUpdated(cidRoot, tags);
    }

    /// @notice Transfer ownership of a dataset
    function transferDatasetOwnership(bytes32 cidRoot, address newOwner) external {
        Dataset storage d = datasets[cidRoot];
        require(d.exists, "notfound");
        require(d.owner == msg.sender, "forbidden");
        require(newOwner != address(0), "zero");
        address prev = d.owner;
        d.owner = newOwner;
        d.updatedAt = block.timestamp;
        emit DatasetOwnershipTransferred(cidRoot, prev, newOwner);
    }

    /// @notice Get dataset metadata
    function getDataset(bytes32 cidRoot) external view returns (
        address owner,
        uint64 size,
        string memory licenseInfo,
        string[] memory tags,
        uint256 registeredAt,
        uint256 updatedAt,
        bool exists
    ) {
        Dataset storage d = datasets[cidRoot];
        return (d.owner, d.size, d.licenseInfo, d.tags, d.registeredAt, d.updatedAt, d.exists);
    }
}



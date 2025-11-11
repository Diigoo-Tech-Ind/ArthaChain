// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ModelRegistry
/// @notice On-chain registry for AI models with lineage, checkpoints, and versioning
contract ModelRegistry {
    struct Model {
        bytes32 modelId;
        address owner;
        bytes32 modelCid;        // Initial model weights CID
        string architecture;      // "llama", "gpt", "vit", etc.
        bytes32 baseModelId;     // Parent model (0 if trained from scratch)
        bytes32 datasetId;       // Training dataset
        bytes32 codeHash;        // Hash of training/inference code
        string version;          // Semantic version
        uint64 registeredAt;
        bool active;
        bytes32[] checkpoints;   // Checkpoint CIDs
        bytes32 licenseCid;      // License document CID
    }

    struct Checkpoint {
        bytes32 checkpointCid;
        bytes32 metricsJsonCid;
        uint256 step;
        uint64 createdAt;
    }

    mapping(bytes32 => Model) public models;
    mapping(bytes32 => Checkpoint[]) public modelCheckpoints;
    mapping(bytes32 => bytes32[]) public modelLineage; // modelId -> parent chain
    mapping(address => bytes32[]) public ownerModels;

    event ModelRegistered(
        bytes32 indexed modelId,
        address indexed owner,
        bytes32 modelCid,
        string architecture,
        bytes32 datasetId
    );
    event CheckpointAdded(bytes32 indexed modelId, bytes32 checkpointCid, uint256 step);
    event ModelPublished(bytes32 indexed modelId, bytes32 checkpointCid);
    event ModelDeactivated(bytes32 indexed modelId);

    function register(
        bytes32 modelCid,
        string calldata architecture,
        bytes32 baseModelId,
        bytes32 datasetId,
        bytes32 codeHash,
        string calldata version,
        bytes32 licenseCid
    ) external returns (bytes32) {
        bytes32 modelId = keccak256(abi.encodePacked(
            msg.sender,
            modelCid,
            architecture,
            datasetId,
            block.timestamp
        ));

        require(models[modelId].owner == address(0), "Model already exists");

        models[modelId] = Model({
            modelId: modelId,
            owner: msg.sender,
            modelCid: modelCid,
            architecture: architecture,
            baseModelId: baseModelId,
            datasetId: datasetId,
            codeHash: codeHash,
            version: version,
            registeredAt: uint64(block.timestamp),
            active: true,
            checkpoints: new bytes32[](0),
            licenseCid: licenseCid
        });

        ownerModels[msg.sender].push(modelId);

        // Build lineage chain
        if (baseModelId != bytes32(0)) {
            modelLineage[modelId].push(baseModelId);
            // Copy parent's lineage
            for (uint i = 0; i < modelLineage[baseModelId].length; i++) {
                modelLineage[modelId].push(modelLineage[baseModelId][i]);
            }
        }

        emit ModelRegistered(modelId, msg.sender, modelCid, architecture, datasetId);
        return modelId;
    }

    function addCheckpoint(
        bytes32 modelId,
        bytes32 checkpointCid,
        bytes32 metricsJsonCid,
        uint256 step
    ) external {
        Model storage model = models[modelId];
        require(model.owner == msg.sender, "Not model owner");
        require(model.active, "Model not active");

        model.checkpoints.push(checkpointCid);

        modelCheckpoints[modelId].push(Checkpoint({
            checkpointCid: checkpointCid,
            metricsJsonCid: metricsJsonCid,
            step: step,
            createdAt: uint64(block.timestamp)
        }));

        emit CheckpointAdded(modelId, checkpointCid, step);
    }

    function getModel(bytes32 modelId) external view returns (Model memory) {
        return models[modelId];
    }

    function getCheckpoints(bytes32 modelId) external view returns (Checkpoint[] memory) {
        return modelCheckpoints[modelId];
    }

    function getLineage(bytes32 modelId) external view returns (bytes32[] memory) {
        return modelLineage[modelId];
    }

    function getOwnerModels(address owner) external view returns (bytes32[] memory) {
        return ownerModels[owner];
    }

    function deactivate(bytes32 modelId) external {
        Model storage model = models[modelId];
        require(model.owner == msg.sender, "Not model owner");
        model.active = false;
        emit ModelDeactivated(modelId);
    }
}

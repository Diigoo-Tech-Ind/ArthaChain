// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ISVDBProofManager {
    function verifyMerkleSample(bytes32 root, bytes32 leaf, bytes32[] calldata branch, uint256 index) external view returns (bool);
}

interface ISVDBProofsV2V3 {
    function verifySalted(bytes32 root, bytes32 salt, bytes32 leaf, bytes32[] calldata branch, uint256 index) external view returns (bool);
}

/// @title SVDB Deal Market (v1 flat pricing)
contract DealMarket {
    struct Deal {
        address client;
        bytes32 manifestRoot;
        uint64 size;
        uint32 replicas;
        uint32 months;
        uint256 endowment;
        uint64 startEpoch;
        uint64 lastPayoutEpoch;
        bool active;
    }

    mapping(bytes32 => Deal) public deals; // key: manifestRoot
    ISVDBProofManager public proofManager;
    ISVDBProofsV2V3 public proofsV2;
    uint256 public pricePerGBMonthWei; // base price
    uint64 public epochSeconds = 60; // demo epoch

    event DealCreated(bytes32 indexed root, address indexed client, uint256 endowment);
    event Payout(bytes32 indexed root, address indexed provider, uint256 amount, uint64 epoch);
    event RetrievalPaid(bytes32 indexed root, address indexed provider, uint64 bytesServed, uint256 amount);
    event RetrievalAggregate(bytes32 indexed root, bytes32 merkleRoot, address indexed provider, uint256 amount);
    event RetrievalAggregateProof(bytes32 indexed root, bytes32 merkleRoot, bytes32 leaf, address indexed provider, uint256 amount);

    constructor(address proofManager_, uint256 priceWei) {
        proofManager = ISVDBProofManager(proofManager_);
        pricePerGBMonthWei = priceWei;
    }

    function setProofsV2(address proofs_) external {
        // For demo; in production use Ownable/DAO
        proofsV2 = ISVDBProofsV2V3(proofs_);
    }

    function createDeal(bytes32 manifestRoot, uint64 sizeBytes, uint32 replicas, uint32 months) external payable {
        require(!deals[manifestRoot].active, "exists");
        uint256 gb = (uint256(sizeBytes) + (1<<30)-1) >> 30;
        uint256 endowment = gb * pricePerGBMonthWei * months * replicas;
        require(msg.value >= endowment, "insufficient endowment");
        deals[manifestRoot] = Deal({
            client: msg.sender,
            manifestRoot: manifestRoot,
            size: sizeBytes,
            replicas: replicas,
            months: months,
            endowment: msg.value,
            startEpoch: uint64(block.timestamp),
            lastPayoutEpoch: 0,
            active: true
        });
        emit DealCreated(manifestRoot, msg.sender, msg.value);
    }

    function streamPayout(
        bytes32 manifestRoot,
        bytes32 leaf,
        bytes32[] calldata branch,
        uint256 index
    ) external {
        Deal storage d = deals[manifestRoot];
        require(d.active, "inactive");
        // Verify proof
        require(proofManager.verifyMerkleSample(manifestRoot, leaf, branch, index), "invalid proof");
        // Compute epoch number
        uint64 epoch = uint64((block.timestamp - d.startEpoch) / epochSeconds);
        require(epoch > d.lastPayoutEpoch, "already paid");
        d.lastPayoutEpoch = epoch;
        // Flat stream per epoch: endowment spread equally across months*30*24*60*60/epochSeconds (approx)
        uint64 totalEpochs = uint64((uint256(d.months) * 30 * 24 * 3600) / epochSeconds);
        if (totalEpochs == 0) totalEpochs = 1;
        uint256 amount = d.endowment / totalEpochs;
        if (amount > address(this).balance) amount = address(this).balance;
        (bool ok,) = msg.sender.call{value: amount}("");
        require(ok, "transfer failed");
        emit Payout(manifestRoot, msg.sender, amount, epoch);
    }

    function streamPayoutV2(
        bytes32 manifestRoot,
        bytes32 salt,
        bytes32 leaf,
        bytes32[] calldata branch,
        uint256 index
    ) external {
        Deal storage d = deals[manifestRoot];
        require(d.active, "inactive");
        require(address(proofsV2) != address(0), "proofsV2");
        // Verify salted proof
        require(proofsV2.verifySalted(manifestRoot, salt, leaf, branch, index), "invalid proof");
        // Compute epoch number
        uint64 epoch = uint64((block.timestamp - d.startEpoch) / epochSeconds);
        require(epoch > d.lastPayoutEpoch, "already paid");
        d.lastPayoutEpoch = epoch;
        uint64 totalEpochs = uint64((uint256(d.months) * 30 * 24 * 3600) / epochSeconds);
        if (totalEpochs == 0) totalEpochs = 1;
        uint256 amount = d.endowment / totalEpochs;
        if (amount > address(this).balance) amount = address(this).balance;
        (bool ok,) = msg.sender.call{value: amount}("");
        require(ok, "transfer failed");
        emit Payout(manifestRoot, msg.sender, amount, epoch);
    }

    function streamPayoutV2Batch(
        bytes32[] calldata manifestRoots,
        bytes32[] calldata salts,
        bytes32[] calldata leaves,
        bytes32[][] calldata branches,
        uint256[] calldata indices,
        address[] calldata providers
    ) external {
        require(
            manifestRoots.length == salts.length &&
            manifestRoots.length == leaves.length &&
            manifestRoots.length == branches.length &&
            manifestRoots.length == indices.length &&
            manifestRoots.length == providers.length,
            "len"
        );
        for (uint256 i = 0; i < manifestRoots.length; i++) {
            bytes32 root = manifestRoots[i];
            Deal storage d = deals[root];
            if (!d.active) { continue; }
            if (address(proofsV2) == address(0)) { continue; }
            bool okv = proofsV2.verifySalted(root, salts[i], leaves[i], branches[i], indices[i]);
            if (!okv) { continue; }
            uint64 epoch = uint64((block.timestamp - d.startEpoch) / epochSeconds);
            if (epoch <= d.lastPayoutEpoch) { continue; }
            d.lastPayoutEpoch = epoch;
            uint64 totalEpochs = uint64((uint256(d.months) * 30 * 24 * 3600) / epochSeconds);
            if (totalEpochs == 0) totalEpochs = 1;
            uint256 amount = d.endowment / totalEpochs;
            if (amount > address(this).balance) amount = address(this).balance;
            (bool ok,) = providers[i].call{value: amount}("");
            if (ok) {
                emit Payout(root, providers[i], amount, epoch);
            }
        }
    }

    /// @notice Record retrieval micro-fee payment to a provider. Must send ETH equal to fee.
    function recordRetrieval(bytes32 manifestRoot, uint64 bytesServed, address payable provider) external payable {
        Deal storage d = deals[manifestRoot];
        require(d.active, "inactive");
        require(provider != address(0), "provider");
        // Forward entire msg.value to provider as micro-fee settlement
        (bool ok,) = provider.call{value: msg.value}("");
        require(ok, "pay");
        emit RetrievalPaid(manifestRoot, provider, bytesServed, msg.value);
    }

    /// @notice Aggregate settlement using a Merkle root (off-chain voucher aggregation)
    function recordRetrievalAggregate(bytes32 manifestRoot, bytes32 merkleRoot, address payable provider) external payable {
        Deal storage d = deals[manifestRoot];
        require(d.active, "inactive");
        require(provider != address(0), "provider");
        (bool ok,) = provider.call{value: msg.value}("");
        require(ok, "pay");
        emit RetrievalAggregate(manifestRoot, merkleRoot, provider, msg.value);
    }

    /// @notice Aggregate settlement with proof-of-leaf under merkleRoot
    function recordRetrievalAggregateProof(
        bytes32 manifestRoot,
        bytes32 merkleRoot,
        bytes32 leaf,
        bytes32[] calldata branch,
        uint256 index,
        address payable provider
    ) external payable {
        Deal storage d = deals[manifestRoot];
        require(d.active, "inactive");
        require(provider != address(0), "provider");
        // Verify leaf inclusion under merkleRoot (keccak over 32-byte nodes)
        bytes32 acc = leaf;
        uint256 idx = index;
        for (uint256 i = 0; i < branch.length; i++) {
            bytes32 sib = branch[i];
            acc = (idx % 2 == 0) ? keccak256(abi.encodePacked(acc, sib)) : keccak256(abi.encodePacked(sib, acc));
            idx >>= 1;
        }
        require(acc == merkleRoot, "bad leaf");
        (bool ok,) = provider.call{value: msg.value}("");
        require(ok, "pay");
        emit RetrievalAggregateProof(manifestRoot, merkleRoot, leaf, provider, msg.value);
    }
}



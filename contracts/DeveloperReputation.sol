// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Developer Reputation (minimal)
/// @notice Tracks developer contribution signals and derives a simple reputation score
contract DeveloperReputation {
    struct Dev {
        uint64 tasksCompleted;
        uint64 modelsPublished;
        uint64 slashFreeEpochs;
        uint64 slashCount;
        uint256 lastUpdated;
    }

    mapping(address => Dev) private devs;
    address public owner;

    modifier onlyOwner() {
        require(msg.sender == owner, "owner");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    event TaskCompleted(address indexed dev);
    event ModelPublished(address indexed dev);
    event SlashRecorded(address indexed dev);
    event SlashFreeEpochRecorded(address indexed dev);

    function get(address developer)
        external
        view
        returns (uint64 tasksCompleted, uint64 modelsPublished, uint64 slashFreeEpochs, uint64 slashCount, uint256 lastUpdated)
    {
        Dev memory d = devs[developer];
        return (d.tasksCompleted, d.modelsPublished, d.slashFreeEpochs, d.slashCount, d.lastUpdated);
    }

    /// @notice Derive a reputation score in basis points [0, 10000]
    function reputationScore(address developer) public view returns (uint256) {
        Dev memory d = devs[developer];
        // Simple weighted sum with diminishing penalty for few slashes
        // score = min(10000, 100*d.tasks + 200*d.models + 50*d.epochs - 500*d.slashes)
        uint256 positive = uint256(d.tasksCompleted) * 100
            + uint256(d.modelsPublished) * 200
            + uint256(d.slashFreeEpochs) * 50;
        uint256 penalty = uint256(d.slashCount) * 500;
        if (penalty > positive) {
            return 0;
        }
        uint256 raw = positive - penalty;
        if (raw > 10000) raw = 10000;
        return raw;
    }

    function recordTaskCompleted(address developer) external onlyOwner {
        Dev storage d = devs[developer];
        unchecked { d.tasksCompleted += 1; }
        d.lastUpdated = block.timestamp;
        emit TaskCompleted(developer);
    }

    function recordModelPublished(address developer) external onlyOwner {
        Dev storage d = devs[developer];
        unchecked { d.modelsPublished += 1; }
        d.lastUpdated = block.timestamp;
        emit ModelPublished(developer);
    }

    function recordSlash(address developer) external onlyOwner {
        Dev storage d = devs[developer];
        unchecked { d.slashCount += 1; }
        d.lastUpdated = block.timestamp;
        emit SlashRecorded(developer);
    }

    function recordSlashFreeEpoch(address developer) external onlyOwner {
        Dev storage d = devs[developer];
        unchecked { d.slashFreeEpochs += 1; }
        d.lastUpdated = block.timestamp;
        emit SlashFreeEpochRecorded(developer);
    }
}



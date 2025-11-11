// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ReputationRegistry
/// @notice Minimal developer reputation and DID primitives
contract ReputationRegistry {
    struct Developer {
        uint256 reputation;        // non-negative, capped
        uint256 lineageContribs;   // number of model/dataset lineage contributions
        uint256 slashFreeEpochs;   // epochs (time windows) without slashes
        string  did;               // optional DID string
    }

    address public admin;
    mapping(address => bool) public updaters; // accounts allowed to adjust reputation
    mapping(address => Developer) public developers;

    uint256 public constant MAX_REPUTATION = 1_000_000;

    event AdminChanged(address indexed admin);
    event UpdaterSet(address indexed updater, bool allowed);
    event DeveloperUpdated(address indexed dev, uint256 reputation, uint256 lineage, uint256 slashFreeEpochs);
    event DidSet(address indexed dev, string did);

    modifier onlyAdmin() { require(msg.sender == admin, "admin"); _; }
    modifier onlyUpdater() { require(updaters[msg.sender] || msg.sender == admin, "updater"); _; }

    constructor(address _admin) { require(_admin != address(0), "zero"); admin = _admin; emit AdminChanged(_admin); }

    function setUpdater(address who, bool allowed) external onlyAdmin { updaters[who] = allowed; emit UpdaterSet(who, allowed); }
    function setAdmin(address who) external onlyAdmin { require(who != address(0), "zero"); admin = who; emit AdminChanged(who); }

    function addReputation(address dev, uint256 delta) external onlyUpdater {
        Developer storage d = developers[dev];
        uint256 nr = d.reputation + delta;
        if (nr > MAX_REPUTATION) nr = MAX_REPUTATION;
        d.reputation = nr;
        emit DeveloperUpdated(dev, d.reputation, d.lineageContribs, d.slashFreeEpochs);
    }

    function subReputation(address dev, uint256 delta) external onlyUpdater {
        Developer storage d = developers[dev];
        if (delta > d.reputation) d.reputation = 0; else d.reputation -= delta;
        emit DeveloperUpdated(dev, d.reputation, d.lineageContribs, d.slashFreeEpochs);
    }

    function recordLineageContribution(address dev) external onlyUpdater {
        Developer storage d = developers[dev];
        d.lineageContribs += 1;
        emit DeveloperUpdated(dev, d.reputation, d.lineageContribs, d.slashFreeEpochs);
    }

    function recordSlashFreeEpoch(address dev) external onlyUpdater {
        Developer storage d = developers[dev];
        d.slashFreeEpochs += 1;
        emit DeveloperUpdated(dev, d.reputation, d.lineageContribs, d.slashFreeEpochs);
    }

    function setDID(address dev, string calldata did) external onlyUpdater {
        developers[dev].did = did;
        emit DidSet(dev, did);
    }
}



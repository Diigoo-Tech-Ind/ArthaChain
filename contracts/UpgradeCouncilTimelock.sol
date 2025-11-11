// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Minimal Multisig + Timelock (T+48h)
contract UpgradeCouncilTimelock {
    uint256 public constant MIN_DELAY = 48 hours;

    address[] public signers;
    uint256 public threshold; // number of required signatures

    struct Proposal {
        address target;
        uint256 value;
        bytes data;
        uint256 eta;
        uint256 approvals;
        bool executed;
        mapping(address => bool) approvedBy;
    }

    mapping(bytes32 => Proposal) private proposals;

    event Proposed(bytes32 indexed id, address indexed target, uint256 value, uint256 eta);
    event Approved(bytes32 indexed id, address indexed signer, uint256 approvals);
    event Executed(bytes32 indexed id);

    constructor(address[] memory _signers, uint256 _threshold) {
        require(_signers.length >= _threshold && _threshold > 0, "cfg");
        signers = _signers;
        threshold = _threshold;
    }

    function isSigner(address a) public view returns (bool) {
        for (uint256 i = 0; i < signers.length; i++) if (signers[i] == a) return true;
        return false;
    }

    function propose(address target, uint256 value, bytes calldata data) external returns (bytes32 id) {
        require(isSigner(msg.sender), "auth");
        uint256 eta = block.timestamp + MIN_DELAY;
        id = keccak256(abi.encode(target, value, data, eta));
        Proposal storage p = proposals[id];
        require(p.eta == 0, "exists");
        p.target = target;
        p.value = value;
        p.data = data;
        p.eta = eta;
        emit Proposed(id, target, value, eta);
        _approve(id);
    }

    function approve(bytes32 id) external {
        require(isSigner(msg.sender), "auth");
        _approve(id);
    }

    function _approve(bytes32 id) internal {
        Proposal storage p = proposals[id];
        require(p.eta != 0, "no prop");
        require(!p.approvedBy[msg.sender], "dup");
        p.approvedBy[msg.sender] = true;
        unchecked { p.approvals += 1; }
        emit Approved(id, msg.sender, p.approvals);
    }

    function execute(bytes32 id) external payable {
        Proposal storage p = proposals[id];
        require(p.eta != 0 && !p.executed, "bad");
        require(block.timestamp >= p.eta, "delay");
        require(p.approvals >= threshold, "quorum");
        p.executed = true;
        (bool ok, ) = p.target.call{value: p.value}(p.data);
        require(ok, "exec");
        emit Executed(id);
    }
}



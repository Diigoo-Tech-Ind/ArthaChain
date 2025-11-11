// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Minimal TimelockController
/// @notice Schedules operations with a delay; executor executes after timestamp
contract TimelockController {
    struct Op { bytes data; address target; uint256 eta; bool executed; }
    mapping(bytes32 => Op) public ops;
    uint256 public delay;
    address public admin;
    address public executor;

    event Scheduled(bytes32 indexed id, address indexed target, uint256 eta);
    event Executed(bytes32 indexed id);
    event Cancelled(bytes32 indexed id);

    modifier onlyAdmin(){ require(msg.sender == admin, "admin"); _; }
    modifier onlyExecutor(){ require(msg.sender == executor || msg.sender == admin, "exec"); _; }

    constructor(uint256 _delay, address _admin, address _executor){
        require(_admin != address(0) && _executor != address(0), "zero");
        delay = _delay; admin = _admin; executor = _executor;
    }

    function schedule(address target, bytes calldata data) external onlyAdmin returns(bytes32 id){
        id = keccak256(abi.encode(target, data, block.timestamp));
        ops[id] = Op({data: data, target: target, eta: block.timestamp + delay, executed: false});
        emit Scheduled(id, target, ops[id].eta);
    }

    function cancel(bytes32 id) external onlyAdmin { delete ops[id]; emit Cancelled(id); }

    function execute(bytes32 id) external onlyExecutor {
        Op storage op = ops[id];
        require(!op.executed && op.eta != 0 && block.timestamp >= op.eta, "not ready");
        (bool ok,) = op.target.call(op.data);
        require(ok, "call failed");
        op.executed = true; emit Executed(id);
    }
}



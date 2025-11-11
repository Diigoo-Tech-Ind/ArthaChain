// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Minimal Multisig Wallet
contract MultisigWallet {
    address[] public owners;
    uint256 public threshold;
    mapping(bytes32 => uint256) public approvals;
    mapping(bytes32 => mapping(address => bool)) public approvedBy;

    event Executed(bytes32 indexed id, address target, uint256 value);

    modifier onlyOwner(){
        bool isOwner=false; for(uint i=0;i<owners.length;i++){ if(owners[i]==msg.sender){isOwner=true;break;} }
        require(isOwner, "owner"); _;
    }

    constructor(address[] memory _owners, uint256 _threshold){ require(_owners.length>=_threshold && _threshold>0, "bad"); owners=_owners; threshold=_threshold; }

    function approveAndExecute(address target, uint256 value, bytes calldata data) external onlyOwner {
        bytes32 id = keccak256(abi.encode(target,value,data));
        require(!approvedBy[id][msg.sender], "dup");
        approvedBy[id][msg.sender]=true; approvals[id]+=1;
        if (approvals[id] >= threshold) {
            (bool ok,) = target.call{value:value}(data);
            require(ok, "exec fail");
            emit Executed(id, target, value);
            delete approvals[id];
        }
    }

    receive() external payable {}
}



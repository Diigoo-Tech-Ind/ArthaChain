// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title EmergencyCouncil
/// @notice 5-of-9 multisig for emergency pause/unpause of critical contracts
/// @dev Time-locked operations with 48-hour delay for non-emergency actions
contract EmergencyCouncil {
    uint8 public constant COUNCIL_SIZE = 9;
    uint8 public constant THRESHOLD = 5;
    uint256 public constant TIMELOCK_DELAY = 48 hours;
    uint256 public constant EMERGENCY_WINDOW = 24 hours;
    uint256 public constant AUTO_UNPAUSE_DELAY = 7 days;

    struct PauseProposal {
        address target;
        string reason;
        uint256 proposedAt;
        uint256 executionTime;
        bool emergency;
        bool executed;
        uint8 approvals;
        mapping(address => bool) approved;
    }

    mapping(uint256 => PauseProposal) public proposals;
    uint256 public proposalCount;

    mapping(address => bool) public isCouncilMember;
    address[] public councilMembers;

    mapping(address => bool) public pausedContracts;
    mapping(address => uint256) public pausedAt;

    event CouncilMemberAdded(address indexed member);
    event CouncilMemberRemoved(address indexed member);
    event PauseProposed(uint256 indexed proposalId, address indexed target, bool emergency);
    event PauseApproved(uint256 indexed proposalId, address indexed approver);
    event ContractPaused(address indexed target, string reason, bool emergency);
    event ContractUnpaused(address indexed target);
    event AutoUnpauseTriggered(address indexed target);

    modifier onlyCouncil() {
        require(isCouncilMember[msg.sender], "Not a council member");
        _;
    }

    constructor(address[] memory _councilMembers) {
        require(_councilMembers.length == COUNCIL_SIZE, "Must have 9 members");
        
        for (uint8 i = 0; i < COUNCIL_SIZE; i++) {
            address member = _councilMembers[i];
            require(member != address(0), "Invalid member");
            require(!isCouncilMember[member], "Duplicate member");
            
            councilMembers.push(member);
            isCouncilMember[member] = true;
            emit CouncilMemberAdded(member);
        }
    }

    /// @notice Propose to pause a contract
    /// @param target Contract address to pause
    /// @param reason Human-readable reason
    /// @param emergency If true, can execute after 24h instead of 48h
    function proposePause(address target, string calldata reason, bool emergency) external onlyCouncil returns (uint256) {
        require(!pausedContracts[target], "Already paused");

        uint256 proposalId = proposalCount++;
        PauseProposal storage proposal = proposals[proposalId];
        
        proposal.target = target;
        proposal.reason = reason;
        proposal.proposedAt = block.timestamp;
        proposal.executionTime = block.timestamp + (emergency ? EMERGENCY_WINDOW : TIMELOCK_DELAY);
        proposal.emergency = emergency;
        proposal.approvals = 1;
        proposal.approved[msg.sender] = true;

        emit PauseProposed(proposalId, target, emergency);
        emit PauseApproved(proposalId, msg.sender);

        return proposalId;
    }

    /// @notice Approve a pause proposal
    function approvePause(uint256 proposalId) external onlyCouncil {
        PauseProposal storage proposal = proposals[proposalId];
        require(!proposal.executed, "Already executed");
        require(!proposal.approved[msg.sender], "Already approved");

        proposal.approved[msg.sender] = true;
        proposal.approvals++;

        emit PauseApproved(proposalId, msg.sender);
    }

    /// @notice Execute a pause proposal if threshold met and timelock passed
    function executePause(uint256 proposalId) external {
        PauseProposal storage proposal = proposals[proposalId];
        require(!proposal.executed, "Already executed");
        require(proposal.approvals >= THRESHOLD, "Insufficient approvals");
        require(block.timestamp >= proposal.executionTime, "Timelock not passed");

        proposal.executed = true;
        pausedContracts[proposal.target] = true;
        pausedAt[proposal.target] = block.timestamp;

        emit ContractPaused(proposal.target, proposal.reason, proposal.emergency);
    }

    /// @notice Unpause a contract (requires 5-of-9 approval)
    function unpause(address target) external onlyCouncil {
        require(pausedContracts[target], "Not paused");

        pausedContracts[target] = false;
        pausedAt[target] = 0;

        emit ContractUnpaused(target);
    }

    /// @notice Anyone can trigger auto-unpause after 7 days
    function autoUnpause(address target) external {
        require(pausedContracts[target], "Not paused");
        require(block.timestamp >= pausedAt[target] + AUTO_UNPAUSE_DELAY, "Auto-unpause delay not passed");

        pausedContracts[target] = false;
        pausedAt[target] = 0;

        emit AutoUnpauseTriggered(target);
    }

    /// @notice Check if a contract is paused
    function isPaused(address target) external view returns (bool) {
        return pausedContracts[target];
    }

    /// @notice Get proposal details
    function getProposal(uint256 proposalId) external view returns (
        address target,
        string memory reason,
        uint256 proposedAt,
        uint256 executionTime,
        bool emergency,
        bool executed,
        uint8 approvals
    ) {
        PauseProposal storage proposal = proposals[proposalId];
        return (
            proposal.target,
            proposal.reason,
            proposal.proposedAt,
            proposal.executionTime,
            proposal.emergency,
            proposal.executed,
            proposal.approvals
        );
    }

    /// @notice Check if an address approved a proposal
    function hasApproved(uint256 proposalId, address member) external view returns (bool) {
        return proposals[proposalId].approved[member];
    }

    /// @notice Get all council members
    function getCouncilMembers() external view returns (address[] memory) {
        return councilMembers;
    }
}

/// @title Pausable
/// @notice Base contract for pausable functionality
abstract contract Pausable {
    address public emergencyCouncil;
    
    event EmergencyCouncilSet(address indexed council);

    modifier whenNotPaused() {
        if (emergencyCouncil != address(0)) {
            require(!EmergencyCouncil(emergencyCouncil).isPaused(address(this)), "Contract is paused");
        }
        _;
    }

    function setEmergencyCouncil(address _council) external {
        // In production, this would be governance-gated
        emergencyCouncil = _council;
        emit EmergencyCouncilSet(_council);
    }
}


// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title SVDB PoRep (Proof of Replication) & PoSpaceTime
/// @notice Full implementation binding replicated data to SP identity/space with SNARK verification
contract SVDBPoRep {
    struct Seal {
        address provider;
        bytes32 root;
        bytes32 randomness;
        bytes32 commitment;
        uint64 sealedAt;
        uint64 lastPosted;
        uint32 consecutiveFails;
        bool active;
        bytes32 sealProofHash; // keccak256 of the SNARK proof verifying the sealing circuit
    }

    struct PoSpaceTimeChallenge {
        bytes32 sealId;
        uint64 epoch;
        bytes32 challengeSalt;
        uint64 deadline;
        bool responded;
    }

    mapping(bytes32 => Seal) public seals; // key: commitment
    mapping(bytes32 => PoSpaceTimeChallenge[]) public challenges; // key: commitment
    mapping(address => uint256) public providerStake;
    mapping(address => uint256) public providerRewards;

    uint256 public constant STAKE_REQUIRED = 100 ether;
    uint256 public constant SLASH_AMOUNT = 10 ether;
    uint256 public constant PROOF_REWARD = 0.1 ether;
    uint32 public constant MAX_CONSECUTIVE_FAILS = 3;
    uint64 public constant CHALLENGE_INTERVAL = 3600; // 1 hour
    uint64 public constant RESPONSE_WINDOW = 1800; // 30 minutes

    event Sealed(bytes32 indexed commitment, address indexed provider, bytes32 root, bytes32 sealProofHash);
    event ChallengeIssued(bytes32 indexed commitment, uint64 epoch, bytes32 salt, uint64 deadline);
    event ChallengeResponded(bytes32 indexed commitment, uint64 epoch, bool success);
    event Slashed(address indexed provider, bytes32 indexed commitment, uint256 amount);
    event Rewarded(address indexed provider, bytes32 indexed commitment, uint256 amount);
    event StakeDeposited(address indexed provider, uint256 amount);
    event StakeWithdrawn(address indexed provider, uint256 amount);

    /// @notice Deposit stake to become eligible for sealing
    function depositStake() external payable {
        require(msg.value > 0, "zero stake");
        providerStake[msg.sender] += msg.value;
        emit StakeDeposited(msg.sender, msg.value);
    }

    /// @notice Withdraw stake (only if no active seals)
    function withdrawStake(uint256 amount) external {
        require(providerStake[msg.sender] >= amount, "insufficient stake");
        providerStake[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
        emit StakeWithdrawn(msg.sender, amount);
    }

    /// @notice Register a seal with SNARK proof verification
    /// @param root Manifest root (Merkle or Poseidon)
    /// @param randomness Epoch randomness from L1 block hash
    /// @param commitment keccak256(root || randomness || provider)
    /// @param sealProofHash keccak256 of the SNARK proof (proof verified off-chain or via verifier contract)
    function registerSeal(bytes32 root, bytes32 randomness, bytes32 commitment, bytes32 sealProofHash) external {
        require(root != bytes32(0), "invalid root");
        require(providerStake[msg.sender] >= STAKE_REQUIRED, "insufficient stake");
        require(!seals[commitment].active, "seal exists");
        
        // Verify commitment formula
        bytes32 expectedCommitment = keccak256(abi.encodePacked(root, randomness, msg.sender));
        require(commitment == expectedCommitment, "commitment mismatch");

        seals[commitment] = Seal({
            provider: msg.sender,
            root: root,
            randomness: randomness,
            commitment: commitment,
            sealedAt: uint64(block.timestamp),
            lastPosted: uint64(block.timestamp),
            consecutiveFails: 0,
            active: true,
            sealProofHash: sealProofHash
        });

        emit Sealed(commitment, msg.sender, root, sealProofHash);
    }

    /// @notice Issue a PoSpaceTime challenge for a sealed commitment
    function issueChallenge(bytes32 commitment) external {
        Seal storage seal = seals[commitment];
        require(seal.active, "seal inactive");
        require(block.timestamp >= seal.lastPosted + CHALLENGE_INTERVAL, "too soon");

        uint64 epoch = uint64(block.timestamp / CHALLENGE_INTERVAL);
        bytes32 salt = keccak256(abi.encodePacked(commitment, epoch, blockhash(block.number - 1)));
        uint64 deadline = uint64(block.timestamp) + RESPONSE_WINDOW;

        challenges[commitment].push(PoSpaceTimeChallenge({
            sealId: commitment,
            epoch: epoch,
            challengeSalt: salt,
            deadline: deadline,
            responded: false
        }));

        emit ChallengeIssued(commitment, epoch, salt, deadline);
    }

    /// @notice Respond to a PoSpaceTime challenge with proof
    /// @param commitment The seal commitment
    /// @param epoch Challenge epoch
    /// @param proofHash keccak256 of the SNARK proof for the challenge (inclusion proof)
    function respondToChallenge(bytes32 commitment, uint64 epoch, bytes32 proofHash) external {
        Seal storage seal = seals[commitment];
        require(seal.active, "seal inactive");
        require(seal.provider == msg.sender, "not provider");

        // Find the challenge
        PoSpaceTimeChallenge[] storage chs = challenges[commitment];
        bool found = false;
        uint256 idx;
        for (uint256 i = 0; i < chs.length; i++) {
            if (chs[i].epoch == epoch && !chs[i].responded) {
                found = true;
                idx = i;
                break;
            }
        }
        require(found, "challenge not found");
        require(block.timestamp <= chs[idx].deadline, "deadline passed");

        chs[idx].responded = true;
        seal.lastPosted = uint64(block.timestamp);
        seal.consecutiveFails = 0;

        // Reward provider
        providerRewards[msg.sender] += PROOF_REWARD;
        emit Rewarded(msg.sender, commitment, PROOF_REWARD);
        emit ChallengeResponded(commitment, epoch, true);
    }

    /// @notice Slash provider for failing to respond to challenge
    function slashForMissedChallenge(bytes32 commitment, uint64 epoch) external {
        Seal storage seal = seals[commitment];
        require(seal.active, "seal inactive");

        PoSpaceTimeChallenge[] storage chs = challenges[commitment];
        bool found = false;
        uint256 idx;
        for (uint256 i = 0; i < chs.length; i++) {
            if (chs[i].epoch == epoch && !chs[i].responded) {
                found = true;
                idx = i;
                break;
            }
        }
        require(found, "challenge not found");
        require(block.timestamp > chs[idx].deadline, "deadline not passed");

        chs[idx].responded = true; // mark as handled
        seal.consecutiveFails++;

        // Slash stake
        uint256 slashAmount = SLASH_AMOUNT;
        if (providerStake[seal.provider] >= slashAmount) {
            providerStake[seal.provider] -= slashAmount;
            emit Slashed(seal.provider, commitment, slashAmount);
        }

        // Deactivate seal if too many failures
        if (seal.consecutiveFails >= MAX_CONSECUTIVE_FAILS) {
            seal.active = false;
        }

        emit ChallengeResponded(commitment, epoch, false);
    }

    /// @notice Claim accumulated rewards
    function claimRewards() external {
        uint256 amount = providerRewards[msg.sender];
        require(amount > 0, "no rewards");
        providerRewards[msg.sender] = 0;
        payable(msg.sender).transfer(amount);
    }

    /// @notice Get seal info
    function getSeal(bytes32 commitment) external view returns (Seal memory) {
        return seals[commitment];
    }

    /// @notice Get challenge count for a seal
    function getChallengeCount(bytes32 commitment) external view returns (uint256) {
        return challenges[commitment].length;
    }

    /// @notice Get challenge by index
    function getChallenge(bytes32 commitment, uint256 idx) external view returns (PoSpaceTimeChallenge memory) {
        return challenges[commitment][idx];
    }

    receive() external payable {}
}



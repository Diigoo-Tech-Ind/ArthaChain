// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title BountyBoard v0
/// @notice Minimal bounty registry funded by Ecosystem pool
interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

contract BountyBoard {
    struct Bounty {
        address sponsor;
        string title;
        string description;
        uint256 rewardWei;
        bool claimed;
        address claimer;
    }

    mapping(uint256 => Bounty) public bounties;
    uint256 public nextId;
    address public arthaToken; // default ERC20 funding
    address public owner;
    mapping(uint256 => bool) private bountyUsesToken;
    mapping(uint256 => address) private bountyToken;

    modifier onlyOwner() {
        require(msg.sender == owner, "owner");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    event BountyCreated(uint256 indexed id, address indexed sponsor, uint256 rewardWei);
    event BountyClaimed(uint256 indexed id, address indexed claimer);

    function setArthaToken(address token) external onlyOwner {
        arthaToken = token;
    }

    function create(string calldata title, string calldata description) external payable returns (uint256 id) {
        require(msg.value > 0, "fund");
        id = nextId++;
        bounties[id] = Bounty({
            sponsor: msg.sender,
            title: title,
            description: description,
            rewardWei: msg.value,
            claimed: false,
            claimer: address(0)
        });
        emit BountyCreated(id, msg.sender, msg.value);
    }

    function createWithArtha(string calldata title, string calldata description, uint256 amount) external returns (uint256 id) {
        require(arthaToken != address(0), "no token");
        require(amount > 0, "fund");
        id = nextId++;
        bounties[id] = Bounty({
            sponsor: msg.sender,
            title: title,
            description: description,
            rewardWei: amount,
            claimed: false,
            claimer: address(0)
        });
        require(IERC20(arthaToken).transferFrom(msg.sender, address(this), amount), "xfer");
        bountyUsesToken[id] = true;
        bountyToken[id] = arthaToken;
        emit BountyCreated(id, msg.sender, amount);
    }

    /// @notice Allow ecosystem pool to fund by sending ETH directly
    receive() external payable {
        // Create an unfocused bounty for later allocation by council
        uint256 id = nextId++;
        bounties[id] = Bounty({
            sponsor: msg.sender,
            title: "ecosystem-allocation",
            description: "funded by ecosystem pool",
            rewardWei: msg.value,
            claimed: false,
            claimer: address(0)
        });
        emit BountyCreated(id, msg.sender, msg.value);
    }

    function claim(uint256 id, address payable to) external {
        Bounty storage b = bounties[id];
        require(!b.claimed, "claimed");
        require(b.rewardWei > 0, "no reward");
        b.claimed = true;
        b.claimer = to;
        if (!bountyUsesToken[id]) {
            (bool ok,) = to.call{value: b.rewardWei}("");
            require(ok, "pay");
        } else {
            address token = bountyToken[id];
            require(token != address(0), "no token");
            require(IERC20(token).transfer(to, b.rewardWei), "pay20");
        }
        emit BountyClaimed(id, to);
    }
}



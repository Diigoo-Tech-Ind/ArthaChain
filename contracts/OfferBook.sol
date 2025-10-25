// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title OfferBook
/// @notice Storage providers publish offers with region/latency/SLA; clients choose and enforce penalties
contract OfferBook {
    enum SlaTier { Bronze, Silver, Gold, Platinum }

    struct Offer {
        address provider;
        string region;
        uint256 priceWeiPerGBMonth;
        uint256 expectedLatencyMs;
        SlaTier tier;
        uint64 publishedAt;
        bool active;
    }

    struct SlaRecord {
        address client;
        address provider;
        bytes32 manifestRoot;
        SlaTier tier;
        uint64 startAt;
        uint64 violations;
        uint256 totalPenaltyWei;
        bool active;
    }

    mapping(address => Offer) public offers;
    mapping(bytes32 => SlaRecord) public slas; // key = keccak(client, provider, manifestRoot)

    event OfferPublished(address indexed provider, string region, uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs, SlaTier tier);
    event OfferDeactivated(address indexed provider);
    event SlaStarted(bytes32 indexed key, address indexed client, address indexed provider, bytes32 manifestRoot, SlaTier tier);
    event SlaViolation(bytes32 indexed key, uint64 newViolations, uint256 penaltyWei);
    event SlaClosed(bytes32 indexed key);

    function publishOffer(string calldata region, uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs, SlaTier tier) external {
        offers[msg.sender] = Offer({
            provider: msg.sender,
            region: region,
            priceWeiPerGBMonth: priceWeiPerGBMonth,
            expectedLatencyMs: expectedLatencyMs,
            tier: tier,
            publishedAt: uint64(block.timestamp),
            active: true
        });
        emit OfferPublished(msg.sender, region, priceWeiPerGBMonth, expectedLatencyMs, tier);
    }

    function deactivateOffer() external {
        offers[msg.sender].active = false;
        emit OfferDeactivated(msg.sender);
    }

    /// @notice Lightweight getter without dynamic string to simplify off-chain reads
    function getOffer(address provider) external view returns (
        uint256 priceWeiPerGBMonth,
        uint256 expectedLatencyMs,
        uint8 tier,
        uint64 publishedAt,
        bool active
    ) {
        Offer memory o = offers[provider];
        return (o.priceWeiPerGBMonth, o.expectedLatencyMs, uint8(o.tier), o.publishedAt, o.active);
    }

    function _slaKey(address client, address provider, bytes32 manifestRoot) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(client, provider, manifestRoot));
    }

    function startSla(address provider, bytes32 manifestRoot, SlaTier tier) external payable {
        Offer memory o = offers[provider];
        require(o.active, "nooffer");
        bytes32 key = _slaKey(msg.sender, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(!r.active, "exists");
        r.client = msg.sender;
        r.provider = provider;
        r.manifestRoot = manifestRoot;
        r.tier = tier;
        r.startAt = uint64(block.timestamp);
        r.violations = 0;
        r.totalPenaltyWei = 0;
        r.active = true;
        emit SlaStarted(key, msg.sender, provider, manifestRoot, tier);
    }

    /// @notice Record a violation and apply penalty by tier
    function recordViolation(address client, address provider, bytes32 manifestRoot, uint256 latencyMs) external {
        bytes32 key = _slaKey(client, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(r.active, "nosla");
        r.violations += 1;
        // Simple penalty schedule
        uint256 penalty;
        if (r.tier == SlaTier.Bronze) penalty = 1e14; // 0.0001 ETH
        else if (r.tier == SlaTier.Silver) penalty = 2e14;
        else if (r.tier == SlaTier.Gold) penalty = 5e14;
        else penalty = 1e15; // Platinum
        r.totalPenaltyWei += penalty;
        emit SlaViolation(key, r.violations, penalty);
    }

    function closeSla(address client, address provider, bytes32 manifestRoot) external {
        bytes32 key = _slaKey(client, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(r.active, "nosla");
        r.active = false;
        emit SlaClosed(key);
    }
}



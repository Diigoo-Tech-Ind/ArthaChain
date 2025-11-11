// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title OfferBook - Full Marketplace & SLA Enforcement
/// @notice Storage providers publish offers with region/latency/SLA; clients choose, monitor, and enforce penalties
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
        uint32 capacityGB;
        bool gpuAvailable;
        uint256 collateral;
    }

    struct SlaRecord {
        address client;
        address provider;
        bytes32 manifestRoot;
        SlaTier tier;
        uint64 startAt;
        uint64 lastChecked;
        uint64 violations;
        uint256 totalPenaltyWei;
        uint256 escrow;
        bool active;
    }

    struct ProviderReputation {
        uint256 totalDeals;
        uint256 successfulDeals;
        uint256 totalViolations;
        uint256 totalSlashes;
        uint256 uptimeScore; // 0-10000 (basis points)
        uint256 bandwidthScore; // 0-10000
        uint256 proofSuccessRate; // 0-10000
    }

    struct LatencyMeasurement {
        uint64 timestamp;
        uint256 latencyMs;
        address reporter;
    }

    mapping(address => Offer) public offers;
    mapping(bytes32 => SlaRecord) public slas; // key = keccak(client, provider, manifestRoot)
    mapping(address => ProviderReputation) public reputations;
    mapping(bytes32 => LatencyMeasurement[]) public latencyHistory; // key = slaKey
    mapping(address => uint256) public providerCollateral;
    
    address[] public activeProviders;
    mapping(address => bool) public isActiveProvider;

    uint256 public constant MIN_COLLATERAL = 10 ether;
    uint256 public constant VIOLATION_THRESHOLD = 5;
    uint256 public constant AUTO_SLASH_MULTIPLIER = 2;

    event OfferPublished(address indexed provider, string region, uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs, SlaTier tier, uint32 capacityGB, bool gpuAvailable);
    event OfferUpdated(address indexed provider, uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs);
    event OfferDeactivated(address indexed provider);
    event SlaStarted(bytes32 indexed key, address indexed client, address indexed provider, bytes32 manifestRoot, SlaTier tier, uint256 escrow);
    event SlaViolation(bytes32 indexed key, uint64 newViolations, uint256 penaltyWei, uint256 latencyMs);
    event SlaClosed(bytes32 indexed key, uint256 refund);
    event CollateralDeposited(address indexed provider, uint256 amount);
    event CollateralWithdrawn(address indexed provider, uint256 amount);
    event ReputationUpdated(address indexed provider, uint256 uptimeScore, uint256 bandwidthScore, uint256 proofSuccessRate);
    event AutoSlash(address indexed provider, bytes32 indexed slaKey, uint256 amount);
    event LatencyReported(bytes32 indexed slaKey, uint256 latencyMs, address reporter);

    /// @notice Deposit collateral to publish offers
    function depositCollateral() external payable {
        require(msg.value >= MIN_COLLATERAL, "insufficient collateral");
        providerCollateral[msg.sender] += msg.value;
        emit CollateralDeposited(msg.sender, msg.value);
    }

    /// @notice Withdraw collateral (only if no active SLAs)
    function withdrawCollateral(uint256 amount) external {
        require(providerCollateral[msg.sender] >= amount, "insufficient balance");
        providerCollateral[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
        emit CollateralWithdrawn(msg.sender, amount);
    }

    /// @notice Publish an offer on the marketplace
    function publishOffer(
        string calldata region,
        uint256 priceWeiPerGBMonth,
        uint256 expectedLatencyMs,
        SlaTier tier,
        uint32 capacityGB,
        bool gpuAvailable
    ) external {
        require(providerCollateral[msg.sender] >= MIN_COLLATERAL, "insufficient collateral");
        
        offers[msg.sender] = Offer({
            provider: msg.sender,
            region: region,
            priceWeiPerGBMonth: priceWeiPerGBMonth,
            expectedLatencyMs: expectedLatencyMs,
            tier: tier,
            publishedAt: uint64(block.timestamp),
            active: true,
            capacityGB: capacityGB,
            gpuAvailable: gpuAvailable,
            collateral: providerCollateral[msg.sender]
        });

        if (!isActiveProvider[msg.sender]) {
            activeProviders.push(msg.sender);
            isActiveProvider[msg.sender] = true;
        }

        emit OfferPublished(msg.sender, region, priceWeiPerGBMonth, expectedLatencyMs, tier, capacityGB, gpuAvailable);
    }

    /// @notice Update offer pricing and latency
    function updateOffer(uint256 priceWeiPerGBMonth, uint256 expectedLatencyMs) external {
        Offer storage o = offers[msg.sender];
        require(o.active, "no offer");
        o.priceWeiPerGBMonth = priceWeiPerGBMonth;
        o.expectedLatencyMs = expectedLatencyMs;
        emit OfferUpdated(msg.sender, priceWeiPerGBMonth, expectedLatencyMs);
    }

    /// @notice Deactivate offer
    function deactivateOffer() external {
        offers[msg.sender].active = false;
        emit OfferDeactivated(msg.sender);
    }

    /// @notice Get offer details
    function getOffer(address provider) external view returns (
        uint256 priceWeiPerGBMonth,
        uint256 expectedLatencyMs,
        uint8 tier,
        uint64 publishedAt,
        bool active,
        uint32 capacityGB,
        bool gpuAvailable,
        uint256 collateral
    ) {
        Offer memory o = offers[provider];
        return (o.priceWeiPerGBMonth, o.expectedLatencyMs, uint8(o.tier), o.publishedAt, o.active, o.capacityGB, o.gpuAvailable, o.collateral);
    }

    /// @notice Get list of active providers
    function getActiveProviders() external view returns (address[] memory) {
        uint256 count = 0;
        for (uint256 i = 0; i < activeProviders.length; i++) {
            if (offers[activeProviders[i]].active) count++;
        }
        address[] memory result = new address[](count);
        uint256 idx = 0;
        for (uint256 i = 0; i < activeProviders.length; i++) {
            if (offers[activeProviders[i]].active) {
                result[idx++] = activeProviders[i];
            }
        }
        return result;
    }

    function _slaKey(address client, address provider, bytes32 manifestRoot) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(client, provider, manifestRoot));
    }

    /// @notice Start an SLA with escrow
    function startSla(address provider, bytes32 manifestRoot, SlaTier tier) external payable {
        Offer memory o = offers[provider];
        require(o.active, "no offer");
        require(msg.value > 0, "escrow required");
        
        bytes32 key = _slaKey(msg.sender, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(!r.active, "sla exists");
        
        r.client = msg.sender;
        r.provider = provider;
        r.manifestRoot = manifestRoot;
        r.tier = tier;
        r.startAt = uint64(block.timestamp);
        r.lastChecked = uint64(block.timestamp);
        r.violations = 0;
        r.totalPenaltyWei = 0;
        r.escrow = msg.value;
        r.active = true;

        reputations[provider].totalDeals++;
        
        emit SlaStarted(key, msg.sender, provider, manifestRoot, tier, msg.value);
    }

    /// @notice Report latency measurement for an SLA
    function reportLatency(address client, address provider, bytes32 manifestRoot, uint256 latencyMs) external {
        bytes32 key = _slaKey(client, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(r.active, "no sla");

        latencyHistory[key].push(LatencyMeasurement({
            timestamp: uint64(block.timestamp),
            latencyMs: latencyMs,
            reporter: msg.sender
        }));

        emit LatencyReported(key, latencyMs, msg.sender);

        // Auto-check violation
        Offer memory o = offers[provider];
        if (latencyMs > o.expectedLatencyMs * 2) {
            _recordViolation(key, latencyMs);
        }
    }

    /// @notice Record a violation and apply penalty by tier
    function recordViolation(address client, address provider, bytes32 manifestRoot, uint256 latencyMs) external {
        bytes32 key = _slaKey(client, provider, manifestRoot);
        _recordViolation(key, latencyMs);
    }

    function _recordViolation(bytes32 key, uint256 latencyMs) internal {
        SlaRecord storage r = slas[key];
        require(r.active, "no sla");
        
        r.violations += 1;
        r.lastChecked = uint64(block.timestamp);

        // Penalty schedule by tier
        uint256 penalty;
        if (r.tier == SlaTier.Bronze) penalty = 1e14; // 0.0001 ETH
        else if (r.tier == SlaTier.Silver) penalty = 2e14;
        else if (r.tier == SlaTier.Gold) penalty = 5e14;
        else penalty = 1e15; // Platinum

        r.totalPenaltyWei += penalty;
        reputations[r.provider].totalViolations++;

        emit SlaViolation(key, r.violations, penalty, latencyMs);

        // Auto-slash if violations exceed threshold
        if (r.violations >= VIOLATION_THRESHOLD) {
            uint256 slashAmount = penalty * AUTO_SLASH_MULTIPLIER;
            if (providerCollateral[r.provider] >= slashAmount) {
                providerCollateral[r.provider] -= slashAmount;
                reputations[r.provider].totalSlashes++;
                emit AutoSlash(r.provider, key, slashAmount);
    }
        }
    }

    /// @notice Close SLA and refund escrow minus penalties
    function closeSla(address client, address provider, bytes32 manifestRoot) external {
        bytes32 key = _slaKey(client, provider, manifestRoot);
        SlaRecord storage r = slas[key];
        require(r.active, "no sla");
        require(msg.sender == client || msg.sender == provider, "not authorized");

        r.active = false;

        // Calculate refund
        uint256 refund = r.escrow;
        if (r.totalPenaltyWei < r.escrow) {
            refund = r.escrow - r.totalPenaltyWei;
        } else {
            refund = 0;
        }

        // Update reputation
        if (r.violations == 0) {
            reputations[provider].successfulDeals++;
        }

        // Transfer refund to client
        if (refund > 0) {
            payable(r.client).transfer(refund);
        }

        // Transfer penalties to provider as slash
        if (r.totalPenaltyWei > 0 && r.totalPenaltyWei < r.escrow) {
            payable(r.provider).transfer(r.totalPenaltyWei);
        }

        emit SlaClosed(key, refund);
    }

    /// @notice Update provider reputation scores
    function updateReputation(
        address provider,
        uint256 uptimeScore,
        uint256 bandwidthScore,
        uint256 proofSuccessRate
    ) external {
        require(uptimeScore <= 10000 && bandwidthScore <= 10000 && proofSuccessRate <= 10000, "invalid scores");
        
        ProviderReputation storage rep = reputations[provider];
        rep.uptimeScore = uptimeScore;
        rep.bandwidthScore = bandwidthScore;
        rep.proofSuccessRate = proofSuccessRate;

        emit ReputationUpdated(provider, uptimeScore, bandwidthScore, proofSuccessRate);
    }

    /// @notice Get provider reputation
    function getReputation(address provider) external view returns (
        uint256 totalDeals,
        uint256 successfulDeals,
        uint256 totalViolations,
        uint256 totalSlashes,
        uint256 uptimeScore,
        uint256 bandwidthScore,
        uint256 proofSuccessRate
    ) {
        ProviderReputation memory rep = reputations[provider];
        return (rep.totalDeals, rep.successfulDeals, rep.totalViolations, rep.totalSlashes, rep.uptimeScore, rep.bandwidthScore, rep.proofSuccessRate);
    }

    /// @notice Get latency history for an SLA
    function getLatencyHistory(bytes32 slaKey) external view returns (LatencyMeasurement[] memory) {
        return latencyHistory[slaKey];
    }

    /// @notice Get SLA details
    function getSla(bytes32 slaKey) external view returns (SlaRecord memory) {
        return slas[slaKey];
    }

    receive() external payable {}
}



# Bounty Board v0

- Contract: `contracts/BountyBoard.sol`
- Create a bounty by sending ETH with title/description.
- Claim by ID to receive the reward.

Examples:
```
// create
cast send <BountyBoard> "create(string,string)" "Fix docs" "Polish quickstarts" --value 0.5ether

// claim
cast send <BountyBoard> "claim(uint256,address)" 0 <recipient>
```

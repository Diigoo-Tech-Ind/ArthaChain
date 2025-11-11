# Permissioning & Exemptions

Contracts/addresses exempt from burns/anti-whale and rationale:

- Validators Pool: reward distribution pool; operational address.
- Rewards Pool: network incentives payouts; operational address.
- Ecosystem Grants Pool: grants; avoid circular burns.
- Marketing Wallet: operational budget.
- Developers Pool: protocol dev rewards.
- DAO Governance Pool: governance funding.
- Treasury Reserve: safety buffer and strategic reserves.

Unit Tests:
- `contracts/test/ArthaCoinExemptions.t.sol` verifies these are marked burnExempt on init.

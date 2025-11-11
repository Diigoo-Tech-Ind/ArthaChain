SHELL := /bin/bash

.PHONY: rust-tests foundry-tests echidna-tests sims all-tests docker-all ci-help

rust-tests:
	cargo test --manifest-path blockchain_node/Cargo.toml

foundry-tests:
	cd contracts && forge test -vvv && forge test --ffi --runs 256 --mc ArthaCoinInvariants -vvv

echidna-tests:
	cd contracts && echidna-test contracts/test/ArthaCoinEchidna.sol --config contracts/echidna.yaml

sims:
	python3 scripts/econ/sim_emissions.py
	python3 scripts/econ/sim_burn_curve.py
	python3 scripts/econ/sim_storage_supply.py
	python3 scripts/econ/sim_deal_market.py

all-tests: rust-tests foundry-tests echidna-tests sims

docker-all:
	./scripts/ci/run_all_tests.sh

ci-help:
	@echo "Push or open a PR to trigger .github/workflows/ci.yml (Rust, Foundry, Echidna, sims)."



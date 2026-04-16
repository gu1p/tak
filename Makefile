.PHONY: check coverage test lint fmt-check docs-check line-limits-check src-test-separation-check workflow-contract-check generated-artifact-ignore-check

TAK_RUN = cargo run --locked -p tak -- run

check:
	$(TAK_RUN) //:check

coverage:
	$(TAK_RUN) //:coverage

fmt-check:
	$(TAK_RUN) //:fmt-check

lint:
	$(TAK_RUN) //:lint

test:
	$(TAK_RUN) //:test

docs-check:
	$(TAK_RUN) //:docs-check

line-limits-check:
	$(TAK_RUN) //:line-limits-check

src-test-separation-check:
	$(TAK_RUN) //:src-test-separation-check

workflow-contract-check:
	$(TAK_RUN) //:workflow-contract-check

generated-artifact-ignore-check:
	$(TAK_RUN) //:generated-artifact-ignore-check

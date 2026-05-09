PYTHON ?= python3.14
VENV   ?= .venv

.PHONY: venv develop install test lint fmt clean

venv:
	$(PYTHON) -m venv $(VENV)
	$(VENV)/bin/pip install --upgrade pip maturin hatchling

# Step 1: build the Rust extension into the venv.
# Step 2: install the Python package (powergrid_env) in editable mode.
develop: venv
	$(VENV)/bin/maturin develop --release -m ../crates/powergrid-py/Cargo.toml
	$(VENV)/bin/pip install -e ".[dev,train]"

install:
	maturin develop --release -m ../crates/powergrid-py/Cargo.toml
	pip install -e ".[dev,train]"

test:
	$(VENV)/bin/pytest tests/ -v

lint:
	$(VENV)/bin/ruff check src/ tests/ scripts/

fmt:
	$(VENV)/bin/ruff format src/ tests/ scripts/

clean:
	rm -rf $(VENV) __pycache__ src/powergrid_env/__pycache__

run *args:
    cargo run --bin better-fonts -- {{args}}

run-ai *args:
    AGENT=1 cargo run --bin better-fonts -- {{args}}

fmt:
    cargo fmt --package better-fonts-cli

lint:
    cargo fmt --package better-fonts-cli -- --check
    cargo clippy --all-targets -- -D warnings

test:
    cargo test --all-targets

coverage:
    mkdir -p .tmp
    cargo llvm-cov --all-targets --fail-under-lines 90 --json --output-path .tmp/coverage-final.json

check: lint test coverage

build:
    cargo build --release --bin better-fonts
    mkdir -p bin
    cp target/release/better-fonts bin/better-fonts

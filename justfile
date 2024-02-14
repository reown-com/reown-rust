binary-crate            := "."

export JUST_ROOT        := justfile_directory()

# Default to listing recipes
_default:
  @just --list --list-prefix '  > '

# Open project documentation in your local browser
open-docs: (_build-docs "open")
  @echo '==> Opening documentation in system browser'

# Fast check project for errors
check:
  @echo '==> Checking project for compile errors'
  cargo check --workspace

# Build service for development
build:
  @echo '==> Building project'
  cargo build

# Build project documentation
build-docs: (_build-docs "")

# Run project test suite, skipping storage tests
test:
  @echo '==> Testing project (default)'
  cargo test --workspace

# Run project test suite, testing all features
test-all:
  @echo '==> Testing project (all features)'
  cargo test --workspace --all-features

# Run test from project documentation
test-doc:
  @echo '==> Testing project docs'
  cargo test --workspace --doc

# Clean build artifacts
clean:
  @echo '==> Cleaning project target/*'
  cargo clean

# Lint the project for any quality issues
lint: check fmt clippy commit-check

# Run project linter
clippy:
  #!/bin/bash
  set -euo pipefail

  if command -v cargo-clippy >/dev/null; then
    echo '==> Running clippy'
    cargo clippy --workspace --all-features --all-targets -- -D clippy::all -W clippy::style
  else
    echo '==> clippy not found in PATH, skipping'
    echo '    ^^^^^^ To install `rustup component add clippy`, see https://github.com/rust-lang/rust-clippy for details'
  fi

# Run code formatting check
fmt:
  #!/bin/bash
  set -euo pipefail

  if command -v cargo-fmt >/dev/null; then
    echo '==> Running rustfmt'
    cargo +nightly fmt --all
  else
    echo '==> rustfmt not found in PATH, skipping'
    echo '    ^^^^^^ To install `rustup component add rustfmt`, see https://github.com/rust-lang/rustfmt for details'
  fi

fmt-imports:
  #!/bin/bash
  set -euo pipefail

  if command -v cargo-fmt >/dev/null; then
    echo '==> Running rustfmt'
    cargo +nightly fmt -- --config group_imports=StdExternalCrate,imports_granularity=One
  else
    echo '==> rustfmt not found in PATH, skipping'
  fi

unit: lint test test-all

devloop: unit fmt-imports

# Run commit checker
commit-check:
  #!/bin/bash
  set -euo pipefail

  if command -v cog >/dev/null; then
    echo '==> Running cog check'
    cog check --from-latest-tag
  else
    echo '==> cog not found in PATH, skipping'
    echo '    ^^^ To install `cargo install --locked cocogitto`, see https://github.com/cocogitto/cocogitto for details'
  fi

# Build project documentation
_build-docs $open="":
  @echo "==> Building project documentation @$JUST_ROOT/target/doc"
  @cargo doc --all-features --workspace --no-deps ${open:+--open}

docker_image := "reqlang:0.1.0"

default:
    @just --list

[unix]
[private]
move-bins:
    cp target/debug/reqlang ~/.cargo/bin/reqlang
    cp target/debug/reqlang-lsp ~/.cargo/bin/reqlang-lsp
    cp target/debug/reqlang-client ~/.cargo/bin/reqlang-client

[windows]
[private]
move-bins:
    cp target/debug/reqlang.exe ~/.cargo/bin/reqlang.exe
    cp target/debug/reqlang-lsp.exe ~/.cargo/bin/reqlang-lsp.exe
    cp target/debug/reqlang-client.exe ~/.cargo/bin/reqlang-client.exe

[unix]
[private]
move-bins-release:
    cp target/release/reqlang ~/.cargo/bin/reqlang
    cp target/release/reqlang-lsp ~/.cargo/bin/reqlang-lsp
    cp target/release/reqlang-client ~/.cargo/bin/reqlang-client

[windows]
[private]
move-bins-release:
    cp target/release/reqlang.exe ~/.cargo/bin/reqlang.exe
    cp target/release/reqlang-lsp.exe ~/.cargo/bin/reqlang-lsp.exe
    cp target/release/reqlang-client.exe ~/.cargo/bin/reqlang-client.exe

# Build typescript types from Rust types required for VS Code extension
build_types:
    cd types && just build

# Build the code
build: build_types
    cargo build
    cd vsc && just build

# Build the code for release
build_release: build_types
    cargo build --release
    cd vsc && just build

# Build the code, install binaries and VS Code extension
install: build && move-bins
    echo 'Installed Bins (Debug)'
    cd vsc && just uninstall && just install

# Build the code for release, install binaries and VS Code extension
install_release: build_release && move-bins-release
    echo 'Installed Bins (Release)'
    cd vsc && just uninstall && just install

# Check linters against code
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all

# Automatically fix linting errors in code
lint-fix:
    cargo clippy --workspace --all-targets --all-features --fix -- -D warnings -W clippy::all

# Format code
format:
    cargo fmt --all
    just lint-fix
    cd vsc && just format

# Check that the code is formatted correctly
format-check:
    cargo fmt --all -- --check

# Run all non-test checks against code
check:
    cargo check --workspace --all-targets
    just format-check
    just lint
    cd vsc && just check

# Run all checks, tests, and build the code
verify:
    just check
    just test
    just build

# Run all tests
test:
    cargo test --workspace --all-targets --all-features

# Remove local branches that have been merged upstream
clean-git-branches:
    git branch -d $(git branch --merged=main | grep -v main) && git fetch --prune

# Build docker image for reqlang cli
build-docker:
    docker build -t {{docker_image}} .

build-docker-no-cache:
    docker build --no-cache -t {{docker_image}} .

# Run docker image for reqlang cli
run-docker *cli_args:
    docker run --rm --read-only -v "/$PWD/examples":/usr/local/src/examples:ro {{docker_image}} {{cli_args}}

run-mock-oauth:
    docker run --rm -p 8080:8080 -h localhost ghcr.io/navikt/mock-oauth2-server:2.1.2

# Run the status_code request file
run-status-request status_code:
    ./examples/valid/status_code.reqlang -e default -f curl -P status_code={{status_code}} | bash

# Build docs for reqlang crate
build-docs:
    cargo doc --no-deps --workspace --exclude cli

# Build and open docs for reqlang crate
build-docs-open:
    cargo doc --no-deps --workspace --exclude cli --open

# Get an estimated lines of code in the project
lines-of-code:
    git ls-files | grep -v package-lock.json | grep -v Cargo.lock | xargs wc -l | grep total 

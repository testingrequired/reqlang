docker_image := "kyleect/reqlang:0.1.0"

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

build_types:
    cd types && just build

build: build_types
    cargo build
    just build-vsc

build_release: build_types
    cargo build --release
    just build-vsc

install: build && move-bins
    echo 'Installed Bins (Debug)'
    cd vsc && just uninstall
    cd vsc && just install

install_release: build_release && move-bins-release
    echo 'Installed Bins (Release)'
    cd vsc && just uninstall && just install

build-vsc:
    cd vsc && just build

install-vsc:
    cd vsc && just install

uninstall-vsc:
    cd vsc && just uninstall

lint *args:
    cargo clippy --workspace --all-targets --all-features -- {{args}}

lint-fix *args:
    cargo clippy --workspace --all-targets --all-features --fix -- {{args}}

format *args:
    cargo fmt --all -- {{args}}

verify:
    cargo check --workspace --all-targets
    just format --check
    just lint -D warnings -W clippy::all
    just test
    just build

test:
    cargo test --workspace --all-targets --all-features

clean-git-branches:
    git branch -d $(git branch --merged=main | grep -v main) && git fetch --prune

build-docker:
    docker build -t {{docker_image}} .

build-docker-no-cache:
    docker build --no-cache -t {{docker_image}} .

run-docker *args:
    docker run --rm --read-only -v "/$PWD/examples":/app/examples:ro {{docker_image}} {{args}}

run-mock-oauth:
    docker run --rm -p 8080:8080 -h localhost ghcr.io/navikt/mock-oauth2-server:2.1.2

run-status-request status_code:
    ./examples/valid/status_code.reqlang -e default -f curl -P status_code={{status_code}} | bash

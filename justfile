default:
    @just --list

[unix]
move-bins:
    cp target/debug/reqlang ~/.cargo/bin/reqlang
    cp target/debug/reqlang-export ~/.cargo/bin/reqlang-export
    cp target/debug/reqlang-lsp ~/.cargo/bin/reqlang-lsp

[windows]
move-bins:
    cp target/debug/reqlang.exe ~/.cargo/bin/reqlang.exe
    cp target/debug/reqlang-export.exe ~/.cargo/bin/reqlang-export.exe
    cp target/debug/reqlang-lsp.exe ~/.cargo/bin/reqlang-lsp.exe

build:
    cargo build && just build-wasm
    just build-vsc

install: build && move-bins
    echo 'Installed Bins (Debug)'
    cd vsc && just uninstall
    cd vsc && just install

build-vsc:
    cd vsc && just build

build-wasm:
    cd wasm && just build
    cd wasm-example && rm -rf node_modules && npm i
    cd wasm-example && npm run test

lint *args:
    cargo clippy --workspace --all-targets --all-features -- {{args}}

format *args:
    cargo fmt --all -- {{args}}

verify:
    cargo check --workspace --all-targets
    cargo check --workspace --all-features --lib --target wasm32-unknown-unknown
    just format --check
    just lint -D warnings -W clippy::all
    just test

test:
    cargo test --workspace --all-targets --all-features
    cd wasm-example && npm run test

clean-git-branches:
    git branch -d $(git branch --merged=main | grep -v main) && git fetch --prune

build-docker:
    docker build -t kyleect/reqlang:0.1.0 .

build-docker-no-cache:
    docker build --no-cache -t kyleect/reqlang:0.1.0 .

run-docker *args:
    docker run --rm --read-only -v "/$PWD/examples":/app/examples:ro kyleect/reqlang:0.1.0 {{args}}

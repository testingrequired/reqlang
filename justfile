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

test:
    cargo test
    cd wasm-example && npm run test

clean-git-branches:
    git branch -d $(git branch --merged=main | grep -v main) && git fetch --prune

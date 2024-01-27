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
    cargo build
    just build-vsc

install: build && move-bins
    echo 'Installed Bins (Debug)'

build-vsc:
    cd vsc && just build

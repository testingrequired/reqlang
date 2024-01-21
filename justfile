default:
    @just --list

[unix]
move-bins:
    cp target/debug/reqlang ~/.cargo/bin/reqlang
    cp target/debug/reqlang-export ~/.cargo/bin/reqlang-export

[windows]
move-bins:
    cp target/debug/reqlang.exe ~/.cargo/bin/reqlang.exe
    cp target/debug/reqlang-export.exe ~/.cargo/bin/reqlang-export.exe

build:
    cargo build

install: build && move-bins
    echo 'Installed Bins (Debug)'
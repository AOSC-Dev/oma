deb:
    cargo build --release
    mkdir -pv completions
    COMPLETE=bash ./target/release/oma > completions/oma.bash
    COMPLETE=fish ./target/release/oma > completions/oma.fish
    ./target/release/oma generate-manpages
    cargo deb -Z xz

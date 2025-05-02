deb:
    cargo build --release
    mkdir -pv completions
    COMPLETE=bash oma > completions/oma.bash
    COMPLETE=fish oma > completions/oma.fish
    ./target/release/oma generate-manpages
    cargo deb -Z xz

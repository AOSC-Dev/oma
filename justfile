deb:
    cargo build --release --no-default-features --features nice-setup
    mkdir -pv completions
    COMPLETE=bash ./target/release/oma > completions/oma.bash
    COMPLETE=zsh ./target/release/oma > completions/_oma
    COMPLETE=fish ./target/release/oma > completions/oma.fish
    LANG=C ./target/release/oma generate-manpages
    cargo deb -Z xz

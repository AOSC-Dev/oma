has_modern_lzma := `pkg-config --atleast-version=5.4 liblzma && echo "true" || echo "false"`
features := if has_modern_lzma == "true" {
    "nice-setup decompress-parallel"
} else {
    "nice-setup"
}

deb:
    cargo build --release --no-default-features --features "{{ features }}"
    mkdir -pv completions
    COMPLETE=bash ./target/release/oma > completions/oma.bash
    COMPLETE=zsh ./target/release/oma > completions/_oma
    COMPLETE=fish ./target/release/oma > completions/oma.fish
    cargo deb -Z xz --features "{{ features }}"

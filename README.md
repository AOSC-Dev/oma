# Oma

Oma (Omakase) ~~(Oh My Ailurus, 小熊猫包管理)~~ is a AOSC OS Package manager.

> Omakase お任せ (adj.): According to the chef's choice. — Marriam-Webster.

## Dependencies

- libapt-pkg 2.5.4
- Glibc
- Ripgrep binary (optional)
- C Compiler
- OpenSSL
- Rustc with cargo

## Build & install

```bash
cargo build --release
cp ./target/release/oma /usr/local/bin/oma
```

## Usage

```bash
saki@Magputer [ aoscpt@master ] $ oma
Usage: oma <COMMAND>

Commands:
  install     Install Package
  upgrade     Update Package
  download    Download Package
  remove      Delete Package
  refresh     Refresh Package database
  show        Show Package
  search      Search Package
  list-files  package list files
  provides    Search file from package
  fix-broken  Fix system dependencies broken status
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## TODO
- [ ] PolicyKit Support
- [ ] Flatpak Support
- [ ] Improve provides (needs p-vector-rs support, see https://github.com/AOSC-Dev/p-vector-rs/pull/2)
- [ ] CDROM Support for AOSC OS/Retro


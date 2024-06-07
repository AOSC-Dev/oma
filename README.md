# oma

oma（Oh My Ailurus, 小熊猫包管理）is a package manager frontend for `libapt-pkg`. oma is the default package manager interface for AOSC OS.

Although it is based on apt, we did quite a bit of extra work, the goal of this project is to make apt with better user interaction (especially for AOSC OS users), you can get a feel for the differences between oma and apt with the following examples:

### Pending Operations

![](screenshot/image.png)

### Multi-thread download
[multi-thread-download.webm](https://github.com/AOSC-Dev/oma/assets/19554922/e857a946-b6c5-4c22-8d56-398b2ce0a624)



### Smart Search
[oma-search.webm](https://github.com/AOSC-Dev/oma/assets/19554922/eed6d992-6464-48eb-8b4f-075ea378bd0c)


### Undo
[undo.webm](https://github.com/AOSC-Dev/oma/assets/19554922/f971313b-15bd-4a8e-9b33-aa5c4645e46b)


...and more.

## Dependencies

- libapt-pkg 2.5.4
- Glibc
- Ripgrep binary (optional, `--no-default-features --features contents-without-rg` to disable)
- C Compiler
- OpenSSL
- Rustc with Cargo
- nettle

## Build 


- build for Debian/Ubuntu

```bash
cargo build —release —no-default-features —features generic
sudo cp ./target/release/oma /usr/local/bin/oma
sudo cp data/config/oma.toml /etc/oma.toml
```
## Usage

```bash
saki@Magputer [ aoscpt@master ] $ oma
Omakase (oma) - Package management interface for AOSC OS

Usage: oma [OPTIONS] [COMMAND]

Commands:
  install     Install package(s) from the repository
  upgrade     Upgrade packages installed on the system
  download    Download package(s) from the repository
  remove      Remove the specified package(s)
  refresh     Refresh repository metadata/catalog
  show        Show information on the specified package(s)
  search      Search for package(s) available from the repository
  files  List files in the specified package
  provides    Search for package(s) that provide(s) certain patterns in a path
  fix-broken  Resolve broken system dependencies in the system
  pick        Install specific version of a package
  mark        Mark status for one or multiple package(s)
  list        List package(s) available from the repository
  depends     Lists dependencies of one or multiple packages
  rdepends    List reverse dependency(ies) for the specified package(s)
  clean       Clear downloaded package cache
  history     Show a history/log of package changes in the system
  help        Print this message or the help of the given subcommand(s)

Options:
      --debug    Run oma with debug mode
  -h, --help     Print help (see more with '--help')
  -V, --version  Print version

```


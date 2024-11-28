![oma banner: header](https://github.com/user-attachments/assets/f68316e1-d007-4586-a3e0-3d9a56c2dea4)

### oma - Oh My Ailurus / Package Manager for AOSC OS

[Features](#features) | [Install](#install) | [Contribute](#contributing)

oma is an attempt at reworking APT's interface, making it more user-friendly, more robust against common user errors, and more performant during package downloads. oma also integrates closely with AOSC OS's various system management functions, from mirror configuration, topic (testing) repository enrollment, to system feature protection.

For a more detailed overview on oma's features, see [features](#Features).

### oma is also available for other dpkg-based OS.

Please see [Install](#install).

## Features

- Pending Operations: Preview and manage upcoming changes with an interactive interface.
- Faster Downloads: Faster package downloads, powered by the performant [reqwest](https://crates.io/crates/reqwest) HTTP and multi-threaded downloads.
- Smart Search: Leveraging the [indicium](https://crates.io/crates/indicium) search engine for more relevant package search results.
- Poka-yoke (Error-Proofing) Mechanisms: Clear guidance and features like operation undo, we help prevent system failures caused by carelessness or errors.
  - Protect system packages: **(Only for AOSC OS)** Prevent users from uninstalling critical system components.
- Compatibility with dpkg-based OS: Supports major dpkg-based distributions like Debian, Ubuntu, and Deepin, making package management more user-friendly.
- Mirror management: Provide a mirror management tools, includeing speedtest and sort.

More Features and Preview, please see [LATEST RELEASE NOTE](https://hackmd.io/cccNjfuZQo24Ys8SW7yNUQ)

### Undo Changes

Roll back operations with a simple command.

[Undo Feature](https://github.com/AOSC-Dev/oma/assets/19554922/f971313b-15bd-4a8e-9b33-aa5c4645e46b)

## Dependencies

To build oma, ensure the following dependencies are installed:

- libapt-pkg (part of [APT](https://salsa.debian.org/apt-team/apt.git))
- [LLVM and Clang](https://llvm.org/)
- [Nettle](https://www.lysator.liu.se/~nisse/nettle/) (recommended) or [OpenSSL](https://openssl.org/)
- [Rustc](https://www.rust-lang.org/) and [Cargo](https://crates.io/)
- [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/) or [pkgconf](http://pkgconf.org/)

During runtime, oma requires or recommends the following:

- [ripgrep](https://github.com/BurntSushi/ripgrep) (optional, accelerates `oma provides`, `oma files`, and `oma command-not-found`)

## Installation

oma is pre-installed with AOSC OS. It is also available for Debian, Ubuntu, Deepin, openKylin, and more dpkg-based OS.

### Automatic Installation

```bash
curl -sSf https://repo.aosc.io/get-oma.sh | sudo sh
```

### Building from Source

1. Clone the repository:

   ```bash
   git clone https://github.com/AOSC-Dev/oma.git
   cd oma
   ```

2. Build the binary as an installable .deb package:

   ```bash
   cargo deb -Z xz
   ```

3. Install and profit!

## Usage

### Entering the interactive package management interface

```bash
oma # without arguments
```

### Example Commands

- Installing a package:

  ```bash
  oma install <package_name>
  ```

- Searching for a package:

  ```bash
  oma search <keyword>
  ```
- Removing a package:

  ```bash
  oma remove <package_name>
  ```

- Refreshing repository metadata (done automatically before `oma install` and `oma upgrade`):

  ```bash
  oma refresh
  ```

For a full list of available sub-commands and arguments, run:

```bash
oma help
```

## Command Reference

| Command      | Description                                 |
| ------------ | ------------------------------------------- |
| `install`    | Install package(s) from the repository      |
| `upgrade`    | Upgrade all installed packages              |
| `download`   | Download package(s) without installing      |
| `remove`     | Remove specified package(s)                 |
| `refresh`    | Refresh repository metadata                 |
| `search`     | Search for package(s) in the repository     |
| `show`       | Show detailed information for a package     |
| `files`      | List files in the specified package         |
| `provides`   | Find packages providing specific patterns   |
| `fix-broken` | Fix broken dependencies                     |
| `pick`       | Install a specific version of package(s)    |
| `mark`       | Mark package(s) with a specific status      |
| `list`       | List all available packages                 |
| `depends`    | Show dependencies for package(s)            |
| `rdepends`   | Show reverse dependencies for package(s)    |
| `clean`      | Clear downloaded package cache              |
| `history`    | Show package history or change logs         |
| `help`       | Show help of oma or the given subcommand(s)

## Contributing

**Contributions are welcome!**

Please feel free to file issues or pull requests to help improve oma.

**Please see [CONTRIBUTING](./CONTRIBUTING.md) for detailed instructions.**

## License

oma is licensed under the GNU General Public License v3.0. See the [COPYING](./COPYING) file for details.

![oma banner: footer](https://github.com/user-attachments/assets/9bf0b9ed-6969-4bf8-b67e-3835925ce8c0)

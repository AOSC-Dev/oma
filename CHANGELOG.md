# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### 🚀 Features

- *(oma-topics)* Add atm state file deserialize failed debug info
- *(oma-refresh)* Support `Acquire-By-Hash`
- *(oma-fetch)* [**breaking**] Allow user set file type, not auto check

### 🐛 Bug Fixes

- Fix atm.list file comment no new line in 1st line
- *(oma-refresh)* Fix some mirror only have Release file ...
- *(oma-refresh)* Fix fetch some mirror has no `InRelease` file
- *(oma-refresh)* Do not use `inrelease_path` value
- Fix build without `aosc` feature
- *(oma-fetch)* Avoid unsupport file type

### 🚜 Refactor

- *(oma-refresh)* Split `update_db` fn
- *(oma-refresh)* Use `and_then` to replaced `match`
- *(oma-refresh)* No need to use `OnceCell`
- Use const var to save AOSC_MIRROR_FILE path
- *(oma-refresh)* Use type builder for `OmaRefresh`

### ⚡ Performance

- *(oma-refresh)* Reuse `reqwest::Clinet`
- Always reuse `reqwest::Client`

### 🎨 Styling

- Apply `cargo clippy`
- Apply `cargo fmt`
- Apply `cargo clippy`
- Apply `cargo clippy`
- Apply `cargo clippy`
- Apply `cargo fmt` and `cargo clippy`
- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Add create pr label
- Add `generic` feature ...

### I18n

- *(zh-TW)* Initialize translation

## [oma-pm-v0.18.12] - 2024-05-18

### 🚀 Features

- *(oma-pm)* Rename `PkgInfo` to `UnsafePkgInfo` ...

### 🐛 Bug Fixes

- Fix `tui.rs` build

### 📚 Documentation

- Update changelog use git-cliff

### 🎨 Styling

- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Add cliff config
- Add auto release body
- *(oma-pm)* Release oma-pm-v0.18.12

## [1.3.11] - 2024-05-17

### 🐛 Bug Fixes

- Fix oma upgrade will segfault

### ⚙️ Miscellaneous Tasks

- Release v1.3.11

## [release-test] - 2024-05-16

### ⚙️ Miscellaneous Tasks

- Add auto release workflow

## [1.3.10] - 2024-05-14

### 🚀 Features

- *(oma-refresh)* Only debug mode display unknown file type warn

### ⚙️ Miscellaneous Tasks

- Release v1.3.10

### I18n

- Display oma lock file path in `failed-to-unlock-oma`

## [oma-refresh-v0.13.2] - 2024-05-14

### 🚀 Features

- *(oma-refresh)* Do not download udeb contents

### 🐛 Bug Fixes

- *(oma-refresh)* Fix compatibility on debian

### ⚙️ Miscellaneous Tasks

- Update all deps and `cargo fmt`
- *(oma-refresh)* Release oma-refresh-v0.13.2

## [oma-refresh-v0.13.1] - 2024-05-13

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.13.1

## [1.3.9] - 2024-05-13

### 🐛 Bug Fixes

- *(oma-refresh)* Handle oma refresh download mirror file contains '+' ...

### ⚙️ Miscellaneous Tasks

- Fix pull request format
- Release v1.3.9

## [1.3.8] - 2024-05-13

### 🐛 Bug Fixes

- Implement missing error type conversion

### ⚙️ Miscellaneous Tasks

- Add tag oma auto create pull request workflow
- Release v1.3.8
- Strip version prefix
- Fix wrong syntax

## [1.3.7] - 2024-05-13

### 🐛 Bug Fixes

- Do not remove `.bin` file in `oma clean`
- Fix `oma upgrade` maybe not retry 3 times
- Revert ctrlc restore terminal feature ...

### ⚙️ Miscellaneous Tasks

- (fix) use reset_shell_mode to reset the terminal
- Drop unused line
- Remove unused dependencies
- Release v1.3.7

## [1.3.6] - 2024-05-07

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.3.6

## [oma-refresh-v0.13.0] - 2024-05-07

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.13.0

## [oma-pm-v0.18.11] - 2024-05-07

### 🚀 Features

- Improve `OutputError` display
- *(oma-refresh)* Add Deb822 sources.list support
- Support glob in `oma show`

### 🐛 Bug Fixes

- *(oma-refresh)* Handle uppcase `signed-by`
- Try to fix terminal nothing after `oma history` exit

### 🚜 Refactor

- *(oma-refresh)* Use `TryFrom` trait for `SourceEntry` convert to `OmaSourceEntry`
- Use `stdout().execute`

### 🎨 Styling

- Apply `cargo fmt`
- Apply `cargo clippy`
- *(oma-refresh)* Improve
- Apply `cargo clippy`

### ⚙️ Miscellaneous Tasks

- Adapt `oma-apt` v0.5 change
- Update all deps
- *(oma-pm)* Release v0.18.11

## [oma-refresh-v0.12.13] - 2024-05-05

### 🚜 Refactor

- *(oma-fetch)* Refactor some step to `file_reader`
- *(oma-fetch)* Refactor some step to `file_reader` (2)

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.3.5] - 2024-05-04

### ⚙️ Miscellaneous Tasks

- Release v1.3.5

## [oma-refresh-v0.12.12+sequoua-header-fi] - 2024-05-04

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.12.12+sequoua-header-fix

## [oma-fetch-v0.9.1] - 2024-05-04

### 🚀 Features

- *(oma-fetch)* Limit thread to 1 if download mirror most of `file:`
- Disable `hyper` and `rustls` debug output with `--debug` arg
- *(oma-fetch)* Remove useless debug output
- *(oma-refresh)* Improve debug struct output format
- Display filename and line number debug message with `--debug` arg
- *(oma-fetch)* Download local file will display download progress
- *(oma-refresh)* Improve `parse_date` debug message

### 🐛 Bug Fixes

- *(oma-fetch)* Fix .bz2 file uncompress in `download_local` function
- *(oma-fetch)* Fix `oma refresh` will segfault
- *(oma-fetch)* Fix download source sort

### 🚜 Refactor

- *(oma-refresh)* No need clone `date` val
- *(oma-refresh)* Avoid redundant string copy in `InReleaseParser`
- *(oma-fetch)* Handle sources length is 0 in `SingleDownloader::try_download`

### 📚 Documentation

- Improve `date_hack` comment
- *(oma-refresh)* Fix a comment typo
- *(oma-refresh)* Add comment

### 🎨 Styling

- Apply `cargo clippy`
- Apply `cargo fmt`

### 🧪 Testing

- *(oma-refresh)* Improve `test_date_hack`

### ⚙️ Miscellaneous Tasks

- *(oma-fetch)* Release oma-fetch-v0.9.1

## [1.3.4] - 2024-05-03

### ⚙️ Miscellaneous Tasks

- Release v1.3.4

## [oma-refresh-v0.12.11+sequoia-header-fix] - 2024-05-03

### 🚀 Features

- *(oma-fetch)* Add bz2 compress file support

### ⚙️ Miscellaneous Tasks

- Bump multi crates

## [1.3.3] - 2024-05-03

### ⚙️ Miscellaneous Tasks

- Release v1.3.3

## [oma-refresh-v0.12.10+sequoia-header-fix] - 2024-05-03

### 🐛 Bug Fixes

- *(oma-refresh)* Do not download `debian-installer` component type
- *(oma-refresh)* Fix logic mistake
- *(oma-refresh)* Fix download compress package list will download uncompress package list
- *(oma-fetch)* Use `BufReader` to fix `Write after end of stream`
- *(oma-refresh)* Handle InRelease is `Thu, 02 May 2024  9:58:03 UTC`
- *(oma-refresh)* Handle `0:58:03`
- *(oma-refresh)* Compatibe some Ubuntu source
- *(oma-fetch)* Only `can_resome` and `allow_resume` will seek to end
- *(oma-fetch)* Fix `download_local` download compress file

### 🚜 Refactor

- *(oma-refresh)* Refactor `InReleaseParser` args

### 🎨 Styling

- *(oma-refresh)* Fix a function name typo
- Apply `cargo clippy`
- Apply `cargo clippy` again

### ⚙️ Miscellaneous Tasks

- *(oma-fetch)* Adjust dependencies
- Bump multi crates

## [1.3.2] - 2024-05-02

### ⚙️ Miscellaneous Tasks

- Release v1.3.2

## [oma-history-v0.4.2] - 2024-05-02

### ⚙️ Miscellaneous Tasks

- *(oma-history)* Release oma-history-v0.4.2

## [oma-refresh-v0.12.9+sequoia-header-fix] - 2024-05-02

### 🚀 Features

- Add more debug info

### 🐛 Bug Fixes

- *(oma-refresh)* Do not raise Error if InRelease has unsupported type
- *(oma-refresh)* Fix `valid_until_data` raise error type
- *(oma-refresh)* Fix InRelease entry on Ubuntu
- *(oma-history)* Fix overflow

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.12.9+sequoia-header-fix

## [1.3.1] - 2024-04-29

### ⚙️ Miscellaneous Tasks

- Release v1.3.1

## [oma-pm-v0.18.9] - 2024-04-29

### 🐛 Bug Fixes

- *(oma-pm)* Fix oma `--yes` execute dpkg

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.18.9

## [1.3.0-beta.5] - 2024-04-24

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.3.0-beta.5

## [0.12.8+sequoia-header-fix] - 2024-04-24

### 🐛 Bug Fixes

- *(oma-console)* Improve oma style progress bar align
- *(oma-console)* More space for display download bytes

### 🚜 Refactor

- *(oma-fetch)* Refactor `http_download` logic

### 🎨 Styling

- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.12.8-with-sequoia-header-fix

## [1.3.0-beta.4] - 2024-04-03

### 🐛 Bug Fixes

- *(oma-refresh)* Fix if InRelease entry file name contains twice `.`
- *(tui)* Fix remove item from packages panel after remove item from pending panel will panic

### 🚜 Refactor

- *(oma-refresh)* No need to clone string at `utc_tzname_quirk` function

### 🎨 Styling

- *(oma-refresh)* Fix a var name typo

### ⚙️ Miscellaneous Tasks

- Fix build without `aosc` feature
- Release v1.3.0-beta.4

## [1.3.0-beta.3] - 2024-04-03

### 🚀 Features

- *(tui)* Input space twice remove item from pending list

### 🐛 Bug Fixes

- *(tui)* Switch panel will selected index 0
- Dependency issue interface do not 80 new line

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.3.0-beta.3

## [oma-pm-v0.18.8] - 2024-04-02

### 🚀 Features

- *(oma-topics)* Filter newest topic to list from multi mirrors

### 🐛 Bug Fixes

- *(oma-pm)* Oma with `--yes` argument will set `DEBIAN_FRONTEND` as `noninteractive`
- *(oma-pm)* Use `dpkg --force-confold --force-confdef` option with `yes` argument

### 🎨 Styling

- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.18.8

## [1.3.0-beta.2] - 2024-03-29

### 🐛 Bug Fixes

- *(tui)* Lock oma before committing instead of immediately after opening tui

### 🎨 Styling

- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Release v1.3.0-beta.2

## [oma-pm-v0.18.7] - 2024-03-29

### 🐛 Bug Fixes

- *(oma-pm)* Improve `mark_install` mark `auto_inst` logic

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.18.7

## [oma-pm-v0.18.6] - 2024-03-28

### 🐛 Bug Fixes

- *(oma-pm)* Workaround check dependency issue and set `auto_inst` flag as true ...

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.18.6

## [oma-utils-v0.7.3] - 2024-03-28

### ⚙️ Miscellaneous Tasks

- Bump multi crates

## [oma-console-v0.11.0] - 2024-03-28

### ⚙️ Miscellaneous Tasks

- *(oma-console)* Release v0.11.0

## [oma-console-v0.10.2] - 2024-03-28

### 🐛 Bug Fixes

- *(oma-console)* Fix Plain text should output to stdout. not stderr

### ⚙️ Miscellaneous Tasks

- Update all deps
- *(oma-console)* Release v0.10.2

## [oma-fetch-v0.8.5] - 2024-03-28

### 🚀 Features

- *(oma-fetch)* Only retry times > 1 will display retry message

### 🎨 Styling

- Apply `cargo fmt`

### ⚙️ Miscellaneous Tasks

- *(oma-fetch)* Release oma-fetch-v0.8.5

## [oma-pm-v0.18.4] - 2024-03-27

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release v0.18.4

## [1.3.0-beta.1] - 2024-03-27

### 🐛 Bug Fixes

- Workaround `mark_install` method auto_instl flag

### ⚙️ Miscellaneous Tasks

- Release v1.3.0-beta.1

## [oma-refresh-0.12.7+sequoia-header-fix] - 2024-03-27

### 🐛 Bug Fixes

- *(oma-refresh)* Fix cleartext-signed repositories
- *(oma-refresh)* Add default_features = false for sequoia-openpgp

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release v0.12.7+sequoia-header-fix

## [oma-topics-v0.8.1] - 2024-03-26

### 🚜 Refactor

- *(oma-topics)* No need check arch in `add` method

### ⚙️ Miscellaneous Tasks

- *(oma-topics)* Release v0.8.1

## [oma-topics-v0.8.0] - 2024-03-26

### 🚀 Features

- *(tui)* Do not bold tips
- Add check terminal size display tips feature
- *(tui)* Improve ui style
- *(tui)* Improve highlight item style
- *(oma-topics)* Filter topics list by arch

### 🐛 Bug Fixes

- *(tui)* Fix remove empty vector entry
- Fix pending remove item will loss cursor
- Run oma tui will lock oma
- *(i18n)* Lint UI strings

### 🎨 Styling

- *(oma-fetch)* Apply `cargo clippy` suggest

### ⚙️ Miscellaneous Tasks

- Remove useless line
- Update all deps
- Bump multi crates

### I18n

- Add another-oma-is-running
- Remove useless translate
- Fix some string

## [oma-pm-v0.18.2] - 2024-03-17

### 🚀 Features

- Add egg

### 🚜 Refactor

- Use move `show_broken_pkg` from `oma-apt` to oma logic ...

### 🎨 Styling

- Fix with `cargo fmt`

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.18.2

## [oma-pm-v0.18.1] - 2024-03-15

### 🚀 Features

- Use `resolvo-deb` to print dependency issue
- Add apt `show_broken` output
- Imporve unmet dependency output
- Improve unmet dependency message output

### 🐛 Bug Fixes

- *(oma-utils)* Adapt new zbus
- Fix install local .deb file show broken dependency
- Fix `show_broken` has wrong output
- Fix `install-recommend` default should is true
- Fix `auto_inst` var logic ...

### 🎨 Styling

- *(oma-pm)* Remove discard zbus api

### ⚙️ Miscellaneous Tasks

- Update all deps
- Remove useless dep and line
- *(oma-pm)* Release oma-pm v0.18.1

### I18n

- Add some strings

## [oma-pm-v0.17.0] - 2024-01-26

### 🚀 Features

- `oma` with subcommand will go to tui interface
- *(tui)* Add `available/removable/installed`
- *(tui)* Some changes
- *(tui)* Start interface add packages available info

### 🚜 Refactor

- *(oma-pm)* Use `OmaSearch` struct to save search index

### 🎨 Styling

- Apply `cargo clippy` suggest

### ⚙️ Miscellaneous Tasks

- Bump multi crates

## [oma-refresh-v0.12.3] - 2024-01-24

### 🚀 Features

- Add `oma topics` alias subcommand `oma topic`

### ⚙️ Miscellaneous Tasks

- Bump multi crates for update deps

## [0.16.2] - 2024-01-18

### ⚙️ Miscellaneous Tasks

- Add oma-dbus.xml

## [oma-utils-v0.7.0] - 2024-01-16

### 🚀 Features

- *(oma-pm)* Add dbus broadcast oma running status message

### ⚙️ Miscellaneous Tasks

- Re set version to 1.3.0-alpha.0
- *(oma-pm)* Release oma-pm v0.16.0
- Bump multi crates

## [1.2.24] - 2024-03-29

### ⚙️ Miscellaneous Tasks

- Release v1.2.24

## [oma-console-v0.10.1] - 2024-03-28

### 🐛 Bug Fixes

- *(oma-console)* Handle if terminal width too small string can't find breakpoint ...

### ⚙️ Miscellaneous Tasks

- *(oma-console)* Release v0.10.1

## [1.2.23] - 2024-03-27

### 🎨 Styling

- *(oma-fetch)* Apply `cargo clippy` suggest

### ⚙️ Miscellaneous Tasks

- Release v1.2.23

## [oma-refresh-v0.12.6+sequoia-header-fix] - 2024-03-27

### 🐛 Bug Fixes

- *(oma-refresh)* Fix cleartext-signed repositories
- *(oma-refresh)* Add default_features = false for sequoia-openpgp

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh v0.12.6+sequoia-header-fix

## [1.2.22] - 2024-03-18

### 🐛 Bug Fixes

- Update `oma-apt` to v0.4.1 to fix description is empty will segfault

### ⚙️ Miscellaneous Tasks

- Release v1.2.22

## [1.2.21] - 2024-03-16

### ⚙️ Miscellaneous Tasks

- Release v1.2.21

## [oma-utils-v0.6.4] - 2024-03-16

### ⚙️ Miscellaneous Tasks

- Try to pin `zvariant_utils` to v1.0.1 to fix rustc 1.74.0 build
- *(oma-utils)* Release v0.6.4

## [1.2.20] - 2024-03-15

### ⚙️ Miscellaneous Tasks

- Release v1.2.20

## [oma-pm-v0.15.13] - 2024-03-15

### 🐛 Bug Fixes

- Fix `install-recommend` default should is true

### 📚 Documentation

- Use gif to preview oma animate
- Fix demo display

### ⚙️ Miscellaneous Tasks

- Remove useless dep
- *(oma-pm)* Release oma-pm v0.15.13

## [1.2.19] - 2024-03-06

### ⚙️ Miscellaneous Tasks

- Release v1.2.19

## [oma-utils-v0.6.3] - 2024-03-06

### 🐛 Bug Fixes

- *(oma-console)* Use `WRITE.prefix_len` to calc prefix length
- Fix `cause_writer` wrong prefix len

### 🚜 Refactor

- *(oma-console)* Improve oma style message length calc method

### 🎨 Styling

- Fix with `cargo fmt`
- Run `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Bump multi crates

## [1.2.18] - 2024-03-06

### ⚙️ Miscellaneous Tasks

- Release v1.2.18

## [oma-pm-v0.15.11] - 2024-03-06

### 🐛 Bug Fixes

- *(oma-pm)* Fix always display long description

### 🚜 Refactor

- *(oma-pm)* `format_description` method no need to return String

## [1.2.17] - 2024-03-06

### ⚙️ Miscellaneous Tasks

- Release v1.2.17

## [oma-console-v0.9.2] - 2024-03-06

### 🐛 Bug Fixes

- *(i18n/zh-CN)* Fix retry count message
- *(oma-console)* Fix oma style message output newline with prefix

### ⚙️ Miscellaneous Tasks

- Release oma-console-v0.9.2

## [1.2.16] - 2024-03-04

### ⚙️ Miscellaneous Tasks

- *(oma-utils)* Downgrade `zbus` to 3.15 to fix rustc 1.74 compile
- Release v1.2.16

## [1.2.15] - 2024-03-03

### 🚀 Features

- *(oma-pm)* Add more debug info
- *(oma-pm)* Add more debug info
- *(oma-pm)* Add more debug info

### 🐛 Bug Fixes

- *(oma-pm)* Fix `apt_style_filename` handle not standard filename

### ⚙️ Miscellaneous Tasks

- Update all deps
- Update all deps
- *(oma-pm)* Bump `oma-pm-operation-type` version
- Release v0.12.15
- Apply clippy and fmt suggest
- Apply cargo fmt

## [1.2.14] - 2024-02-25

### ⚙️ Miscellaneous Tasks

- Release v1.2.14

## [oma-refresh-v0.12.5] - 2024-02-25

### 🚀 Features

- *(oma-pm)* Add more debug info
- *(oma-refresh)* Refactor UTC marker hack as utc_tzname_quirk()

### 🐛 Bug Fixes

- *(i18n/zh-CN)* Fix {$path} template
- *(oma-pm)* Check virtual package in `find_unmet_deps_with_markinstall`
- *(oma-pm)* Fix unmet version check
- *(oma-pm)* Fix unmet version check (2)
- *(oma-pm)* Fix unmet version check (3)
- *(oma-pm)* Fix unmet version check (or issue)
- *(oma-pm)* Fix unmet version check (or issue) (2)
- *(oma-pm)* Try to fix pre-dep unmet dep ui
- *(oma-pm)* Try to fix pre-dep unmet dep ui (2)
- *(oma-pm)* Try to fix pre-dep unmet dep ui (3)
- *(oma-refresh)* Make `Valid-Until' field optional
- *(oma-refresh)* Allow InRelease files signed with SHA-1
- *(oma-contents)* Use io more precisely
- *(oma-refresh)* Remove lifetime annotation for StandardPolicy
- *(oma-contents)* Simplify io usage
- *(oma-refresh)* Also apply utc_tzname_quirk() to Valid-Until
- *(oma-refresh)* Drop unneeded type definition for v = VerifierBuilder

### 🎨 Styling

- Fix with `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Run cargo fmt
- Run cargo clippy (warnings as error)
- Apply cargo fmt
- Rename Omakase as oma
- Update screenshot
- *(oma-refresh)* Release oma-refresh-v0.12.5

### Hack

- *(oma-refresh)* Support UTC notation "UTC"

## [1.2.13] - 2024-02-16

### ⚙️ Miscellaneous Tasks

- Release v1.2.13

## [oma-topics-v0.7.2] - 2024-01-24

### ⚙️ Miscellaneous Tasks

- Bump multi crates for update deps
- Default use `Rustls`
- Bump multi crates

## [1.2.12] - 2024-01-23

### 🐛 Bug Fixes

- *(oma-topics)* Refresh topics will disable not exist topics

### ⚙️ Miscellaneous Tasks

- Release v1.2.12

## [1.2.11] - 2024-01-23

### 🚀 Features

- Add `oma topics` alias subcommand `oma topic`

### ⚙️ Miscellaneous Tasks

- Downgrade `rustix` to 0.38.28 to fix loongarch64 build
- Release v1.2.11

## [1.2.10] - 2024-01-20

### 🐛 Bug Fixes

- *(lang)* Disable bidirectional isolation in Fluent

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.2.10

## [1.2.9] - 2024-01-18

### ⚙️ Miscellaneous Tasks

- Release v1.2.9

## [oma-topics-v0.7.1] - 2024-01-18

### ⚙️ Miscellaneous Tasks

- *(oma-topics)* Release oma-pm v0.7.1

## [oma-pm-v0.15.8] - 2024-01-18

### 🐛 Bug Fixes

- *(oma-pm)* Handle search result not only one provide (2)
- *(oma-topics)* `/var/lib/atm/state` does not exist will create new

### ⚙️ Miscellaneous Tasks

- Update all deps
- *(oma-pm)* Release oma-pm v0.15.8

## [1.2.8] - 2024-01-18

### ⚙️ Miscellaneous Tasks

- Release v1.2.8

## [oma-pm-v0.15.7] - 2024-01-18

### 🐛 Bug Fixes

- *(oma-pm)* Handle search result not only one provide

### ⚙️ Miscellaneous Tasks

- Release oma-pm v0.15.7

## [1.2.7] - 2024-01-14

### 🚀 Features

- *(error)* Improve `OmaDbusError::FailedConnectDbus` error handle

### 🐛 Bug Fixes

- *(subcmd/topic)* Fix `--opt-in` always retrun user select topic does not exist
- Fix(subcmd/topic): fix `--opt-in` always retrun user select topic does not exist (2)

### 🚜 Refactor

- *(main)* Allow `OmaDbusError` enum other error use empty error description

### ⚙️ Miscellaneous Tasks

- Fix version bump
- Add `cargo fmt` and `cargo clippy` check
- Fix clippy ci
- Improve clippy check
- Improve `build` step
- Update all deps
- MSRV Version 1.3.0
- Update `tabled` to 0.15
- Update all deps
- Release v1.2.7

## [1.2.6] - 2023-12-14

### 🐛 Bug Fixes

- *(utils)* Remove repeatable warn
- *(oma-pm)* If the dpkg state is corrupted, automatically run dpkg `--configure -a` to fix it

### ⚙️ Miscellaneous Tasks

- Release v1.2.6

### Meta

- Rebrand Omakase => oma

## [1.2.5] - 2023-12-11

### ⚙️ Miscellaneous Tasks

- Release v1.2.5

## [oma-refresh-v0.12.1] - 2023-12-11

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Release oma-refresh-v0.12.1

## [oma-fetch-v0.8.1] - 2023-12-11

### 🚀 Features

- Disable `i18n_embed` crate logger in non-debug mode

### 🐛 Bug Fixes

- *(oma-refresh)* Flat repo is not only path is '/'
- *(oma-fetch)* Fix download file list postition
- *(oma-fetch)* Fix `source` sort issue lead to local source download failed

### ⚙️ Miscellaneous Tasks

- Update all deps
- *(oma-fetch)* Release oma-fetch-v0.8.1

### I18n

- Fix invaild value in `can-not-parse-valid-until` translate

## [1.2.4] - 2023-12-09

### 🐛 Bug Fixes

- Fix no_check_dbus logic

### ⚙️ Miscellaneous Tasks

- Release v1.2.4

## [1.2.3] - 2023-12-09

### 🚀 Features

- *(utils)* Add some tips if `dbus_check` has error
- *(utils)* Add `no_check_dbus_warn` fn to display no check dbus warn

### 🐛 Bug Fixes

- *(utils)* Allow user use `--yes` to yes check battery status
- Only download one candidate of one package

### 🎨 Styling

- Fix with `cargo fmt`

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.15.5
- Update all deps
- Release v1.2.3

## [1.2.2] - 2023-12-06

### 🐛 Bug Fixes

- *(subcmd/history)* Not allow undo `undo` operation

### ⚙️ Miscellaneous Tasks

- Release v1.2.2

## [1.2.1] - 2023-12-06

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.2.1

## [oma-history-v0.3.0] - 2023-12-06

### 🚀 Features

- *(oma-history)* Raise `HistoryError::HistoryEmpty` if no such database, table and table is empty
- *(oma-history)* Switch history.db path to `/var/lib/oma/history.db`

### 🐛 Bug Fixes

- *(oma-history)* Raise `History::ParseDbError` if table parse failed

### ⚙️ Miscellaneous Tasks

- *(oma-history)* Release oma-history-v0.3.0

## [1.2.0] - 2023-12-05

### 🚀 Features

- Add `--sysroot` option to allow user change another rootfs
- Improve egg
- Re-add egg with license
- Add adapt egg scale
- Use `strerror` to display `io::Error` message
- Use locale language to output `io::Error`
- Chain `err.source()`
- Add `TRACE` to error output
- Improve `TRACE` output
- Use `tracing` crate to display oma message
- *(oma-console)* Use json to output debug mode message
- Debug mode uses `tracing` `fmt::layer()`.
- Style `TRACE` with color magenta
- *(oma-pm)* Add `OmaDatabaseError::NoAilurus`
- *(oma-pm)* Improve `OmaDatabaseError::Ailurus` output
- Move `OmaDatabaseError::NoAilurus` logic to oma frontend
- *(subcmd/list)* Default sort result output
- Improve 266 error output
- *(oma-pm)* Improve 266 egg again
- *(oma-topics)* Adapt `sysroot`
- *(oma-utils)* Adapt dpkg `sysroot`
- *(oma-utils)* Fill missing sysroot adapt
- Write oma history if get error
- *(utils)* Default return 1 if pkexec failed to get exit code
- *(oma-pm)* Sort summary package list
- *(oma-console)* Shorten progress bar length
- *(subcmd/utils)* Use `tokio.shutdown_background()` method to shutdown tokio
- Add `tokio-console` feature to monitor async runtime performance
- *(oma-console)* Set non global progress bar msg wide max length to 62
- Display due_to if download has error
- *(oma-history)* Log success/fail status
- *(subcmd/history,undo)* Add success/fail status display
- *(table)* Add two new line before print history table
- *(oma-pm)* Add unmet dependency version info if pkg does not exist
- *(config, args)* Add `--no-check-dbus` flag to argument and add `no_check_dbus` field to config ...
- *(args)* Add `--no-check-dbus` help
- *(error)* Log to terminal error struct with `--debug` flag
- *(topics)* Add flag `--refresh-mirror`
- *(oma-contents)* Add `parse_contents` fn to parse contents file

### 🐛 Bug Fixes

- *(subcmd)* Fix some missing `sysroot` argument include subcommand
- *(oma-pm)* Check AptConfig `Dir` size
- Fix always display egg
- *(oma-pm)* Fix default sysroot path argument
- *(oma-refresh)* Fix a typo
- *(error)* Fix output message infomation
- *(oma-console)* Fix new line terminal lenth < 80
- *(oma-contents)* Fix progress bar always mini terminal mode
- *(oma-refresh)* Fix InRelease verify rootfs dir
- *(oma-console)* Fix terminal width is 90 new line progress bar
- *(oma-refresh)* Fix create `/var/lib/apt/lists` if does not exist
- *(oma-utils)* Fix dpkg `sysroot` argument
- *(subcmd/upgrade)* Fix sucess message display
- *(subcmd/utils)* Fix return error
- *(oma-contents)* Fix unexcept `oma files pushpkg` result and unexcept `oma provides XXX`
- `main()` init tracing logger
- *(oma-pm)* Canonicalize user input `sysroot`
- *(error)* Only display reqwest error url filename if url filename length <= 256
- *(oma-pm)* Add missing `oma_utils` crate feature to fix build
- *(error)* Make rust-analyzer happy
- *(error)* Fix build after linted i18n string and id
- *(args)* Add missing `-i` `-u` long argument in `oma list`
- *(error)* Fix `download-failed` message is file name does not exist
- *(oma-console)* Adjust progress bar align
- *(main)* Fix compile
- *(oma-refresh)* Do not read file if InRelease parse failed
- *(oma-refresh)* Do not display data if `InRelease` parse failed
- *(oma-refresh)* Do not log any `InReleaseSyntaxError` field
- *(subcmd/show)* Fix a typo: automatc => automatic
- *(oma-pm)* Fix `oma show` APT-Source field redundant line

### 🚜 Refactor

- *(oma-topics)* [**breaking**] Improve error type
- *(oma-fetch)* Improve debug error message
- *(oma-fetch)* [**breaking**] Use thiserror `transparent` reqwest error
- *(oma-console)* [**breaking**] All use `io::Result` replace `OmaConsoleResult`
- *(oma-contents)* [**breaking**] Save `OmaContents::ExecuteRgFailed` error context
- *(oma-fetch)* [**breaking**] Improve `DownloadError` error context
- *(oma-pm)* [**breaking**] Improve `OmaAptError` error context
- *(oma-refresh)* Save `RefreshError` error context
- Output error context
- Improve error context to nice
- Undo some vscode stupid changes
- *(oma-pm)* [**breaking**] Split `OmaAptError::IOErrpr`
- *(oma-refresh)* [**breaking**] Split `VerifyError::IOError`
- *(oma-contents)* [**breaking**] Split `OmaContentsError:IOError`
- *(oma-contetns)* Split `OmaContents::IOError`
- *(oma-refresh)* [**breaking**] Split `RefreshError::IOError`
- *(oma-topics)* [**breaking**] Split `OmaTopicsError::IOError`
- *(oma-contents)* [**breaking**] Split `OmaContents::IOError` woth `contains-without-rg` feature
- Refactor display error step to `display_error` fn
- Move `history.rs` logic to `oma-history` crate
- *(oma-pm)* Use `small-map` to improve `get_deps` and `get_rdeps` performance
- *(oma-refresh)* Use `smallvec` and `small-map` crate to improve performance
- *(oma-contents)* Fix deprecated function
- *(oma-topics)* `add` fn no need to use `async`
- *(oma-pm-operation-type)* From `oma-pm` move some type to this crate
- *(oma-pm)* Pub use `oma_pm_operation_type`
- *(oma-console)* Use `feature` to split features
- *(oma-utils)* [**breaking**] Split `zError` to `OmaDbusError`
- *(oma-history)* Refactor history database struct
- *(oma-history)* Do not `unwrap`
- *(oma-history)* Use a more granular approach to querying the database
- *(config)* Set default fn to `const`
- *(oma-history)* Only get once result in `find_history_by_id` fn
- *(oma-history)* No need to `clone` result
- *(oma-refresh)* [**breaking**] RefreshError log inrelease file location
- *(oma-contents)* Adapt new `winnow` crate

### 🎨 Styling

- Fix with `cargo clippy`
- Run cargo clippy
- Fix with `cargo fmt`
- Remove useless line
- Fix with `cargo fmt`
- Improve code style
- Fix with `cargo clippy`
- *(oma-history)* Fix with `cargo clippy`
- Fix style with `autopep8`
- Fix with `cargo clippy`
- Fix with `cargo clippy`
- Fix with `cargo fmt`
- Fix with `cargo fmt`

### 🧪 Testing

- *(oma-console)* Fix `msg` example

### ⚙️ Miscellaneous Tasks

- MSRV Version 1.2.0-alpha.0
- *(oma-refresh)* Release oma-refresh v0.7.0
- *(oma-pm)* Release oma-pm v0.10.0
- *(oma-pm)* Release oma-pm v0.10.1
- *(oma-console)* No need to use `thiserror` crate
- *(oma-contents)* No need to use `which` crate
- Bump multi crates
- *(oma-contents)* Release oma-contents-v0.5.0
- Bump multi crates
- *(oma-contents)* Release oma-contents v0.6.0
- Bump multi crates
- *(oma-refresh)* Release oma-refresh v0.10.0
- Bump multi crates
- *(oma-console)* Release oma-console v0.8.1
- *(oma-refresh)* Release oma-refresh v0.11.1
- *(oma-console)* Remove unnecessary dep
- Bump multi crates
- *(oma-utils)* Release oma-utils-v0.5.1
- *(oma-history)* Add description
- *(oma-history)* Use GPLv3
- Update all deps
- *(oma-pm)* Release oma-pm-v0.14.1
- *(oma-console)* Release oma-console-v0.8.3
- *(oma-contents)* Release oma-contents-v0.6.1
- Release oma-p-v0.15.0; oma-history-v0.1.2
- *(oma-pm-operation-type)* Add description and license
- *(oma-pm)* Release oma-pm-v0.15.1
- *(oma-history)* Use MIT LICENSE
- *(oma-history)* Release oma-history-v0.1.3
- Bump multi crate
- Bump multi crates
- Default disable `egg` feature
- *(oma-history)* Release oma-history-v0.2.0
- *(oma-pm)* Release v0.15.4
- *(oma-contents)* Release oma-contents-v0.7.0
- *(oma-console)* Release oma-console-v0.9.1
- *(oma-refresh)* Release oma-refresh-v0.12.0
- *(oma-contents)* Release oma-contents-v0.7.1
- Release v1.2.0

### Completions

- Remove egg `--ailurus` completion

### Config

- Add `refresh_pure_database` field

### I18n

- Re-add lost translate entry
- Move some string to ftl file
- Lint strings

### Reffactor

- *(subcmd)* Move undo.rs to history.rs

### Script

- Fix if stmt is `fl!\\(\\n\\s*`
- Fix script run not in pwd

### Script/clean-unused-translate-entry

- Run `autopep8`

### Script/clean_unused_translate_entry

- Fix style with `pylint`

### Scripts

- Lint again

## [1.1.8] - 2023-11-23

### ⚙️ Miscellaneous Tasks

- Release v1.1.8

## [oma-pm-v0.9.1] - 2023-11-23

### 🚀 Features

- *(utils)* Default return 1 if pkexec failed to get exit code

### 🐛 Bug Fixes

- *(subcmd/show)* Fix `oma show` with multi arguments
- *(subcmd/show)* Exit with code 1 if no result
- *(oma-pm)* Fix if has dependency but no require version ...

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.9.1

## [1.1.7] - 2023-11-03

### 🚀 Features

- Support write log with oma return error

### 🐛 Bug Fixes

- *(error)* Fix `OmaAptError::FailedTODownload` translate
- Fix compile error
- Fix a typo
- *(subcmd/utisl)* Fix `history-tips-2` display

### 🎨 Styling

- Fix with `cargo clippy`

### I18n

- Translate `OmaTopicsError::SerdeError`

## [1.1.6] - 2023-10-23

### 🎨 Styling

- Fix with `cargo clippy`

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.1.6

## [oma-pm-v0.9.0] - 2023-10-23

### 🐛 Bug Fixes

- *(oma-pm)* Download packages failed return error

### ⚙️ Miscellaneous Tasks

- *(oma-pm)* Release oma-pm-v0.9.0

## [1.1.5] - 2023-10-22

### 🐛 Bug Fixes

- `command-not-found` subcmd '-A' unexpected argument

### ⚙️ Miscellaneous Tasks

- Remove useless file
- Update all deps
- Release v1.1.5

## [1.1.4] - 2023-10-18

### 🐛 Bug Fixes

- *(main)* Fix plugin execute path

### ⚙️ Miscellaneous Tasks

- Release v1.1.4

## [1.1.3] - 2023-10-18

### 🐛 Bug Fixes

- *(args)* Fix get plugins logic

### 🎨 Styling

- Fix with `cargo fmt`

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.1.3

### I18n

- *(en-US)* Fix unsync string

## [1.1.2] - 2023-10-17

### 🚀 Features

- Add features `sequoia-openssl-backend` ...

### ⚙️ Miscellaneous Tasks

- Release v1.1.2

## [oma-refresh-v0.6.10] - 2023-10-17

### 🚀 Features

- *(oma-refresh)* Add `sequoia-openssl-backend` feature to allow user use openssl as sequoia backend

### ⚙️ Miscellaneous Tasks

- *(oma-refresh)* Remove useless file
- *(oma-refresh)* Release oma-refresh-v0.6.10

## [1.1.1] - 2023-10-17

### 🎨 Styling

- *(oma-utils)* Fix with `cargo clippy`

### ⚙️ Miscellaneous Tasks

- Release v1.1.1

### I18n

- "N package(s) will be X" X -> Xed

## [oma-utils-v0.2.0] - 2023-10-17

### 🐛 Bug Fixes

- *(oma-refresh)* Fix get suite name logic
- *(oma-fetch)* Fix `oma-refresh` refresh database always download database
- *(oma-fetch)* Fix retry download wrong progress
- *(oma-pm)* Fix pkg is marked hold logic

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps
- *(oma-refresh)* Release oma-refresh-v0.6.8
- *(oma-fetch)* Release v0.5.1
- Update all deps
- Bump multi crates

## [1.1.0] - 2023-10-16

### ⚙️ Miscellaneous Tasks

- Release v1.1.0

## [oma-refresh-v0.6.7] - 2023-10-16

### 🚜 Refactor

- *(oma-fetch)* Refactor and improve Error kind enum

### ⚙️ Miscellaneous Tasks

- `inquire` use `console` backend to remove unnecessary dependencies
- Update all deps
- Bump multi crates

## [oma-utils-v0.1.9] - 2023-10-16

### 🚀 Features

- *(oma-console)* Use `icu_segmenter` crate to help oma calculator text breakpoint

### ⚙️ Miscellaneous Tasks

- Bump multi crate

## [oma-refresh-v0.6.5] - 2023-10-15

### 🚀 Features

- *(subcmd/utils)* Add more unexpected char

### 🐛 Bug Fixes

- *(oma-console)* Fix `bar_writeln` logic mistake
- *(oma-fetch)* Download error should finish progress bar

### ⚙️ Miscellaneous Tasks

- *(oma-console)* Release oma-console-v0.4.1
- Update all deps
- Bump multi crates

### I18n

- Do not ask user to discern unexpected behaviours
- *(en-US)* Lint UI strings

## [1.1.0-beta.9] - 2023-10-14

### 🐛 Bug Fixes

- *(subcmd/show)* Fix `other_version` overflow
- *(subcmd/show)* Return 1 if can't find package

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.1.0-beta.9

## [1.1.0-beta.8] - 2023-10-13

### ⚙️ Miscellaneous Tasks

- Release v1.1.0-beta.8

## [oma-utils-v0.1.8] - 2023-10-13

### 🐛 Bug Fixes

- *(oma-pm)* Fix `oma download` download unavailable candidate package
- *(subcmd/search)* Fix terminal output

### 🚜 Refactor

- *(oma-console)* Refactor terminal `writeln` and `bar_writeln` function

### 🎨 Styling

- Fix with `cargo clippy`

### ⚙️ Miscellaneous Tasks

- Update all deps
- *(oma-console)* Release oma-console-v0.4.0
- Bump multi crates

## [1.1.0-beta.7] - 2023-10-13

### 🚀 Features

- *(oma-fetcher)* Add download local source feature
- *(oma-refresh)* Init
- *(oma-pm)* Add OmaDatabase impl
- *(oma-pm)* Add query_from_version and query_from_branch function
- *(oma-pm)* Add virtual pkg support and query_from_branch function
- *(oma-refresh)* After the database refresh is complete fsync
- *(oma-fetch)* Add retry times
- *(oma-fetch)* Improve try_http_download error handle
- *(oma-pm)* Add OmaApt struct
- *(oma-refresh)* Add translate
- *(oma-pm)* Support local package install
- Add remove package feature
- *(oma-pm)* Remove pkg add protect bool
- *(oma-pm)* Add opration.rs ....zzz
- Api change to support multi mirror download
- *(oma-pm)* Fill of remove() function
- *(oma-console)* Use DEBUG globar var to store yes/no display debug message
- *(oma)* Retry 3 times for oma upgrade
- Dry_run mode do not refresh and display pending view
- Improve debug output
- Checksum pure database
- Download compress file after extract
- *(oma-contents)* Adapt oma-refresh changes
- *(oma-pm)* Find unmet dep only display layer 1 dep
- Improve oma rdepends output
- *(oma-utils)* Add mark_version_status function
- *(oma-pm)* Add mark_install_status function
- Oma mark check root
- *(oma)* Oma history is back
- Add Size-delta field on oma history; improve file output
- *(oma-utils)* Add take walk lock and check battery feature
- *(oma-fetch)* Display done message if env is not atty
- *(oma-pm)* Do not display apt progress if not is terminal
- *(oma)* Table display remove size delta message
- Drop oma download --with-deps feature
- Terminal output pending ui message
- Move oma history undo to oma undo
- Improve write_history_entry performance
- Add operation package version
- Add oma history 'replay' table feature
- *(oma-refresh)* Refresh done no need to manual fsync
- *(oma-utils)* Use feature to select abstract code
- Topic function do not compile by no aosc feature
- Use sqlite to write entry to history database
- Add oma history/undo date display
- *(oma-pm)* Use chrono to get and parse datetime
- Add oma download download done tips
- *(oma-contents)* Use ContentsEvent to send translate msg to oma binary
- Improve contents-may-not-be-accurate output message style
- Add oma list/show --all option
- Add progress spinner in search feature
- Add progress spinner in clean feature
- Not allow oma undo fix-broken
- Oma operation do not display pending ui review message in terminal
- Oma remove table detail header left align
- New line pending ui
- If not is_terminal disable dpkg read database indicatif
- Do not ring if not is_terminal
- *(oma-refresh)* Do not fetch database with the same content but in a different compression format
- *(oma-contents)* Add aosc feature and move some logic to oma binary
- *(oma-fetch)* Add translate
- *(oma/search)* New color theme!
- Adjust search color theme
- Adjust search color theme again
- Add --no-color option to do not output color to terminal ...
- Oma install install_recommends arg conflict no_install_recommends arg
- Exit code to 0 if allow ctrlc
- *(oma-fetch)* Switch to callback event, no more indicatif in oma-fetch
- *(oma-refresh)* Add topic closing message
- Add --no-progress option to no output progress bar to terminal
- Fill oma --no-progress feature
- *(oma-pm)* Add no_progress option to control dpkg use-pty value
- *(oma-fetch)* Add DownloadEvent::AllDone to allow control global progress bar finish and clear
- Support plugin like: oma-mirror, you can use oma mirror to run it
- Oma history pending ui q after back to menu
- Check expected pattern in oma depends/rdepends/list/search
- Use timestemp to store history datetime
- Drop purge-as-default and differentiate oma remove and oma remove --remove-config (purge)
- Improve oma download failed error message
- *(oma-refresh)* Add download_compress argument to allow user download prue database; do not cipunited in oma-refresh
- Add download_pure_db option in /etc/oma.toml config file
- *(oma-topics)* Sort topic list
- Add `oma purge` subcommand, but hide this.
- *(oma-pm)* Control download package message length
- *(oma-fetch)* Clean up the ProgressBar when an error is encountered
- *(oma-pm)* Allow user `oma remove package` after same operation to purge config
- *(oma-contents)* No more fallback to grep crate when no ripgrep binary is available
- *(oma-contents)* Fill of command-not-found feature with `contents-without-rg`
- *(oma-contents)* Use `--pcre2` to improve rg search contents performance
- *(oma-topics)* Add more debug message
- Set `oma topics` page size to 8 to fix small terminal
- Try to calculator `oma topics` page size
- *(oma-topics)* Write enabled topic to sources.list before check is available in mirror
- Disable progress bar if debug mode is enabled
- Do not print to terminal if user abort operation
- *(table)* Add new line before print review message in terminal
- *(subcmd/list)* Changes ...
- *(subcmd/search)* Improve `oma search` output
- *(args)* Use `/usr/libexec` to init oma plugins

### 🐛 Bug Fixes

- *(oma-contents)* No result retutn error
- *(oma-refresh)* Clear decompress progress bar
- *(oma-refresh)* Do not fetch repeatedly source
- *(oma-refresh)* Do not always decompress contents every refresh
- *(oma-refresh)* Adapt new oma-fetch api
- *(oma-pm)* Fix ask is essential after will remove package
- *(oma-pm)* Fix local package install
- *(oma-pm)* Fix a typo
- *(oma-console)* Fix logger marco multi use
- *(oma)* Wait searcg pager exit
- *(oma-contents)* Wrong Querymode match
- Oma show multi package query
- Oma list no package argument output
- Oma show APT-Spirces display
- --{no-,}install-{recommend,suggest}
- Merge master 5d6d2e82f0125d4c8f871228b8cbeb3de53260f1 change
- Fix oma pending ui table align
- Fix u64 overflow to oma remove pkg to failed
- *(oma-contents)* Fix space in file
- Use version.arch() replaced pkg.arch() to get package arch
- Oma upgrade add find breaks logic
- Tokio::runtime inner can not run tokio::runtime
- Try to fix some break item wrong result
- Try to fix unmet dep wrong error output
- Check env is root
- Check mark pkg is installed
- *(oma-pm)* Add loop count check in url_no_escape function
- *(oma)* Oma history wait pager to exit
- Oma history command navigation
- Remove useless argument in oma history to fix build
- *(oma-fetch)* Use progress bar println to display message
- Fix flat-repo fetch
- *(oma-pm)* Fix oma topics cancal select topic downgrade pkg
- Handle could not find any package for keyword
- Fix-broken check operation is emppty
- Seek file to start if file exist
- Fix oma upgrade loop not return
- Undo need root; history no need root
- *(oma-pm)* Fix get selected pkgs logic
- Add missing oma list --all tips
- Improve method to get package version branches
- Oma list get all mirror list files branches
- Oma list packages arg is empty do not display -a tips
- *(oma-pm)* Add oma-utils feature to fix compile
- *(oma-pm)* Fix history text log Commandline display
- *(oma-pm)* Oma show if package not downloable display unknown
- Oma list use the correct method to get branches
- *(src/history)* Do not write history if running in dry-run mode
- *(oma-refresh)* Fix local mirror host name get
- *(oma-refresh)* Fix local mirror download url get
- Fix missing error match
- *(oma-fetch)* Fix local mirror package fetch
- Do not write terminal twice message in --yes mode
- *(oma-pm)* Mark_delete after resolve deps to fix autoremove packages
- A typo searching -> cleaning
- Oma download display download to path
- Try to fix version source check
- Oma list only display upgradable in installed version
- *(oma-pm)* Mark_install use branch to compare source
- Allow multi value set at the same time
- Fix mips64r6el contents fetch ...
- *(oma-fetch)* Fix warning message display
- *(oma-contents)* Do not search zip contents in mips64r6el
- *(oma-pm)* Fix oma install downgrade package mark install logic
- *(oma-pm)* Mark reinstall protect package mark
- *(oma-pm)* Fix oma fix-broken with different results from apt install -f
- Fix oma history ui string
- *(oma-pm)* Fix user remove package also display autoremove tag
- *(oma-pm)* Fix real_pkg function if pkg has version and provides
- *(oma-fetch)* Escape url try to fix can not download '+' in package name packages
- Do not display review message with --yes
- *(utils)* I18n message in progress bar display
- *(subcommand/command-not-found)* Do not display empty error message
- *(oma-pm)* Do not download empty list
- Execute plugin logic
- *(oma-refresh)* Fix topic does not exist auto disable topic
- *(oma-topics)* Do not display no need remove topic message
- *(oma-refresh)* Do not displat topic closeing message in non-topic mirror
- *(oma-pm)* Query branch and version set candidate to fix oma show with branch/version
- *(oma-pm)* Check branch array boundary
- Fix a typo expected -> unexpected
- Typo Installl -> Installed
- Use oma history/undo local datetime display
- Fallback timestemp to 0 if invaild timestemp
- Improve due_to message
- Improve command-not-found error output message
- *(oma-fetch)* Request head failed clear progress bar
- *(oma-refresh)* Calc decompress download total size
- *(oma-refresh)* Fix panic if InRelease has Compress file type and have no decompress file type
- *(oma-refresh)* Fix wrong contents download size
- Fix build without aosc feature
- *(oma-pm)* Do not count marked keep package
- *(subcmd/command-not-found)* Check err message is empty
- Do not display due to if return error and error is empty
- *(oma-contents)* /bin and /sbin search
- *(oma-console)* Fix progress bar align by global progress bar
- *(subcmd/list)* Fix package always display branch `unknown`
- Dueto -> info
- *(oma-contents)* Fix --features contents-without-rg build
- *(oma-contents)* Use contetns db created time to check database is accurate
- *(oma-contents)* Throw Error before clear progress bar
- *(oma-contents)* Fix features `no-rg-binary` binary only search
- *(oma-contents)* Fix `no-rg-binary` feature build
- Fix `contents-without-rg` feature build
- *(oma-topics)* Fix wrong error type
- *(oma-pm)* Drop unnecessary new line in oma show
- *(oma-topics)* Refresh online topics to fix `oma topics` always return 1
- *(oma-topics)* Try to fix multipie sources.list entries remove duplicate topic entries
- Drop useless error enum
- *(oma-topics)* Do not display duplicate topic entries
- *(oma-ferch)* Fix fetch local source position
- *(subcmd/topics)* If terminal heigh < 4, page size must be 1
- Fix find unmet dep pending ui display
- *(table)* Fix print table to less and stderr conflict issue
- *(oma-pm)* Add `OmaAptError::PkgUnavailable` to handle if package:version unavailable
- *(subcmd/list)* Handle if package is unavailable in mirror
- *(subcmd/command-not-found)* Due_to -> info
- Add missing translate to fix build
- *(subcmd/search)* Fix search open less
- *(oma-fetch)* Retry times count start from 1 not 0
- *(table)* Unmet dep ui always use PAGER
- *(oma-pm)* Fix local source package install
- *(oma-pm)* Fix refactor mistake ...
- *(subcmd/list)* Do not panic if has package but package no version (will this happen?)
- *(oma-contents)* Drop `rg` argument `--pcre2` ...
- *(oma-pm, oma-fetch)* Fix `oma download` download package with checksum
- *(oma-fetch)* Fix reset global bar position with download failed
- *(subcmd/topics)* Fix display line length and terminal width issue
- *(oma-pm)* Fix pkg section is empty will oma panic
- *(subcmd/search)* Package description should align

### 🚜 Refactor

- Done for contents.rs to oma-contents crate
- Add crate oma-console
- *(oma-console)* Abstract tips and has_x11
- Add oma-topics crate
- *(oma-console)* Do not const Writer::default as WRITER
- *(oma-fetcher)* Add todo
- *(oma-fetcher)* Done for http(s) source download
- *(oma-console)* Add progressbar style
- *(oma-fetch)* Do not handle result in start_download function
- *(oma-refresh)* Done for decompress
- *(oma-refresh)* Done 1
- *(oma-pm)* Pkg.rs => oma-pm
- *(oma-pm)* Api adjust
- *(oma-pm)* Improve lifetime logic
- *(oma-i18n)* I18n -> oma-i18n
- *(oma-pm)* Done for operation_map
- *(oma-pm)* Done OmaApt::commit function
- *(main)* Install/remove/upgrade/refresh done
- *(oma-pm)* Improve api design
- --install-dbg flag is back
- More args back
- *(oma)* Pending ui is back!
- Add remove/upgrade pending ui
- Oma remove after autoremove feature is back
- *(oma-console)* Improve debug marco
- *(oma)* Refresh info is back
- *(oma)* Oma show is back!
- Oma search is back
- *(oma-contents)* Redesign api
- Oma files is back
- Oma provides is back
- Oma fix-broken is back
- Oma pick is back
- Command-not-found is back
- Oma list is back
- Oma clean is back
- Oma pkgnames is back
- Move logic to command.rs
- Remove useless code
- Move fix_broken function to command.rs
- Unmet dep ui is back
- Do some todo
- Check disk size is back
- Oma depends is back
- Dry-run mode is back
- Oma rdepends is back
- Some display info is back
- Already-installed message is back
- Yes warn is back
- Fill of error translate (50%)
- Fill of error translate todo
- *(oma-contents)* Lint code
- Add some error translate
- Add some error translate (90%?)
- *(oma-topics)* Use async
- *(oma-fetch)* Use builder api design
- Remove useless file; lint
- Fill of error output (100%)
- Oma topics is back
- Improve oma topics downgrade logic
- *(oma-topics)* Inner reqwest::Client
- Remove useless code
- Add topics cli ui string
- Root check is back
- *(oma-utils)* Can set pkgs as argument in mark_version_status function
- Oma mark is back
- Write history feature is back
- *(oma-utils)* Re-abstract code
- Log history database is back
- History tips (oma undo) is back
- Abstract some step to normal_commit function
- Do not open file twice
- Abstract some step to dbus_check function
- Improve list_history performance
- Try to use Cow<str> to push vec (improve performance?)
- *(bin/history)* No need to query id
- Oma read oma.toml config feature is back
- *(src/main)* No need to clone some string
- *(oma-contents)* Callback no need to use Sync trait
- *(src/table)* Use tabled builder to create new table
- *(oma-pm)* No need to clone some var in search function
- *(oma-pm)* Use version.is_downloadable to check package version is downloadable
- *(oma-utils)* Move oma-pm url_no_escape function to oma-utils
- *(oma-refresh)* Adapt new oma-fetch api
- *(src/command)* Adapt oma-fetch new API
- *(subcommand)* Move command.rs to subcommand module
- *(oma-fetch)* Refactor try_donwload function to SingleDownloader impl to fix clippy
- Improve arg parser logic
- *(oma-refresh)* Improve closed topic logic
- *(oma-refresh)* Improve closed topic logic again
- *(oma-pm)* No need to set candidate to query package
- *(oma-refresh)* Refactor InRelease::new to no need use spawn_blocking
- *(oma-console)* Oma_spinner and oma_style_pb function inner unwrap
- Gloal progress bar use prefix to display 'Progress'
- *(oma-fetch)* Use Arc to clone callback
- *(oma-fetch)* Some var no need to clone
- *(oma-fetch)* Refactor clone (1)
- *(oma-fetch)* Refactor clone (2)
- *(oma-contents)* Use Arc<T> and Arc<Mutex<T>> to clone some var
- Some var use refrence
- Some var no need to clone
- Use `iter.count()` replace `iter.collect::<Vec<_>>().len()`
- *(subcmd/show)* Use `pkgs.len()` replace `pkgs.iter().count()`
- *(oma-refresh)* Improve read repo data performance
- *(oma-refresh)* Remove `update_db` function unnecessary steps
- *(oma-contents)* Use winnow to improve parse contents performance
- *(oma-contents)* No use `rg --json` output to improve performance
- *(oma-contents)* No format file: path string in oma-contents crate
- Replace some `sort()` to `sort_unstable()`
- *(oma-contents)* No need to use `Arc<Mutex<T>>` in Contents paths var wrapper to improve performance
- *(oma-contents)* No need to let multi times `contain_contents_names`
- *(oma-contents)* Use `BufReader` + `rayon` to read contents single line
- *(oma-contents)* No need to use `Arc<Mutex<Vec<(String, String)`, use `.par_iter()...collect::<Vec<_>>`
- Move `ask_user_do_as_i_say` function from `oma-pm` crate to oma binary code
- *(oma-refresh)* Drop unnecessary clone
- *(oma-topic)* Drop unnecessary clone
- *(oma-pm)* Refactor `PkgInfo` struct to improve `oma search` performance
- *(oma-pm)* Improve `PkgInfo` struct and impl
- *(oma-fetch)* Improve logic
- *(oma-pm)* No need insert `section` string to `SearchEntry` struct in `search_packages` function

### 📚 Documentation

- Add some comment
- *(oma-refresh)* Add changelog
- Add some comment
- *(README)* Update README
- Fix markdown syntax error
- Fix asciinema preivew
- *(subcmd/topics)* Add comment

### 🎨 Styling

- Use cargo-fmt to format code
- *(oma-pm)* Remove useless line
- Run cargo clippy and cargo fmt to lint code
- Run cargo clippy and cargo fmt to lint code
- Remove useless code
- Run cargo clippy and cargo fmt to lint code
- Run cargo clippy and cargo fmt to lint code
- Run cargo clippy and cargo fmt to lint code
- Use cargo-fmt and cargo-clippy to lint code
- Use cargo-fmt and cargo-clippy to lint code
- Use cargo-fmt to format code
- Run cargo clippy and cargo fmt to lint code
- Lint code style
- Use cargo-fmt to format code
- *(oma-contents)* Lint code
- *(oma-contents)* Use cargo-fmt and cargo-clippy to lint code
- Use cargo-fmt to format code
- *(oma-pm)* Improve code style
- *(oma-pm)* Improve code style
- Run cargo clippy and cargo fmt to lint code
- Use cargo-fmt to format code
- *(oma-fetch, oma-pm, command)* Use cargo-fmt to format code
- *(oma-fetch)* Improve code style
- Run cargo clippy and cargo fmt to lint code
- Lint build.rs
- Run cargo clippy and cargo fmt to lint code
- Fix var word typo
- Use cargo-fmt and cargo-clippy to lint code
- Use cargo clippy to lint code
- *(oma-refresh)* Fix clippy
- Fix clippy
- Run cargo clippy and cargo fmt to lint code
- *(subcmd/utils)* Use `array[..N]` replace `array[0], array[1], ..., array[N-1]` to imrpove code style
- Run cargo clippy and cargo fmt to lint code
- Use `cargo clippy` to lint code
- *(oma-pm)* Fix clippy
- Lint code use `cargo clippy`
- *(oma-pm)* Fix with `cargo clippy`
- Fix style use `cargo clippy`
- *(oma-fetch)* Fix style with `cargo fmt`
- Adjust `Cargo.toml` style
- *(oma-topics)* Adjust code style
- Run cargo clippy and cargo fmt to lint code

### 🧪 Testing

- *(oma-pm)* Add test_branch_search
- *(oma-pm)* Add example
- *(oma-pm)* Refactor
- *(oma-pm)* Refactor again
- *(oma-pm)* Add download pkgs example
- *(oma-pm)* Example texlive -> vscodium to save your sweet time
- *(oma-pm)* Update example
- *(oma-pm)* Fix example

### ⚙️ Miscellaneous Tasks

- *(oma-fetch)* Update example
- *(oma-refresh)* Fmt example
- Update all deps
- Update all deps
- *(oma-refresh)* Drop useless dep
- Remove useless dep
- *(oma-console)* Fill of comment
- *(oma-console)* Add changelog
- *(oma-console)* Add desc
- *(oma-console)* Use MIT license
- *(oma-contents)* Add desc and license (MIT)
- *(oma-contents)* Add changelog
- *(oma-contents)* Set oma-console version as 0.1.0
- *(oma-contents)* Fill in comments
- *(oma-utils)* Add desc and LICENSE (MIT) and comment
- *(oma-utils)* Add changelog
- *(oma-pm)* Switch to oma-apt (own rust-apt fork)
- *(oma-fetch)* Fill in comment, desc, license
- *(oma-pm)* Add desc and license
- *(oma-fetch)* Add changelog
- *(oma-pm)* Fill in comment
- *(oma-pm)* Add changelog
- *(oma-pm)* Fix license
- *(oma-topics)* Use oma-apt-sources-list crate (own fork)
- Update cargo lock
- *(oma-topics)* Fill in comment, desc and license
- *(oma-topics)* Add changelog
- *(oma-refresh)* Use oma-debcontrol crate (own fork)
- *(oma-utils)* 0.1.3
- Update all deps and cargo clippy
- Adjust some deps
- Adjust some deps (again)
- *(oma-refresh)* Switch to chrono
- *(oma-contents)* Switch to chrono
- *(oma-contents)* Adjust chrono features
- Adjust nix feature
- *(oma-pm)* Update indicium to 0.5.1
- Update all deps
- *(oma-refresh)* Add license and desc
- *(oma-refresh)* Fill some dep version
- *(oma-topics)* Bump to 0.1.2
- Update i18n-embd, i18n-embd-fl and rust-embd to newest version
- *(oma-console)* Release 0.2.0
- Bump all dep oma-console version
- *(oma-refresh)* Release 0.4.2
- *(oma-refresh)* Release 0.5.0
- Set oma-console version to fix cargo publish oma-refresh
- *(oma-refresh)* Release 0.5.1
- Update all deps
- *(oma-refresh)* Release 0.5.2
- Set oma version as 1.0.9999
- Update all deps
- *(oma-contents)* Releasse 0.2.0
- *(oma-contents)* No need to use `grep` ceate
- Update all deps
- *(oma-contents)* Set `which` crate to optional
- *(oma-topics)* Remove unnecessary dep
- *(oma-contents)* Release 0.3.0
- *(oma-topics)* Release 0.2.0
- *(oma-refresh)* Release 0.6.0
- *(oma-pm)* Release 0.5.0
- *(oma-pm)* Release 0.6.0
- Release oma v1.1-beta1
- *(oma-topics)* Release v0.3.0
- Release oma v1.1.0-beta2
- *(oma-fetch)* Release v0.3.0
- Release oma-v1.1.0-beta.3
- Release v1.1.0-beta.4
- *(oma-fetch)* Release oma-fetch-v0.3.4
- *(oma-console)* Release oma-console-v0.3.0
- Release multi crates
- *(oma-contents)* Release oma-contents-v0.3.1
- Release v1.1.0-beta.5
- Switch `nix` crate to `rustix`
- *(oma-refresh)* Pub use `oma_fetch::DownloadEvent`
- *(oma-refresh)* Release oma-refresh-v0.6.3
- *(oma-fetch)* Release oma-fetch-v0.3.5
- *(oma-pm)* Release oma-pm-v0.7.1
- Update all deps
- Release v1.1.0-beta.6
- Update indicium to v0.5.2
- Update all deps
- *(oma-pm)* Release oma-pm-v0.7.2
- Release v1.1.0-beta.7

### I18n

- Use symlink to create oma_refresh.ftl
- Remove useless full comma
- Improve battery tips
- Add searching and cleaning translate
- Remove useless space
- *(en-US)* Lint translation strings
- *(zh-CN)* Lint translation strings
- *(en-US)* Fix topic prompt
- Improve UI string for pkg-unavailable
- *(zh-CN)* 必备组件 => 关键组件
- Remove useless translate string

### Refacor

- Abstract resolve() function

### Refactrr

- *(oma-refresh)* `collect_download_task` function no need `async`

### Sytyle

- *(oma-topics)* Fmt

## [1.0.8] - 2023-10-10

### ⚙️ Miscellaneous Tasks

- Update all deps
- Release v1.0.8

## [1.0.7] - 2023-08-18

### 🐛 Bug Fixes

- Switch flate zlib backend to default to fix loongarch64 build

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.6] - 2023-08-16

### 🐛 Bug Fixes

- Do not check inrelease valid_until and date field in flat repo
- Fix flat repo download path
- Try to fix flat repo path (2)

## [1.0.5] - 2023-08-09

### 🚀 Features

- Do not add -dbg package to dep issue item

### 🐛 Bug Fixes

- Oma upgrade add find breaks logic
- Try to fix unmet dep wrong error output

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.4] - 2023-08-07

### 🐛 Bug Fixes

- Use version.arch() replaced pkg.arch() to get package arch

### 🚜 Refactor

- *(oma)* Lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.3] - 2023-08-06

### 🐛 Bug Fixes

- *(oma)* Fix glob in oma remove

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.2] - 2023-08-06

### 🐛 Bug Fixes

- *(contents)* Fix space in file

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.1] - 2023-08-04

### 🐛 Bug Fixes

- (zh_CN) 职守 => 值守

### ⚙️ Miscellaneous Tasks

- Update all deps

## [1.0.0] - 2023-08-04

### 🐛 Bug Fixes

- Correct typos in oma.rs

### ⚙️ Miscellaneous Tasks

- Release 1.0
- Update all deps

## [0.45.6] - 2023-07-26

### 🚀 Features

- Display command-not-found error message

### 🐛 Bug Fixes

- *(download)* If downloaded wrong file size > right size, reset global bar
- *(download)* Fix logic mistake
- *(oma)* Do not display empty error message

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.45.5] - 2023-07-14

### 🐛 Bug Fixes

- *(formatter)* Fix find_unmet_dep logic

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

### I18n

- Fix unmet-dependencies ui string

## [0.45.4] - 2023-07-12

### 🐛 Bug Fixes

- *(db)* Do not scan closed topics multi times
- *(db)* Fix not topics url 404 not found error handle
- *(db)* Fix build
- *(db)* Do not download error message write to due to
- *(topics)* Handle if atm.list does not exist

### 🚜 Refactor

- *(db)* Improve logic
- *(topics)* Use tokio::task::spawn_blocking to run scan sources.list
- *(topics)* Improve sources.list scan error handle

### 🎨 Styling

- *(topic)* Clean up useless code
- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

### I18n

- *(zh-CN)* 校验和验证 -> 完整性验证

## [0.45.3] - 2023-07-07

### 🐛 Bug Fixes

- Pin grep-cli version to fix rustc 1.68.2 build

### ⚙️ Miscellaneous Tasks

- Update all deps

### I18n

- Fix some zh-CN wrong ui string

## [0.45.2] - 2023-07-06

### 🚀 Features

- Add more debug info
- Refresh database flush display progress spinner
- Improve save topics message display

### 🐛 Bug Fixes

- Some deb filename (like apt) local name parse
- Do not write not sync mirror to atm.list
- Oma topics will fallback to repo.aosc.io mirror if apt-gen-list status file is empty
- Oma topics fallback repo.aosc.io use https oops
- Do not display incompatible arch topics
- Add missing fallback in write_enabled function
- Tty envivment do not overflow display

### 🚜 Refactor

- Improve handle url method
- Improve refresh database logic
- Improve mirror_is_ok function message
- Improve update database logic

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code
- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps
- Update all deps
- Update all deps

### I18n

- Add some translate
- Refine new strings
- Add config.rs translate

## [0.45.1] - 2023-07-05

### 🐛 Bug Fixes

- Oma rdepends output i18n issue

## [0.45.0] - 2023-07-05

### 🚀 Features

- Add oma config to config oma network_thread and protect_essentials
- Add oma topics progress spinner

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.44.2] - 2023-07-03

### 🐛 Bug Fixes

- Handle pkexec file path 'no such file or directory'

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.44.1] - 2023-06-25

### 🚀 Features

- Oma list-files -> oma files
- Display earlier/expired signature mirror name
- Improve download database message display
- Improve download database message display (again)

### 🐛 Bug Fixes

- Oma upgrade will auto purge autoremove package

### 🚜 Refactor

- No need to push pkg_score function

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update README

## [0.44.0] - 2023-06-18

### 🚀 Features

- Log oma run result status

### 🚜 Refactor

- Refact install_handle_error and install_handle_error_retry

### 🎨 Styling

- Remove useless line
- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.43.2] - 2023-06-11

### 🚀 Features

- Only action is non empty push to oma history undo list
- Use default clap style

## [0.43.1] - 2023-06-11

### 🐛 Bug Fixes

- Improve error message context in fetch local mirror (file://)

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.43.0] - 2023-06-10

### 🚀 Features

- New line for oma history undo tips
- Improve contents-may-not-be-accurate tips

### 🐛 Bug Fixes

- Do not display downloading package tips if user ctrlc pending ui
- Undo opration tips should display 'redo'
- Use modified() to get update time delta
- Sometimes progressbar stdout eat oma message

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.42.0] - 2023-06-09

### 🚀 Features

- Improve redo logic

### 🎨 Styling

- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.41.1] - 2023-06-08

### 🐛 Bug Fixes

- Add some missing oma bash completions
- Fix some subcommand packages name completion
- Use console::measure_text_width to calc string width to fix sometimes strip prefix will panic
- Add missing fish completions
- Sometimes progress bar println message not print to new line

### 🎨 Styling

- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps
- Remove useless line in Cargo.toml
- Update all deps

## [0.41.0] - 2023-06-06

### 🚀 Features

- Use indicium search engine to oma search package
- Add utils.rs translate template
- Add verify.rs translate template
- Add topics.rs translate template
- Add tpkgrs translate template
- Add pager.rs translate template
- Add oma.rs translate template
- Add main.rs translate template
- Add formatter.rs translate template
- Add download.rs translate template
- Add db.rs translate template
- Add contents.rs translate template
- Add checksum.rs translate template
- Move help message from inquire to topics.rs to translate
- Add scan topic to remove string
- Oma download add --with-deps flag to download package(s) with deps
- Add oma history feature
- Add oma optration done undo tips
- Add missing op done tips
- If action not empty display undo tips

### 🐛 Bug Fixes

- Fix some provide package order
- Remove useless entry in oma.ftl
- Fix do-not-edit-topic-sources-list new line
- Remove useless " in oma.ftl
- Use fluent new line format
- Fluent some need use string

### 🚜 Refactor

- Remove repeated string
- Refactor contents.rs
- Refactor db.rs
- Add InstallOptions::default()

### 🎨 Styling

- Add missing new line symbol in zh-CN/oma.ftl
- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Remove git rebase comment
- If i18n translate updated re run build.rs script
- Add not automatic build i18n method
- Update all deps

### I18n

- Make Omakase speak English
- Reword pid => PID
- Remove 'type to filter item' in topic tips
- Fix typos in en-US
- (WIP) zh-CN localization
- Add colon symbol; adjust some output format
- Fix dep-error-desc desc
- Add 'pick-tips' string
- Add missing i18n string
- Delete repeated full comma
- Fill zh-CN missing translate template
- Fix debug-symbol-available ui string issue
- Fix scan-topic-is-removed name display
- (en-US) tweak wording and punctuation marks
- (zh-CN) finish translation
- Adapt some string to i18n; fix redo install package
- Add all history string to i18n
- Sync en-US translate string to zh-CN
- (en-US) improve UI strings
- (zh-CN) complete localization

## [0.40.0] - 2023-05-25

### 🚀 Features

- Oma contents bin search use strsim to filter result
- Add oma provides/list-files --bin flag to only search bin files
- Add i18n support framework

### 🐛 Bug Fixes

- Fix oma list-files --bin argument name
- Fix oma compile and add i18n example
- (again) try to fix unicode width new line display issue
- Fix only noarch topic enable

### 🚜 Refactor

- No need to use Either
- Box dyn Iterator can auto infer type

### 📚 Documentation

- Add more code comment

### 🎨 Styling

- Run cargo clippy to lint code
- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Remove useless dep
- Update all deps

## [0.39.0] - 2023-05-14

### 🚀 Features

- Ignore case search word and pkg description
- Always lowercase search word
- Oma download success display downloaded package count
- Max terminal len limit to 150
- Oma search if strsim score is 1000 (max) display 'full match'

### 🐛 Bug Fixes

- No need to unlock oma two twice
- Oma list glob support
- Oma list only installed version display installed

### ⚙️ Miscellaneous Tasks

- Use zlib-ng to improve performance
- Update all deps

## [0.38.2] - 2023-05-12

### 🚀 Features

- Try to flushing file add progress spinner
- Try to flushing file add progress spinner again

### 🐛 Bug Fixes

- Use tokio::io::copy replaced tokio::fs::copy

### 🎨 Styling

- Use cargo-fmt to format code

## [0.38.1] - 2023-05-12

### 🚀 Features

- Copy file use fs::copy to improve preforence; use ProgressSpinner to display fetch local source progress

### 🐛 Bug Fixes

- Fetch local source inc global bar
- Half-configure do not mark pkg as reinstall status
- Fix mirror is updated oma refresh will checksum mismatch
- Download global bar reset position if checksum fail and not allow resume

### 🎨 Styling

- Use cargo-fmt to format code
- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.38.0] - 2023-05-11

### 🚀 Features

- Add some update database debug message
- Add fetch local source package progress bar
- Use current thread to fetch local source
- Add more debug output for fetch local source
- Improve oma show APT-Source info output

### 🐛 Bug Fixes

- Fetch local source pkg use oma style progress bar
- Fetch local source do not uncompress in local source (from) directory
- Fix run decompress file
- Oma refresh progress bar inc
- Fetch local source InRelease inc progress
- Use right method to get apt style source

### 🚜 Refactor

- Do not read buf all to memory in download_and_extract_db_local method

### 🎨 Styling

- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.37.1] - 2023-05-11

### 🐛 Bug Fixes

- Fix fetch local source database file
- Check file is exist in download_and_extract_db_local
- Fix fetch local source database filename

### 🎨 Styling

- Use cargo clippy to lint code
- Use cargo fmt to format code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.37.0] - 2023-05-11

### 🚀 Features

- Search order move package to top if pkg.name == input and installed
- Tips user virtual package didn't mark
- Improve download dir not exist error output
- More precise handling of IOError in the try_download function
- Improve try_download error output

### 🐛 Bug Fixes

- Do not display error to due_to in oma topics
- Fix run oma fix-broken
- Fix run oma install --install-dbg
- Cli::writeln do not output next empty line

## [0.36.3] - 2023-05-09

### 🐛 Bug Fixes

- Error and due_to to right order

### 🚜 Refactor

- Use error_due_to function to easily handle the due_to case
- Abtsract error_due_to method

### ⚙️ Miscellaneous Tasks

- Update rust-apt version and adapt it

## [0.36.2] - 2023-05-09

### 🚀 Features

- Try_download return error display due_to

### 🐛 Bug Fixes

- Do not decompress BinContents

### 🎨 Styling

- Use cargo clippy and fmt to lint code

## [0.36.1] - 2023-05-09

### 🐛 Bug Fixes

- Packages argument after add some argument flag to wrong result

## [0.36.0] - 2023-05-09

### 🚀 Features

- Improve oma repends output
- Add more error output in try_download method

### 🐛 Bug Fixes

- Do not download package success download next package
- Do not append decompress file
- This loop never actually loops in try_download method
- Download success break loop in try_download method

### 🚜 Refactor

- Optimize try_download logic
- Use true/false replaced Ok/Err in try_download method

### 🎨 Styling

- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update some deps and adapt new rust-apt version
- Update all deps

## [0.35.0] - 2023-05-06

### 🚀 Features

- Recommend -> recommends, suggest -> suggests in oma install [ARGS]
- Add oma install --no-install-recommends and --no-install-suggests

### 🐛 Bug Fixes

- Fix force-yes, no-install-{recommends,suggests} argument

### 🚜 Refactor

- Set Config struct name as AptConfig

### 🎨 Styling

- Use cargo-fmt to format code

## [0.34.0] - 2023-05-06

### 🚀 Features

- Return 1 if oma show pkgs result is empty
- Add oma pkgnames for shell completion
- Add shell competions feature
- Support fish completion
- Add oma systemd service
- Oma install/remove/upgrade -y should display review message
- Display command not found error if oma command-not-found no results found

### 🐛 Bug Fixes

- Improve UI strings for oma pending ui output
- Apt_lock_inner failed do not retry
- Retry 3 times, not 4
- Retry 3 times, not 4 (again)
- Fetch database global progress bar overflow
- Fix wrong oma pkgnames parameter name ...

### 🚜 Refactor

- Oma args function return exit code
- Optimize main logic
- No need to collect package list in oma list method

### 🎨 Styling

- Use cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.33.1] - 2023-05-04

### 🚀 Features

- Add Shell integrations

### 🐛 Bug Fixes

- Improve UI strings for oma refresh output
- Oma command-not-fould should return 127
- Oma command-not-found always return 127
- Push missing fish command-not-found commit
- Improve command-not-found directions

### 📚 Documentation

- Improve oma install --install-recommend and --install-suggest help message

### Meta

- Move PolicyKit rules to /data/policykit
- License code under GPLv3

## [0.33.0] - 2023-05-04

### 🚀 Features

- Handle if pkg current_state == 4 or 2 (half-install or half-configure)
- Add more debug message for needs_fix_system method
- Oma install add --install-recommend and --install-suggest argument
- Add more debug for download method
- Add query upgadable packages progress spinner
- Add terminal bell if oma operation is done

### 🐛 Bug Fixes

- Both contents-all and contents-ARCH must be downloaded
- Do not panic with display CJK message

### 🚜 Refactor

- Abstract install_other logic
- No need to collect upgrade package in update_inner method
- Optmize search_pkgs filter logic
- Optmize search_pkgs filter logic again
- Optmize search_pkgs filter logic
- No need to collect checksum entry to parse
- No need to download multi contents
- Use BinContents to command-not-found feature
- Use BufReader to decompress package database
- Use Box to optimize logic in decompress method
- No need to collect in search_pkgs method

### 🎨 Styling

- Use cargo-fmt and cargo-clippy to lint code
- Use cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps
- Update all deps

## [0.32.2] - 2023-05-02

### 🐛 Bug Fixes

- Fetch inrelease return checksum mismatch error if mirror inrelease is updated
- Truncate file and set file length == 0 if file_size >= download total_size

## [0.32.1] - 2023-05-02

### 🚀 Features

- Open new thread to check contents file metadata
- Return 0 if operation allow ctrlc

### 🐛 Bug Fixes

- Oma mark needs root

### 🚜 Refactor

- Optmize download db logic again
- Optmize local mirror download and extract logic

### ⚙️ Miscellaneous Tasks

- Update anstream to 0.3.2

## [0.32.0] - 2023-05-01

### 🚀 Features

- Adjust terminal width < 90 progress bar style

## [0.31.1] - 2023-05-01

### 🚀 Features

- Display searching contents message if match is empty
- Check contents create time to tell user contents file may not be accurate

### 🐛 Bug Fixes

- Do not panic with display CJK message

### 🚜 Refactor

- Download progress spinner no need to use new thread wait request send

## [0.31.0] - 2023-04-30

### 🚀 Features

- Do not inc global bar if file exist and running checksum
- Improve ui string
- Display resume info

### 🚜 Refactor

- Improve get file_size logic
- Use validator to verify integrity while downloading
- Improve download methold open file times
- Re use validator to improve checksum

### 📚 Documentation

- Add some comment in download method

### 🎨 Styling

- Inline function in download method
- Use cargo-clippy to lint code

## [0.30.3] - 2023-04-30

### 🚜 Refactor

- Improve resume download logic

### 🎨 Styling

- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Remove useless test
- Update all deps

## [0.30.2] - 2023-04-29

### 🐛 Bug Fixes

- Download again when checksum does not match and returns 416
- Revert retry 2 times start dpkg-force-all mode

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.30.1] - 2023-04-29

### 🐛 Bug Fixes

- Add missing ! to fix wrong logic in scan_closed_topic
- Reson => Reason

### 🚜 Refactor

- Improve auto close topic

### ⚙️ Miscellaneous Tasks

- Remove uselses test

## [0.30.0] - 2023-04-27

### 🚀 Features

- Add topics feature
- Update_db if url is closed topic, remove url from apt sources.list
- Drop inquire searcher
- Drop inquire searcher curosr

### 🐛 Bug Fixes

- If package newest version in other enabled topics, downgrade to stable version
- Don't let the progress spinner thread dead loop if the download has errors
- Do not save file with download failed; return error if 404 not found url is not closed topic

### 🚜 Refactor

- Use spawn_blocking to execute rm_topic method

### 🎨 Styling

- Use cargo clippy to lint code
- Use cargo clippy to lint code again
- Use cargo-fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.29.1] - 2023-04-23

### 🚀 Features

- Set clap help header and usage color as bright blue
- Improve clap oma style theme ...
- Check InRelaese date and vaild-until

### 🐛 Bug Fixes

- Not allow_resume file wrong reset length
- Download doesn exist file will return error

### 🚜 Refactor

- Improve download method logic

### 🎨 Styling

- Remove useless refrence flag
- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.29.0] - 2023-04-19

### 🚀 Features

- Sort oma search order, UPGRADE > AVAIL > INSTALLED

### 🚜 Refactor

- Use trait to get prefix string

## [0.28.2] - 2023-04-19

### 🚀 Features

- Command-not-found do not display progress spinner

### 🎨 Styling

- Lint code use myself brain and cargo-clippy

## [0.28.1] - 2023-04-19

### 🐛 Bug Fixes

- Fix-broken no need to do anything useless to run apt_install method

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.28.0] - 2023-04-18

### 🚀 Features

- Oma download do not display downloaded specific pkgs
- Check system needs fix status
- Check system needs fix status in oma {upgrade,fix-brokeen}

### 🐛 Bug Fixes

- Oma download path maybe return error

### 📚 Documentation

- Add current_state comment
- Afixcurrent_state comment a typo

### ⚙️ Miscellaneous Tasks

- Update all deps
- Update h2 to v0.3.18

## [0.27.0] - 2023-04-17

### 🚀 Features

- Fetch un-compress database file in mips64r6el arch
- Allow resume exist download package progress

### 🐛 Bug Fixes

- Download failed reset wrong progress bar status

### 🎨 Styling

- Use cargo-clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.26.0] - 2023-04-13

### 🚀 Features

- Add upgradable check unmet dependency

### 🐛 Bug Fixes

- If can not get ARCH, error missing context
- If get ARCH run dpkg to failed, error missing context
- If get ARCH run dpkg to failed, error missing context (2)

### 🚜 Refactor

- Use dpkg --print-architecture to get arch name

### 🎨 Styling

- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.25.0] - 2023-04-11

### 🚀 Features

- Support oma -v to display oma version
- Support mips64r6el arch

### 🐛 Bug Fixes

- Missing --version (-v, -V) help message
- Repeated version flag to run build.rs script to failed

### ⚙️ Miscellaneous Tasks

- Update all deps

### Tree-wide

- Capitalise first letter of project description

## [0.24.3] - 2023-04-09

### 🐛 Bug Fixes

- Can not set logger with --debug flag

## [0.24.2] - 2023-04-09

### 🚀 Features

- Improve command-not-found output

### 🐛 Bug Fixes

- Provides search absolute path can't get any result
- Pick can not get no_upgrade argument to panic

### 🎨 Styling

- Use cargo clippy to lint code
- Use cargo-fmt to format code

## [0.24.1] - 2023-04-09

### 🐛 Bug Fixes

- Oma dep output wrong grammar
- Reinstall does not in repo version to panic
- Pick no_fix_broekn wrong argument name to panic
- No additional version info tips

### 🚜 Refactor

- Improve list method code style

### 📚 Documentation

- Update README

### 🎨 Styling

- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update dep crossbeam-channel to 0.5.8

## [0.24.0] - 2023-04-08

### 🐛 Bug Fixes

- Use PossibleValues to fix oma-mark man document
- Fix without dry-run argument subcommand run
- Fix oma show needs packages argument
- Set search arg name as pattern
- Fix wrong oma list info display

### 🚜 Refactor

- Improve setup dry_tun flag logic

### 📚 Documentation

- Improve help and man document

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.23.0] - 2023-04-06

### 🚀 Features

- --debug argument now can run without dry-run mode
- Add cache.get_archives spinner
- Add --no-autoremove argument for oma {install,upgrade,remove,pick}
- Add query packages database spinner
- Oma install do not autoremove by default
- Oma pick do not autoremove by default

### 🐛 Bug Fixes

- Fix global bar progress percent color
- Fix refresh database file exist global bar progress
- Fix oma pick no_autoremove arg requires
- Fix query database zombie progress bar

### 🚜 Refactor

- Set Multiprogress Bar as lazy var
- Improve pending ui detail capitalize logic

### 🎨 Styling

- Run cargo clippy to lint code
- Use cargo-clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update serde-yaml to 0.9.20

## [0.22.0] - 2023-04-05

### 🚀 Features

- Build all subcommand man
- If needs run dpkg --configure -a, run it
- Error output message adjust

### 🐛 Bug Fixes

- Fix autoremove/non-autoremove pkg pending ui wrong detail

### 🚜 Refactor

- Improve capitalize output message logic in apt_handler mehod

### 🎨 Styling

- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.21.0] - 2023-04-03

### 🚀 Features

- If retry 2 times apt has error, go to dpkg-force-all mode
- If update dpkg-force-all mode after has broken count, return error

### 🐛 Bug Fixes

- Fix a typo

### 🎨 Styling

- Use cargo fmt and cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.20.0] - 2023-04-02

### 🚀 Features

- Improve error message display
- Improve progress bar style
- Improve progress bar style again

### 🐛 Bug Fixes

- Fix oma subcommand history run
- Fix /run/lock directory does not exist

## [0.19.0] - 2023-04-01

### 🚀 Features

- Add {upgrade,install,fix-broken} subcommand --dpkg-force-all argument

### 🐛 Bug Fixes

- Add missing progress bar logic

### 🎨 Styling

- Use cargo-fmt to format code

### ⚙️ Miscellaneous Tasks

- Update rustix dep

## [0.18.1] - 2023-04-01

### 🐛 Bug Fixes

- Pending ui message too loong to panic
- Do not display download progress in retry
- Fix yes argument download

### 🚜 Refactor

- Optmize download before check file is exist logic

## [0.18.0] - 2023-03-31

### 🚀 Features

- Improve command short help

### 🐛 Bug Fixes

- Fix package name ends_with deb install
- Add missing subcommand ...
- Add missing oma mark help message

### 🎨 Styling

- Use cargo clippy to lint code

### ⚙️ Miscellaneous Tasks

- Add man to .gitignore
- Remove useless file
- Update all deps

## [0.17.1] - 2023-03-31

### 🚀 Features

- Add extract and verify database progress bar
- Try use clap to gen man
- Output man pages to /man

### 🚜 Refactor

- Use clap build api to build argument

### 🎨 Styling

- Move single_handler code line location
- Run cargo clippy

### ⚙️ Miscellaneous Tasks

- Remove useless tracing-subscriber envfilter dep
- Update README
- Clap_cli.rs => args.rs
- Update all deps

## [0.17.0] - 2023-03-28

### 🚀 Features

- Add policykit support
- Add .policy file to add policykit oma infomation
- If fetch last url has error, output error prefix

### 🐛 Bug Fixes

- Fix exit code with policykit run
- Fix download database global bar display in file:// prefix local mirror
- Try to fix download progress bar count
- Fix warning message before global bar draw display

### 🚜 Refactor

- Do not always in running in async runtime
- Refactor some code style
- Refactor content::file_handle method; rename to remove_prefix
- Decompress database do not block tokio runner

### 🎨 Styling

- OmaAction => Oma
- Run cargo fmt and clippy

### ⚙️ Miscellaneous Tasks

- Add dependencies comment in Cargo.toml
- Update rust-apt to newest git snapshot
- Update all deps

### Io.aosc.oma.apply.policy

- Improve UI strings
- Default to /bin/oma

## [0.16.0] - 2023-03-27

### 🚀 Features

- Support provides package install
- Oma download add argument --path (-p)
- Oma download success will display download packages path
- Read Dir::Cache::Archives value to get apt set download dir config
- Improve oma search result sort
- Fix reset is_provide status in search_pkgs method
- Find all provides in search.rs method to improve search result
- Log user actions in a human-readable fashion
- Improve oma log Resolver key style
- Log file remove 'Resolver' word
- Add oma --debug argument to see dry-run debug message
- Do not display JSON-like args info in dry-run mode
- Oma dep/rdep improve grammar display
- Oma dep/rdep PreDepended by => Pre-Depended by

### 🐛 Bug Fixes

- Local mirror progress display
- Oma download do not download non candidate pkg
- Only virtual pkg get provides to get real pkg
- Fix archive dir read logic
- Add Dir::Cache::Archives fallback logic
- Fix local package install
- Fix oma start-date/end-date localtime offset

### 🚜 Refactor

- Improve get local pkgs
- Improve query pkg method

### 🎨 Styling

- Run cargo clippy'
- Drop useless line
- Use cargo clippy and cargo fmt
- Lint code use cargo clippy
- Run cargo clippy and cargo fmt to lint code

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.15.0] - 2023-03-24

### 🚀 Features

- Dry-run mode display oma and OS info
- Improve log user-family output
- Set oma and os dry-run info as debug
- Dry-run read RUST_LOG env to see debug message (default RUST_LOG is info)

### 🐛 Bug Fixes

- Do not real run {mark,clean,download} in dry-run mode
- Fix dry-run in fix-broken subcommand argument
- Fix dry-run default display log

### 🎨 Styling

- Improve pick method code style
- Use cargo fmt to lint code style

### ⚙️ Miscellaneous Tasks

- Update all deps

## [0.14.0] - 2023-03-23

### 🚀 Features

- If pkg is essential, oma will reject it mark to delete
- Add oma --dry-run argument
- Dry-run mode add args tracing

### 🚜 Refactor

- Use fs::read to replace fs::File::open and read_buf
- Improve DOWNLOAD_DIR var use

## [0.13.2] - 2023-03-22

### Cargo.toml

- Bump version to 0.13.2 for adapt cargo-smart-release

### README

- Update

### Action

- If find_unmet_deps can't find any dependencies problem, return apt's error

### Changelog

- New

### Formatter

- Add find_unmet_deps_with_markinstall method to handle if mark_install can't success
- Find_unmet_deps_with_markinstall method do not display 'User aborted the operation' info
- Find_unmet_deps_with_markinstall if apt cache could not find pkg, add to UnmetTable list

### Lint

- Use cargo fmt
- Cargo fmt

### Main

- Add fake clap output for wrong --ailurus argument count
- Fake clap more like real clap
- Fake clap more like real clap

## [0.13.1] - 2023-03-21

### Release

- 0.13.0
- 0.13.1

### Action

- Add unmet dependency error output
- Do not display user abort op in find_unmet dep method
- Improve find unmet dep logic

### Contents

- Progress spinner use oma style

### Formatter

- Add Break and Conflict to unmet dependencies table
- Add PreDepends to unmet dependencies table
- Improve unmet pending ui style
- Unmet ui do not right align
- Improve pending ui style

### Lint

- Use Lazy<Writer> replaced OnceCell<Writer>
- Adjust code stract
- Use OnceCell::Lazy<PathBuf> to replace Path String
- Use cargo clippy

### Pkg

- Move mark_install method to pkg.rs

## [0.1.0-alpha.12] - 2023-03-19

### README

- Update TODO

### Release

- 0.1.0-alpha.12

### Action

- Improve pending ui style ...
- Fix pending ui upgrade/install style
- Adjust upgrade table color again
- Improve remove table ui statement
- Rewrite log write
- Fix install loop
- Log format adjust
- Adjust log format

### Deps

- Use once_cell replaced lazy_static

### Download

- Improve download InRelease ProgressBar
- Use bouncingBall spinner style
- Code clean up
- Improve 'download' method logic ...
- Remove redundant reqwest error handle

### Lint

- Use cargo clippy

### Main

- Add some ailurus
- Add oma log feature ...
- Set log filename as history

## [0.1.0-alpha.11] - 2023-03-16

### Verify

- Fix multi key in one cert file  error handle
- Fix multi key in one cert file error handle (2)

## [0.1.0-alpha.10] - 2023-03-16

### ⚙️ Miscellaneous Tasks

- Try to fix ci

### README

- Update TODO
- Update TODO and usage
- Fix a typo

### Action

- Add 'oma clean' subcommand
- Improve install version select
- Improve pick select version
- Pick display branch if version is equal
- Fix pick panic
- Fix pick display wrong branch
- Improve pick version display
- Fix oma pick select pos
- Add oma list argument --upgradable (-u)
- Add 'yes' option
- Add yes warn
- Fix install need root tips
- Add 'force_yes' argument to apt_handler method

### Bin

- Move main.rs to bin/oma.rs

### Bin/oma

- Allow yes option
- Add yes warn

### Cli

- Fix dead forloop

### Db

- Fix local source twice fetch
- Optimize update_db logic
- Add apt sources.list signed-by support

### Deps

- Update
- Use own debcontrol-rs fork to fix rustc warning

### Download

- Optimize down_package method logic
- Set global pb steady_tick as 100ms
- Fix global pb style

### Formatter

- Try to fix apt automatic install
- Try to fix apt automatic install (2)
- {Yes,Apt}InstallProgress -> OmaAptInstallProgress ...
- If no --yes do not set --force-confold

### Lint

- Use cargo clippy
- Use cargo clippy
- Use cargo clippy
- Make cargo clippy happy
- Use cargo clippy

### Main

- Add clean subcommand description
- Try to fix random segfault
- Oma-yes =? oma --yes/-y
- Add args comment

### Pkg

- Fix the version selection problem of the same version but different sources

### Verify

- Support .asc file verify
- Add missing key dueto
- Fix multi key in one cert file parser

## [0.1.0-alpha.9] - 2023-03-11

### READNE

- Update usage and fix  typo

### Action

- Fix list installed display
- Improve fix-broken feature
- Oma install default fix_broken pkg
- Add oma install --no-fix-broken and --no-upgrade argument
- Add some comment; improve display_result logic
- Add tips to oma install doesn't not exist pkg
- Oma list/show/search if results is empty, return error
- Fix mark hold/unhold pkgs can't unlock dpkg
- Try to fix install count == 0 but configure count != 0, oma exit

### Db

- Fix a typo

### Deps

- Use fs4 replaced fs2 crate

### Download

- Move packages_download function from db.rs

### Lint

- Comment unuse code

### Release

- 0.1.0-alpha.9

### Util

- Size_checker display human bytes

## [0.1.0-alpha.8] - 2023-03-08

### Action

- Fix install wrong pkg version

### Lint

- Use cargo fmt and clippy

### Pager

- Different pages display different tips

### Release

- 0.1.0-alpha.8

### Search

- If height > 1 page, use less to display
- If input equal provide name, sort to top
- Improve cmp logic

## [0.1.0-alpha.7] - 2023-03-05

### Action

- Abstract some step to mark_install function
- Output package file path when local installation returns an error
- Fix install local pkg version select
- List add automatic status display
- List add display additional version info output
- Fix another_version info display
- Fix another_version info display again
- Add oma show -a argument
- Show add display additional version info output
- Check root after lock oma to fix need root tips
- If oma remove package does not exist display info
- Subcommand 'mark' adjust

### Conntents

- Adjust pb steady_tick and if rg return non-zero code return error

### Contents

- Add progress spinner output

### Db

- Fix local source metadata fetch
- Fetch done display info
- Improve local deb install logic

### Lint

- Cargo clippy
- Improve logic
- Use cargo clippy, fmt

### Main

- Oma remove add 'purge' alias

### Release

- 0.1.0-alpha.7

## [0.1.0-alpha.6] - 2023-03-03

### Action

- Fix marked upgrade/downgrade check
- Fix download need sudo
- Remove debug output

### Release

- 0.1.0-alpha.6

## [0.1.0-alpha.5] - 2023-03-02

### ⚙️ Miscellaneous Tasks

- Add rust templete
- Try fix ci

### Cargo.toml

- Bump version to 0.1.0-alpha.4

### README

- Update dep
- Add TODO
- Fix a typo

### Action

- Add 'oma refresh' tips to tell user can upgradable and auto removable package
- Fix handle if package depends does not exist
- Support reinstall local package
- Improve local package reinstall logic

### Lint

- Use cargo clippy
- Use cargo clippy

### Release

- 0.1.0-alpha.5

## [0.1.0-alpha.3] - 2023-02-25

### Action

- New, this is User Action Control
- Add remove feature
- Add refresh to only update package database

### Cargo.toml

- Set name as oma (Oh My Ailurus)
- Bump version to 0.1.0-alpha.1
- Buml ver to 0.1.0-alpha.2
- Bump version to 0.1.0.alpha.3

### README

- Rename to oma, fix grammar
- Update
- Add a definition for Omakase
- Add Japanese spelling for Omakase
- Add dependencies

### Action

- Use rust-apt to calculate dep
- Update and install done
- Fix comment typo
- Support apt install fish/stable
- Abstract apt install step to apt_install function
- Improve install/update feature
- Fix autoremove
- Improve retry mechanism
- Add more info output
- Add result display ...
- Fix packages_download
- Pager add download size and disk_size display
- Use libapt-pkg to check download version
- Fix display select package version ...
- Support like oma upgrade meowdict ...
- Protect mark_install with oma install PACKAGE/BRANCH
- Fix downgrade color
- Fix like oma upgrade fish=3.5.1-1
- Oma install support glob ...
- Fix remove result wrong issue ...
- Improve display result
- Use rust-apt to read all new and old pkg database
- Use more rust-apt library
- Improve display package version logic
- Add install .deb from local
- Code all clean up
- Improve install tips ...
- Use info to tell user what package is installed
- Fix local install .deb package
- Install_handle add comment
- If local install error display file path
- Add search feature
- Fix install with branch and version
- Fix local install again ...
- Add 'download' command
- Download command only download package ifself
- Move root check to need root function
- Add fix-broken command
- Fix-broken command add pending operations page
- Fix-broken add packages_download step
- Lock ctrlc in dpkg install
- Remove dpkg ctrlc handler ...
- Abstract is_root function
- Use search::show_pkgs to get pkg info, improve code style
- Add oma install --dbg(--install-dbg) argument
- Add 'pick' subcommand
- Move cache.resolve(true) to apt_install function inner
- Move cache.resolve(true) to apt_handle function inner
- Remove useless line
- Fix autoremove step
- Add 'mark' command
- Rm useless line
- Fix install size calculate display
- Size byte display B -> iB
- Fix a typo
- Use thiserror to control retry
- Use anyhow to handle non-apt errors in cache.upgrade
- Add 'command-not-found' subcommand
- Fix list display
- Fix list preformance and style
- Sort output order and add --installed argument
- Fix next line output logic
- List display package arch
- Fix list installed display logic
- List function improve code style

### Blackbox

- Add apt_calc function
- Add AptAction::Purge
- Add apt -s info parse
- Add debug info

### Blackbox/dpkg_executer

- All done
- Fill of remove and purge feature

### Blackbox/dpkg_run

- Improve abstraction

### Checksum

- Fix checksum eat memory issue ...

### Cli

- Use stderr to output info/warn/debug/dueto ...

### Contents

- Done, but so slow
- Improve contents logic
- Improve output result
- Improve code style
- Improve code style again
- Use ripgrep cli to read contents ...
- Fix rg error return
- Improve error return
- Improve error return again
- Fix regex security issue
- Use regex::escape to replace my escape step
- If local contents file is empty, run update db
- If user run oma provides/list-files, force run update_db
- Revert update_db feature ...
- Fix list-files package display
- Improve output result
- Add error output if contents not exist
- Fix contents line is pkg_group logic
- Remove useless char
- Adapt command-not-found subcommand
- Fix area/section/package line

### Db

- Fix package download with version
- Remove useless function
- Handle file:// or cdrom:// mirror
- Add a comment
- Multi thread download InRelease files
- Support flat and local mirror
- Fix update_db checksum logic
- Fix flat repo refresh logic
- Remove useless dbg
- Fix non-flat local mirror refresh logic
- Improve flat/non-flat mirror refresh logic
- Improve flat/non-flat mirror refresh logic again

### Deps

- Use debcontrol to replace 8dparser
- Update rust-apt to new git commit
- Update and set rust-apt to crate version
- No need to use indexmap
- Rust-apt use my fork to fix search/show panic ...
- Use rust-apt https://gitlab.com/volian/rust-apt/ newest git

### Download

- Fix filename to compatible apt download
- Fix a bug, if arch = xxxnoarch
- Learn omakase
- Fix if /var/lib/apt doesn't not exist
- Improve download code logic
- Use async to partial download
- Fix progress bar file size
- Add error check
- Use MultiProgress to display download progress
- Add download thread limit
- Fix download filename
- Handle pb message if text width > 48
- Progressbar add count and len
- Add global progress bar to global download packages progress
- Improve global progress bar
- Fix global bar number color
- Fix color in non-global bar
- Improve download message
- Improve download logic ...
- Clean again
- Code clean up again
- Abstract some step to try_download
- Fix libapt get url
- Fix oma_style_pb in terminal length < 100 run

### Download_db

- Add fetch database multiprogress
- Improve pb style
- Pb display branch

### Formatter

- Add more ouput

### Lint

- Run cargo clippy
- Use cargo clippy
- Use cargo clippy
- Fix cargo clippy
- Use cargo clippy
- Use cargo clippy
- Cargo fmt
- Use cargo clippy
- Use cargo clippy
- Use cargo clippy
- Use cargo clippy

### Main

- Use clap to handle subcommand
- Add exit code
- Set update feature subcommand name as upgrade ...
- Improve error handle
- If error exit code 1
- Add oma info and root check ...
- Add oma show command
- Remove useless display version info
- Move update alias to refresh command
- Rename search-file command to provides
- Improve lock/unkock logic from szclsya/sasm
- Unlock_oma with has error
- Move unlock step from try_main to main

### Main,action

- Rename list-file to list-files

### Pager

- Handle ctrlc exit status

### Search

- Improve search ui style
- Fix a typo
- Fix upgradable output
- Improve upgradable ui style
- Set section bases package as blue color
- Fix APT-Source field display ...

### Update

- Use vector to put ChecksumItem
- Fill of download package list and contents
- All done
- Handle if apt Installed-Size and dpkg mismatch
- UpdatePackage add filename and from field; fix var name mistake
- UpdatePackage add some size field
- Dpkg_executer: retry 3 times to workround dpkg trigger cycle

### Util

- Check available space before download and installation

### Utils

- Fix size_checker in chroot



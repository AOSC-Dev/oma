# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2023-08-18)

<csr-id-9b58969c8836740f4d205fba10f4857b70674070/>
<csr-id-bbade3d123272c927ece6a8c0d7ef0a5d2f20ee9/>
<csr-id-a9dbffa13072234f00b3058d68e2c61ff48a5cb5/>
<csr-id-e408f1d2e34e132b74a3b91b09d904f536a4e184/>
<csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/>
<csr-id-42a30f3c99799b933d4ae663c543376d9644c634/>
<csr-id-c3ce6357561acc73e2cea20766230d27e860d96a/>
<csr-id-53d477570f1519ccbd964ad6560a74b15acd7df0/>
<csr-id-00e6c62d5b9a765dc8b11472614ca714c965a729/>
<csr-id-a1695307a23587ced897257d400de39b645805e5/>
<csr-id-6f65b3656809f431f3da938e7a9eac10b9922d60/>
<csr-id-217a8b9e973971581591ecc5c95f0960ca1eba8a/>
<csr-id-5356ded0e3fb12b175262319f9b29b5c64ec74c0/>
<csr-id-e9063ab2283dd2d3e9c2f24db60bfe2561448de1/>

### Chore

 - <csr-id-9b58969c8836740f4d205fba10f4857b70674070/> fill in comment, desc and license
 - <csr-id-bbade3d123272c927ece6a8c0d7ef0a5d2f20ee9/> use oma-apt-sources-list crate (own fork)
 - <csr-id-a9dbffa13072234f00b3058d68e2c61ff48a5cb5/> inquire -> oma-inquire
 - <csr-id-e408f1d2e34e132b74a3b91b09d904f536a4e184/> drop useless dep
 - <csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/> no need to use tracing

### Chore

 - <csr-id-e3101b38d83114b13e89024a0fa21246eca764e5/> add changelog

### New Features

 - <csr-id-888b7dc90264c1dcce301c2e4350442d8a137478/> add download local source feature

### Bug Fixes

 - <csr-id-948b6d93cd92ea9b52b0bb00f302ce037c6bc4ae/> clear decompress progress bar

### Other

 - <csr-id-42a30f3c99799b933d4ae663c543376d9644c634/> fmt

### Refactor

 - <csr-id-c3ce6357561acc73e2cea20766230d27e860d96a/> inner reqwest::Client
 - <csr-id-53d477570f1519ccbd964ad6560a74b15acd7df0/> oma topics is back
 - <csr-id-00e6c62d5b9a765dc8b11472614ca714c965a729/> fill of error output (100%)
 - <csr-id-a1695307a23587ced897257d400de39b645805e5/> use async
 - <csr-id-6f65b3656809f431f3da938e7a9eac10b9922d60/> improve debug marco
 - <csr-id-217a8b9e973971581591ecc5c95f0960ca1eba8a/> add todo
 - <csr-id-5356ded0e3fb12b175262319f9b29b5c64ec74c0/> do not const Writer::default as WRITER
 - <csr-id-e9063ab2283dd2d3e9c2f24db60bfe2561448de1/> add oma-topics crate

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add changelog ([`e3101b3`](https://github.com/AOSC-Dev/oma/commit/e3101b38d83114b13e89024a0fa21246eca764e5))
    - Fmt ([`42a30f3`](https://github.com/AOSC-Dev/oma/commit/42a30f3c99799b933d4ae663c543376d9644c634))
    - Fill in comment, desc and license ([`9b58969`](https://github.com/AOSC-Dev/oma/commit/9b58969c8836740f4d205fba10f4857b70674070))
    - Use oma-apt-sources-list crate (own fork) ([`bbade3d`](https://github.com/AOSC-Dev/oma/commit/bbade3d123272c927ece6a8c0d7ef0a5d2f20ee9))
    - Inner reqwest::Client ([`c3ce635`](https://github.com/AOSC-Dev/oma/commit/c3ce6357561acc73e2cea20766230d27e860d96a))
    - Oma topics is back ([`53d4775`](https://github.com/AOSC-Dev/oma/commit/53d477570f1519ccbd964ad6560a74b15acd7df0))
    - Inquire -> oma-inquire ([`a9dbffa`](https://github.com/AOSC-Dev/oma/commit/a9dbffa13072234f00b3058d68e2c61ff48a5cb5))
    - Drop useless dep ([`e408f1d`](https://github.com/AOSC-Dev/oma/commit/e408f1d2e34e132b74a3b91b09d904f536a4e184))
    - Fill of error output (100%) ([`00e6c62`](https://github.com/AOSC-Dev/oma/commit/00e6c62d5b9a765dc8b11472614ca714c965a729))
    - Use async ([`a169530`](https://github.com/AOSC-Dev/oma/commit/a1695307a23587ced897257d400de39b645805e5))
    - Fix cargo clippy ([`687af7c`](https://github.com/AOSC-Dev/oma/commit/687af7c78c4ec7f7454ef5dafc300568b0bee354))
    - No need to use tracing ([`fa15124`](https://github.com/AOSC-Dev/oma/commit/fa15124038b9eaf8234766b33a98297c62d5b001))
    - Improve debug marco ([`6f65b36`](https://github.com/AOSC-Dev/oma/commit/6f65b3656809f431f3da938e7a9eac10b9922d60))
    - Clear decompress progress bar ([`948b6d9`](https://github.com/AOSC-Dev/oma/commit/948b6d93cd92ea9b52b0bb00f302ce037c6bc4ae))
    - Add download local source feature ([`888b7dc`](https://github.com/AOSC-Dev/oma/commit/888b7dc90264c1dcce301c2e4350442d8a137478))
    - Add todo ([`217a8b9`](https://github.com/AOSC-Dev/oma/commit/217a8b9e973971581591ecc5c95f0960ca1eba8a))
    - Do not const Writer::default as WRITER ([`5356ded`](https://github.com/AOSC-Dev/oma/commit/5356ded0e3fb12b175262319f9b29b5c64ec74c0))
    - Add oma-topics crate ([`e9063ab`](https://github.com/AOSC-Dev/oma/commit/e9063ab2283dd2d3e9c2f24db60bfe2561448de1))
</details>


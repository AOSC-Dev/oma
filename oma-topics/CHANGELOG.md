# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.2 (2023-09-06)

### Chore

 - <csr-id-bdaa6b35f91364b02c6e7d125ca2edbe1b9a427f/> Bump to 0.1.2
 - <csr-id-57003169329e01d60172d3531e7f3817bacf46da/> Adapt tokio enabled feature
 - <csr-id-4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac/> Adjust some deps (again)
 - <csr-id-ae9e8606bb35ace1db58d2ce88dff0545892c9c8/> Add changelog
 - <csr-id-4731867afda8d6eb3397d383b8a26bf04c4f8364/> Fill in comment, desc and license
 - <csr-id-831f0c306f3f362af003da83a8ccc4351031d1df/> Use oma-apt-sources-list crate (own fork)
 - <csr-id-57fcaa531bc827a8661cf2a4f0f8a50c39289277/> Inquire -> oma-inquire
 - <csr-id-0e14c25a9f5ad34da79df93cd3e686e81323f320/> Drop useless dep
 - <csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/> No need to use tracing

### New Features

 - <csr-id-bff724f28c2943ef490c4c7b0a2b15384cb95550/> Add download local source feature

### Bug Fixes

 - <csr-id-2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5/> Clear decompress progress bar

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt

### Refactor

 - <csr-id-1a3d60e8665faf452a217a478bf0b1c7ce3e445b/> Inner reqwest::Client
 - <csr-id-db0240919451651f494b5bb6a828240c6310b9c7/> Oma topics is back
 - <csr-id-2e01d440b78c79d07b1f04ef67866865934c4049/> Fill of error output (100%)
 - <csr-id-5622e3699691081f0de4466379c14bc539e69c11/> Use async
 - <csr-id-b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef/> Improve debug marco
 - <csr-id-87ebc18929eb44c4b96f0cc9a30822a5277ff440/> Add todo
 - <csr-id-f310ff2486eaba37b8e659991429a81dfea4dff7/> Do not const Writer::default as WRITER
 - <csr-id-df5692d9cd2dea3e882205dcce6d0558b539e279/> Add oma-topics crate

### Style

 - <csr-id-95dd8757eca13ade18af1fea8435336a956a7406/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release over the course of 14 calendar days.
 - 21 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump to 0.1.2 ([`bdaa6b3`](https://github.com/AOSC-Dev/oma/commit/bdaa6b35f91364b02c6e7d125ca2edbe1b9a427f))
    - Release oma-console v0.1.2, oma-topics v0.1.1, oma-refresh v0.3.0 ([`5f4e6d8`](https://github.com/AOSC-Dev/oma/commit/5f4e6d8262f42724c8f796fc0b6c560a39d3fd5f))
    - Adapt tokio enabled feature ([`5700316`](https://github.com/AOSC-Dev/oma/commit/57003169329e01d60172d3531e7f3817bacf46da))
    - Adjust some deps (again) ([`4d25b67`](https://github.com/AOSC-Dev/oma/commit/4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac))
    - Bump oma-console v0.1.1, oma-fetch v0.1.2, oma-utils v0.1.4, oma-pm v0.2.1 ([`64f5d1b`](https://github.com/AOSC-Dev/oma/commit/64f5d1bf4f93b7b3b1f5a00134e232409458e5e3))
    - Run cargo clippy and cargo fmt to lint code ([`95dd875`](https://github.com/AOSC-Dev/oma/commit/95dd8757eca13ade18af1fea8435336a956a7406))
    - Release oma-topics v0.1.0 ([`1bfcdc7`](https://github.com/AOSC-Dev/oma/commit/1bfcdc72769618d853146538a6791956656c191a))
    - Add changelog ([`ae9e860`](https://github.com/AOSC-Dev/oma/commit/ae9e8606bb35ace1db58d2ce88dff0545892c9c8))
    - Fmt ([`9bb6e19`](https://github.com/AOSC-Dev/oma/commit/9bb6e19a703bc76515a7fa70c19aaafef38c7d7b))
    - Fill in comment, desc and license ([`4731867`](https://github.com/AOSC-Dev/oma/commit/4731867afda8d6eb3397d383b8a26bf04c4f8364))
    - Use oma-apt-sources-list crate (own fork) ([`831f0c3`](https://github.com/AOSC-Dev/oma/commit/831f0c306f3f362af003da83a8ccc4351031d1df))
    - Inner reqwest::Client ([`1a3d60e`](https://github.com/AOSC-Dev/oma/commit/1a3d60e8665faf452a217a478bf0b1c7ce3e445b))
    - Oma topics is back ([`db02409`](https://github.com/AOSC-Dev/oma/commit/db0240919451651f494b5bb6a828240c6310b9c7))
    - Inquire -> oma-inquire ([`57fcaa5`](https://github.com/AOSC-Dev/oma/commit/57fcaa531bc827a8661cf2a4f0f8a50c39289277))
    - Drop useless dep ([`0e14c25`](https://github.com/AOSC-Dev/oma/commit/0e14c25a9f5ad34da79df93cd3e686e81323f320))
    - Fill of error output (100%) ([`2e01d44`](https://github.com/AOSC-Dev/oma/commit/2e01d440b78c79d07b1f04ef67866865934c4049))
    - Use async ([`5622e36`](https://github.com/AOSC-Dev/oma/commit/5622e3699691081f0de4466379c14bc539e69c11))
    - Fix cargo clippy ([`6757986`](https://github.com/AOSC-Dev/oma/commit/6757986e906cafe053bffd13dd6768931beb87ea))
    - No need to use tracing ([`0ca5be7`](https://github.com/AOSC-Dev/oma/commit/0ca5be73a7ddb70e3a07b63ef21f2f873e420832))
    - Improve debug marco ([`b8fc1a9`](https://github.com/AOSC-Dev/oma/commit/b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef))
    - Clear decompress progress bar ([`2f40bc8`](https://github.com/AOSC-Dev/oma/commit/2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5))
    - Add download local source feature ([`bff724f`](https://github.com/AOSC-Dev/oma/commit/bff724f28c2943ef490c4c7b0a2b15384cb95550))
    - Add todo ([`87ebc18`](https://github.com/AOSC-Dev/oma/commit/87ebc18929eb44c4b96f0cc9a30822a5277ff440))
    - Do not const Writer::default as WRITER ([`f310ff2`](https://github.com/AOSC-Dev/oma/commit/f310ff2486eaba37b8e659991429a81dfea4dff7))
    - Add oma-topics crate ([`df5692d`](https://github.com/AOSC-Dev/oma/commit/df5692d9cd2dea3e882205dcce6d0558b539e279))
</details>

## v0.1.1 (2023-09-05)

<csr-id-57003169329e01d60172d3531e7f3817bacf46da/>
<csr-id-4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac/>
<csr-id-ae9e8606bb35ace1db58d2ce88dff0545892c9c8/>
<csr-id-4731867afda8d6eb3397d383b8a26bf04c4f8364/>
<csr-id-831f0c306f3f362af003da83a8ccc4351031d1df/>
<csr-id-57fcaa531bc827a8661cf2a4f0f8a50c39289277/>
<csr-id-0e14c25a9f5ad34da79df93cd3e686e81323f320/>
<csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/>
<csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/>
<csr-id-1a3d60e8665faf452a217a478bf0b1c7ce3e445b/>
<csr-id-db0240919451651f494b5bb6a828240c6310b9c7/>
<csr-id-2e01d440b78c79d07b1f04ef67866865934c4049/>
<csr-id-5622e3699691081f0de4466379c14bc539e69c11/>
<csr-id-b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef/>
<csr-id-87ebc18929eb44c4b96f0cc9a30822a5277ff440/>
<csr-id-f310ff2486eaba37b8e659991429a81dfea4dff7/>
<csr-id-df5692d9cd2dea3e882205dcce6d0558b539e279/>
<csr-id-95dd8757eca13ade18af1fea8435336a956a7406/>

### Chore

 - <csr-id-57003169329e01d60172d3531e7f3817bacf46da/> Adapt tokio enabled feature
 - <csr-id-4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac/> Adjust some deps (again)
 - <csr-id-ae9e8606bb35ace1db58d2ce88dff0545892c9c8/> Add changelog
 - <csr-id-4731867afda8d6eb3397d383b8a26bf04c4f8364/> Fill in comment, desc and license
 - <csr-id-831f0c306f3f362af003da83a8ccc4351031d1df/> Use oma-apt-sources-list crate (own fork)
 - <csr-id-57fcaa531bc827a8661cf2a4f0f8a50c39289277/> Inquire -> oma-inquire
 - <csr-id-0e14c25a9f5ad34da79df93cd3e686e81323f320/> Drop useless dep
 - <csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/> No need to use tracing

### New Features

 - <csr-id-bff724f28c2943ef490c4c7b0a2b15384cb95550/> Add download local source feature

### Bug Fixes

 - <csr-id-2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5/> Clear decompress progress bar

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt

### Refactor

 - <csr-id-1a3d60e8665faf452a217a478bf0b1c7ce3e445b/> Inner reqwest::Client
 - <csr-id-db0240919451651f494b5bb6a828240c6310b9c7/> Oma topics is back
 - <csr-id-2e01d440b78c79d07b1f04ef67866865934c4049/> Fill of error output (100%)
 - <csr-id-5622e3699691081f0de4466379c14bc539e69c11/> Use async
 - <csr-id-b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef/> Improve debug marco
 - <csr-id-87ebc18929eb44c4b96f0cc9a30822a5277ff440/> Add todo
 - <csr-id-f310ff2486eaba37b8e659991429a81dfea4dff7/> Do not const Writer::default as WRITER
 - <csr-id-df5692d9cd2dea3e882205dcce6d0558b539e279/> Add oma-topics crate

### Style

 - <csr-id-95dd8757eca13ade18af1fea8435336a956a7406/> Run cargo clippy and cargo fmt to lint code

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
<csr-id-e3101b38d83114b13e89024a0fa21246eca764e5/>

### Chore

 - <csr-id-9b58969c8836740f4d205fba10f4857b70674070/> fill in comment, desc and license
 - <csr-id-bbade3d123272c927ece6a8c0d7ef0a5d2f20ee9/> use oma-apt-sources-list crate (own fork)
 - <csr-id-a9dbffa13072234f00b3058d68e2c61ff48a5cb5/> inquire -> oma-inquire
 - <csr-id-e408f1d2e34e132b74a3b91b09d904f536a4e184/> drop useless dep
 - <csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/> no need to use tracing

### Chore

 - <csr-id-e3101b38d83114b13e89024a0fa21246eca764e5/> add changelog

### New Features

 - <csr-id-888b7dc90264c1dcce301c2e4350442d8a137478/> Add download local source feature

### Bug Fixes

 - <csr-id-948b6d93cd92ea9b52b0bb00f302ce037c6bc4ae/> Clear decompress progress bar

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


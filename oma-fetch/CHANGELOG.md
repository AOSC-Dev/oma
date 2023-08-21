# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.1 (2023-08-21)

### Other

 - <csr-id-42a30f3c99799b933d4ae663c543376d9644c634/> fmt

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fmt ([`42a30f3`](https://github.com/AOSC-Dev/oma/commit/42a30f3c99799b933d4ae663c543376d9644c634))
</details>

## v0.1.0 (2023-08-18)

<csr-id-063342819b6d1350c06f268f90c04e125096aee4/>
<csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/>
<csr-id-bbe38a4fafc8c87a602f78175ae02d3edb60c794/>
<csr-id-a6e9e31fd80bdce5faea0162d3b7b47379dff987/>
<csr-id-718d2ebf3b11fe3e7859d55f0e6b08346a8e6b5f/>
<csr-id-31d6abe71e498a660b191542b120b44d98d34d2c/>
<csr-id-3ef5ec5a6832a01f4ce85b40f754efd4bcc55514/>
<csr-id-b84f130fad9fed69f9ca66a283c4a99db558b5fd/>
<csr-id-bb833287d6d439c622e737148d609c1b848e5efa/>
<csr-id-653fb5a711e50c1d686dfc82ed99cbe5508bf03e/>

### Chore

 - <csr-id-063342819b6d1350c06f268f90c04e125096aee4/> Fill in comment, desc, license
 - <csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/> No need to use tracing

### Chore

 - <csr-id-653fb5a711e50c1d686dfc82ed99cbe5508bf03e/> Add changelog

### New Features

 - <csr-id-44e6cdd45bdc46a9e28cb277456f8d9f602f5671/> Display done message if env is not atty
 - <csr-id-d0dfc7bdafa46443654c119bc0c774e3a0f9b387/> Download compress file after extract
 - <csr-id-b68f74f150559c020643e8ded32b1b03089c4bae/> Api change to support multi mirror download
 - <csr-id-33308c75c1070aaaefa6c92330a4bf56c89fe6ed/> Improve try_http_download error handle
 - <csr-id-924fc2bcf11e48f04776ce085237404480110f1f/> Add retry times

### Bug Fixes

 - <csr-id-4f5b4b641687620028a8574b829f1bbb1ecf1759/> Use progress bar println to display message
 - <csr-id-1df53643e761c81b14d3b265bbb992c5e175a239/> Merge master 5d6d2e82f0125d4c8f871228b8cbeb3de53260f1 change
 - <csr-id-b40fc7d2ec46274865adcd529f28a17ecd8f73e9/> Adapt new oma-fetch api
 - <csr-id-948b6d93cd92ea9b52b0bb00f302ce037c6bc4ae/> Clear decompress progress bar

### Refactor

 - <csr-id-bbe38a4fafc8c87a602f78175ae02d3edb60c794/> Remove useless file; lint
 - <csr-id-a6e9e31fd80bdce5faea0162d3b7b47379dff987/> Use builder api design
 - <csr-id-718d2ebf3b11fe3e7859d55f0e6b08346a8e6b5f/> Fill of error translate (50%)
 - <csr-id-31d6abe71e498a660b191542b120b44d98d34d2c/> Do some todo
 - <csr-id-3ef5ec5a6832a01f4ce85b40f754efd4bcc55514/> Done 1
 - <csr-id-b84f130fad9fed69f9ca66a283c4a99db558b5fd/> Do not handle result in start_download function

### Style

 - <csr-id-bb833287d6d439c622e737148d609c1b848e5efa/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release.
 - 19 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-fetch v0.1.0 ([`aac4df6`](https://github.com/AOSC-Dev/oma/commit/aac4df6c53c9ebb0de1b695cda4bb9f0b6c1fb04))
    - Add changelog ([`653fb5a`](https://github.com/AOSC-Dev/oma/commit/653fb5a711e50c1d686dfc82ed99cbe5508bf03e))
    - Fill in comment, desc, license ([`0633428`](https://github.com/AOSC-Dev/oma/commit/063342819b6d1350c06f268f90c04e125096aee4))
    - Display done message if env is not atty ([`44e6cdd`](https://github.com/AOSC-Dev/oma/commit/44e6cdd45bdc46a9e28cb277456f8d9f602f5671))
    - Use progress bar println to display message ([`4f5b4b6`](https://github.com/AOSC-Dev/oma/commit/4f5b4b641687620028a8574b829f1bbb1ecf1759))
    - Remove useless file; lint ([`bbe38a4`](https://github.com/AOSC-Dev/oma/commit/bbe38a4fafc8c87a602f78175ae02d3edb60c794))
    - Download compress file after extract ([`d0dfc7b`](https://github.com/AOSC-Dev/oma/commit/d0dfc7bdafa46443654c119bc0c774e3a0f9b387))
    - Use builder api design ([`a6e9e31`](https://github.com/AOSC-Dev/oma/commit/a6e9e31fd80bdce5faea0162d3b7b47379dff987))
    - Fill of error translate (50%) ([`718d2eb`](https://github.com/AOSC-Dev/oma/commit/718d2ebf3b11fe3e7859d55f0e6b08346a8e6b5f))
    - Merge master 5d6d2e82f0125d4c8f871228b8cbeb3de53260f1 change ([`1df5364`](https://github.com/AOSC-Dev/oma/commit/1df53643e761c81b14d3b265bbb992c5e175a239))
    - Do some todo ([`31d6abe`](https://github.com/AOSC-Dev/oma/commit/31d6abe71e498a660b191542b120b44d98d34d2c))
    - Cargo fmt ([`75b6c86`](https://github.com/AOSC-Dev/oma/commit/75b6c866b398d90ee55655e29c436303673b8a52))
    - Fix cargo clippy ([`687af7c`](https://github.com/AOSC-Dev/oma/commit/687af7c78c4ec7f7454ef5dafc300568b0bee354))
    - No need to use tracing ([`fa15124`](https://github.com/AOSC-Dev/oma/commit/fa15124038b9eaf8234766b33a98297c62d5b001))
    - Adapt new oma-fetch api ([`b40fc7d`](https://github.com/AOSC-Dev/oma/commit/b40fc7d2ec46274865adcd529f28a17ecd8f73e9))
    - Api change to support multi mirror download ([`b68f74f`](https://github.com/AOSC-Dev/oma/commit/b68f74f150559c020643e8ded32b1b03089c4bae))
    - Improve try_http_download error handle ([`33308c7`](https://github.com/AOSC-Dev/oma/commit/33308c75c1070aaaefa6c92330a4bf56c89fe6ed))
    - Add retry times ([`924fc2b`](https://github.com/AOSC-Dev/oma/commit/924fc2bcf11e48f04776ce085237404480110f1f))
    - Run cargo clippy and cargo fmt to lint code ([`bb83328`](https://github.com/AOSC-Dev/oma/commit/bb833287d6d439c622e737148d609c1b848e5efa))
    - Clear decompress progress bar ([`948b6d9`](https://github.com/AOSC-Dev/oma/commit/948b6d93cd92ea9b52b0bb00f302ce037c6bc4ae))
    - Done 1 ([`3ef5ec5`](https://github.com/AOSC-Dev/oma/commit/3ef5ec5a6832a01f4ce85b40f754efd4bcc55514))
    - 6 ([`4b4d394`](https://github.com/AOSC-Dev/oma/commit/4b4d394642e2df41382b608ab4784793727a79bd))
    - Some changes(4) ([`51780a0`](https://github.com/AOSC-Dev/oma/commit/51780a08a9f9f3b3a62fc968e9897673bcd882a0))
    - Do not handle result in start_download function ([`b84f130`](https://github.com/AOSC-Dev/oma/commit/b84f130fad9fed69f9ca66a283c4a99db558b5fd))
    - Some change ([`4e2e98b`](https://github.com/AOSC-Dev/oma/commit/4e2e98b722c908078293a8d6553665ecb4614b41))
    - Oma-fetcher -> oma-fetch ([`2cc68ad`](https://github.com/AOSC-Dev/oma/commit/2cc68ade26e0882863fa0a1dde715ab3982cb222))
</details>


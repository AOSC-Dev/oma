# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.2 (2023-09-19)

### Chore

 - <csr-id-0270767e885e10dc5f81c82daa60d6acb5bbe27b/> Bump all dep oma-console version

### Documentation

 - <csr-id-2da19a790dc2bb43aafa7d2c28971e2e56635c93/> Add some comment

### Bug Fixes

 - <csr-id-8b9f6e9e4596bef76e5b82c6b40e819456b140d6/> Request head failed clear progress bar

### Refactor

 - <csr-id-106191aee434077f178150f06ccb36889c482317/> Refactor clone (2)
 - <csr-id-a8c2b4a48b64dd106322200cd920c62c30d558b0/> Refactor clone (1)
 - <csr-id-1487c4af4f4e7d7d5f8c51b571d90848cacec465/> Some var no need to clone
 - <csr-id-b205d6813b4366b4f3475538127be099845ac2e5/> Use Arc to clone callback
 - <csr-id-c8721caeb9b5314599b56be4fc6c482d8e65e191/> Oma_spinner and oma_style_pb function inner unwrap
   Because the template is fixed, the template should not return any errors

### Style

 - <csr-id-24dd59fba6f2056d1f7dd4ee9b094b557d29abe4/> Run cargo clippy and cargo fmt to lint code
 - <csr-id-c3f7ae354f99479bdd487d98e8872d53f0e33a59/> Fix clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 7 calendar days.
 - 12 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Run cargo clippy and cargo fmt to lint code ([`24dd59f`](https://github.com/AOSC-Dev/oma/commit/24dd59fba6f2056d1f7dd4ee9b094b557d29abe4))
    - Fix clippy ([`c3f7ae3`](https://github.com/AOSC-Dev/oma/commit/c3f7ae354f99479bdd487d98e8872d53f0e33a59))
    - Refactor clone (2) ([`106191a`](https://github.com/AOSC-Dev/oma/commit/106191aee434077f178150f06ccb36889c482317))
    - Refactor clone (1) ([`a8c2b4a`](https://github.com/AOSC-Dev/oma/commit/a8c2b4a48b64dd106322200cd920c62c30d558b0))
    - Some var no need to clone ([`1487c4a`](https://github.com/AOSC-Dev/oma/commit/1487c4af4f4e7d7d5f8c51b571d90848cacec465))
    - Use Arc to clone callback ([`b205d68`](https://github.com/AOSC-Dev/oma/commit/b205d6813b4366b4f3475538127be099845ac2e5))
    - Request head failed clear progress bar ([`8b9f6e9`](https://github.com/AOSC-Dev/oma/commit/8b9f6e9e4596bef76e5b82c6b40e819456b140d6))
    - Bump all dep oma-console version ([`0270767`](https://github.com/AOSC-Dev/oma/commit/0270767e885e10dc5f81c82daa60d6acb5bbe27b))
    - Oma_spinner and oma_style_pb function inner unwrap ([`c8721ca`](https://github.com/AOSC-Dev/oma/commit/c8721caeb9b5314599b56be4fc6c482d8e65e191))
    - Add some comment ([`2da19a7`](https://github.com/AOSC-Dev/oma/commit/2da19a790dc2bb43aafa7d2c28971e2e56635c93))
</details>

## v0.3.1 (2023-09-06)

<csr-id-6436e59e5891fb21ccc512f884003af415209aa4/>

### Refactor

 - <csr-id-6436e59e5891fb21ccc512f884003af415209aa4/> Refactor try_donwload function to SingleDownloader impl to fix clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-fetch v0.3.1 ([`1e65ad3`](https://github.com/AOSC-Dev/oma/commit/1e65ad3641b396cb5c6e8675b431d4b176f9e314))
    - Refactor try_donwload function to SingleDownloader impl to fix clippy ([`6436e59`](https://github.com/AOSC-Dev/oma/commit/6436e59e5891fb21ccc512f884003af415209aa4))
</details>

## v0.3.0 (2023-09-06)

### New Features

 - <csr-id-a0750502605cabb6d7385f1cbc96edf639324cb5/> Add DownloadEvent::AllDone to allow control global progress bar finish and clear
 - <csr-id-13018326745688027422575eb5a364a050c4c691/> Add --no-progress option to no output progress bar to terminal

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-fetch v0.3.0, safety bump 2 crates ([`0959dfb`](https://github.com/AOSC-Dev/oma/commit/0959dfb5414f46c96d7b7aa39c485bdc1d3862de))
    - Add DownloadEvent::AllDone to allow control global progress bar finish and clear ([`a075050`](https://github.com/AOSC-Dev/oma/commit/a0750502605cabb6d7385f1cbc96edf639324cb5))
    - Add --no-progress option to no output progress bar to terminal ([`1301832`](https://github.com/AOSC-Dev/oma/commit/13018326745688027422575eb5a364a050c4c691))
</details>

## v0.2.0 (2023-09-05)

<csr-id-8f2cb7c6f2bf4e118d0b5fe17105a4a2fd6164f5/>
<csr-id-1943b764ee60248d6c02f820e50cdc1e5d73716b/>
<csr-id-b4283b72c5e8ed5ffaed7ca27fe345d0d43394dd/>
<csr-id-1875106a3ac133a463bb1c251ba11b5b8b1429d6/>

### New Features

 - <csr-id-0ee47f5f866bc12c59955bd88822bb2e487af743/> Switch to callback event, no more indicatif in oma-fetch

### Refactor

 - <csr-id-8f2cb7c6f2bf4e118d0b5fe17105a4a2fd6164f5/> Adapt oma-fetch new API
 - <csr-id-1943b764ee60248d6c02f820e50cdc1e5d73716b/> Adapt new oma-fetch api

### Style

 - <csr-id-b4283b72c5e8ed5ffaed7ca27fe345d0d43394dd/> Improve code style
 - <csr-id-1875106a3ac133a463bb1c251ba11b5b8b1429d6/> Use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 2 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-fetch v0.2.0, safety bump 2 crates ([`3d643f9`](https://github.com/AOSC-Dev/oma/commit/3d643f98588d93c60a094808b794624e78d464b7))
    - Improve code style ([`b4283b7`](https://github.com/AOSC-Dev/oma/commit/b4283b72c5e8ed5ffaed7ca27fe345d0d43394dd))
    - Use cargo-fmt to format code ([`1875106`](https://github.com/AOSC-Dev/oma/commit/1875106a3ac133a463bb1c251ba11b5b8b1429d6))
    - Adapt oma-fetch new API ([`8f2cb7c`](https://github.com/AOSC-Dev/oma/commit/8f2cb7c6f2bf4e118d0b5fe17105a4a2fd6164f5))
    - Adapt new oma-fetch api ([`1943b76`](https://github.com/AOSC-Dev/oma/commit/1943b764ee60248d6c02f820e50cdc1e5d73716b))
    - Switch to callback event, no more indicatif in oma-fetch ([`0ee47f5`](https://github.com/AOSC-Dev/oma/commit/0ee47f5f866bc12c59955bd88822bb2e487af743))
</details>

## v0.1.3 (2023-09-02)

<csr-id-57003169329e01d60172d3531e7f3817bacf46da/>
<csr-id-922fb8aa093a6050c4fdc848f2e5fab369db6095/>

### Chore

 - <csr-id-57003169329e01d60172d3531e7f3817bacf46da/> Adapt tokio enabled feature
 - <csr-id-922fb8aa093a6050c4fdc848f2e5fab369db6095/> Adjust some deps

### New Features

 - <csr-id-2fe754324e2eb2b3d43c89e162059bfaffeabae3/> Add translate

### Bug Fixes

 - <csr-id-7f5dea18cdda862fc36fe2d4560ff10ed07baa1d/> Escape url try to fix can not download '+' in package name packages
 - <csr-id-30b2f5c194cdb2fe74b74a20b200ebfb340b118c/> Fix warning message display

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 7 calendar days.
 - 7 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma-fetch v0.1.3 ([`808db0b`](https://github.com/AOSC-Dev/oma/commit/808db0bef0e9b4c001d1c2e1a291bd2d7a4a1871))
    - Escape url try to fix can not download '+' in package name packages ([`7f5dea1`](https://github.com/AOSC-Dev/oma/commit/7f5dea18cdda862fc36fe2d4560ff10ed07baa1d))
    - Add translate ([`2fe7543`](https://github.com/AOSC-Dev/oma/commit/2fe754324e2eb2b3d43c89e162059bfaffeabae3))
    - Fix warning message display ([`30b2f5c`](https://github.com/AOSC-Dev/oma/commit/30b2f5c194cdb2fe74b74a20b200ebfb340b118c))
    - Adapt tokio enabled feature ([`5700316`](https://github.com/AOSC-Dev/oma/commit/57003169329e01d60172d3531e7f3817bacf46da))
    - Adjust some deps ([`922fb8a`](https://github.com/AOSC-Dev/oma/commit/922fb8aa093a6050c4fdc848f2e5fab369db6095))
</details>

## v0.1.2 (2023-08-26)

<csr-id-aa5a70e9fbb44a2ee75f1d8d3e7923a867a81a2f/>
<csr-id-8d69e5d695da8e25a89274fd5ca562a01c8a39f5/>
<csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/>
<csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/>
<csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/>
<csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/>
<csr-id-24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b/>
<csr-id-7560c558cbfc68ccb488bac29aa15477e74d9607/>
<csr-id-88efbe1e674c3a3030144ad3b0690d1e2095cdaf/>
<csr-id-53c3f0ea394ef470cb7be1d5dec077ba923cb860/>
<csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/>

### Chore

 - <csr-id-aa5a70e9fbb44a2ee75f1d8d3e7923a867a81a2f/> Add changelog
 - <csr-id-8d69e5d695da8e25a89274fd5ca562a01c8a39f5/> Fill in comment, desc, license
 - <csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/> No need to use tracing

### New Features

 - <csr-id-996417b5a659b404729d79522d02d11561b0375d/> Display done message if env is not atty
 - <csr-id-67c9c44809f1ae091913d851fc2e8b18163eb037/> Download compress file after extract
 - <csr-id-bf8ecc46a1741fc725e19d6727b1329fc429aa80/> Api change to support multi mirror download
 - <csr-id-15a7ecc8638cc7d1591e6e0611ba58066e7a81a6/> Improve try_http_download error handle
 - <csr-id-62cf61992658f55f86456a788c2490521d8ff48b/> Add retry times

### Bug Fixes

<csr-id-00958c5b1824a4cbd32aafed5e899ca7da596c82/>
<csr-id-e8f4fc32507d33fa24aaa71c474b2ce0d936ca37/>
<csr-id-6ff39b47d20f24e194187e1c0a35f3f4f615d410/>
<csr-id-2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5/>

 - <csr-id-3d4a16a9c675a5ee8ed0bbcabd152fdd78761052/> Fix local mirror package fetch
 - <csr-id-d6b4d8d439403fed3fa4dab41d205b29c77c052a/> Fix local mirror download url get
 - <csr-id-f9148fd48f07bedb08c4ccb4099df634de1228b0/> Fix oma upgrade loop not return
   - Also clippy and fmt

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt

### Refactor

 - <csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/> Remove useless file; lint
 - <csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/> Use builder api design
 - <csr-id-24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b/> Fill of error translate (50%)
 - <csr-id-7560c558cbfc68ccb488bac29aa15477e74d9607/> Do some todo
 - <csr-id-88efbe1e674c3a3030144ad3b0690d1e2095cdaf/> Done 1
 - <csr-id-53c3f0ea394ef470cb7be1d5dec077ba923cb860/> Do not handle result in start_download function

### Style

 - <csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 32 commits contributed to the release over the course of 4 calendar days.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma-console v0.1.1, oma-fetch v0.1.2, oma-utils v0.1.4, oma-pm v0.2.1 ([`64f5d1b`](https://github.com/AOSC-Dev/oma/commit/64f5d1bf4f93b7b3b1f5a00134e232409458e5e3))
    - Fix local mirror package fetch ([`3d4a16a`](https://github.com/AOSC-Dev/oma/commit/3d4a16a9c675a5ee8ed0bbcabd152fdd78761052))
    - Fix local mirror download url get ([`d6b4d8d`](https://github.com/AOSC-Dev/oma/commit/d6b4d8d439403fed3fa4dab41d205b29c77c052a))
    - Fix oma upgrade loop not return ([`f9148fd`](https://github.com/AOSC-Dev/oma/commit/f9148fd48f07bedb08c4ccb4099df634de1228b0))
    - Bump oma-fetch v0.1.1, oma-utils v0.1.1, oma-pm v0.2.0 ([`51b4ab2`](https://github.com/AOSC-Dev/oma/commit/51b4ab259c5fe014493c78e04f5c6671f56d95e8))
    - Fmt ([`9bb6e19`](https://github.com/AOSC-Dev/oma/commit/9bb6e19a703bc76515a7fa70c19aaafef38c7d7b))
    - Release oma-fetch v0.1.0 ([`44310c7`](https://github.com/AOSC-Dev/oma/commit/44310c73d6f24473fd7ecab0d56f3d97a7164f65))
    - Add changelog ([`aa5a70e`](https://github.com/AOSC-Dev/oma/commit/aa5a70e9fbb44a2ee75f1d8d3e7923a867a81a2f))
    - Fill in comment, desc, license ([`8d69e5d`](https://github.com/AOSC-Dev/oma/commit/8d69e5d695da8e25a89274fd5ca562a01c8a39f5))
    - Display done message if env is not atty ([`996417b`](https://github.com/AOSC-Dev/oma/commit/996417b5a659b404729d79522d02d11561b0375d))
    - Use progress bar println to display message ([`00958c5`](https://github.com/AOSC-Dev/oma/commit/00958c5b1824a4cbd32aafed5e899ca7da596c82))
    - Remove useless file; lint ([`336b02c`](https://github.com/AOSC-Dev/oma/commit/336b02cd7f1e950d028724c11d2318bed0495ddc))
    - Download compress file after extract ([`67c9c44`](https://github.com/AOSC-Dev/oma/commit/67c9c44809f1ae091913d851fc2e8b18163eb037))
    - Use builder api design ([`b097de9`](https://github.com/AOSC-Dev/oma/commit/b097de9165dc0f1a8d970b750c84d6f5fc8ead81))
    - Fill of error translate (50%) ([`24ca3e6`](https://github.com/AOSC-Dev/oma/commit/24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b))
    - Merge master 5d6d2e82f0125d4c8f871228b8cbeb3de53260f1 change ([`e8f4fc3`](https://github.com/AOSC-Dev/oma/commit/e8f4fc32507d33fa24aaa71c474b2ce0d936ca37))
    - Do some todo ([`7560c55`](https://github.com/AOSC-Dev/oma/commit/7560c558cbfc68ccb488bac29aa15477e74d9607))
    - Cargo fmt ([`b0f6954`](https://github.com/AOSC-Dev/oma/commit/b0f69541f4d8baa5abb92d1db2e73fe6dc4c71f5))
    - Fix cargo clippy ([`6757986`](https://github.com/AOSC-Dev/oma/commit/6757986e906cafe053bffd13dd6768931beb87ea))
    - No need to use tracing ([`0ca5be7`](https://github.com/AOSC-Dev/oma/commit/0ca5be73a7ddb70e3a07b63ef21f2f873e420832))
    - Adapt new oma-fetch api ([`6ff39b4`](https://github.com/AOSC-Dev/oma/commit/6ff39b47d20f24e194187e1c0a35f3f4f615d410))
    - Api change to support multi mirror download ([`bf8ecc4`](https://github.com/AOSC-Dev/oma/commit/bf8ecc46a1741fc725e19d6727b1329fc429aa80))
    - Improve try_http_download error handle ([`15a7ecc`](https://github.com/AOSC-Dev/oma/commit/15a7ecc8638cc7d1591e6e0611ba58066e7a81a6))
    - Add retry times ([`62cf619`](https://github.com/AOSC-Dev/oma/commit/62cf61992658f55f86456a788c2490521d8ff48b))
    - Run cargo clippy and cargo fmt to lint code ([`9de51fa`](https://github.com/AOSC-Dev/oma/commit/9de51fa2cf2993c10acfd05d3cda133e6140ac44))
    - Clear decompress progress bar ([`2f40bc8`](https://github.com/AOSC-Dev/oma/commit/2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5))
    - Done 1 ([`88efbe1`](https://github.com/AOSC-Dev/oma/commit/88efbe1e674c3a3030144ad3b0690d1e2095cdaf))
    - 6 ([`9b195b0`](https://github.com/AOSC-Dev/oma/commit/9b195b04f2f7e224f096aa6c04aaba56c55b1698))
    - Some changes(4) ([`6450e2d`](https://github.com/AOSC-Dev/oma/commit/6450e2d2a7588d958be39cbecb375872422277f2))
    - Do not handle result in start_download function ([`53c3f0e`](https://github.com/AOSC-Dev/oma/commit/53c3f0ea394ef470cb7be1d5dec077ba923cb860))
    - Some change ([`5d16784`](https://github.com/AOSC-Dev/oma/commit/5d16784215b2c47059c335e5f03c94ffaaf63693))
    - Oma-fetcher -> oma-fetch ([`70e11f8`](https://github.com/AOSC-Dev/oma/commit/70e11f8d3354a5989b4576fe924f66c5f7ec72ac))
</details>

## v0.1.1 (2023-08-21)

<csr-id-42a30f3c99799b933d4ae663c543376d9644c634/>

### Other

 - <csr-id-42a30f3c99799b933d4ae663c543376d9644c634/> fmt

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


# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.44.0 (2023-06-18)

### Chore

 - <csr-id-4395c373c3ff9f230dc125216d6377d4350a5625/> Update all deps

### New Features

 - <csr-id-daf799250993dc3c5a72133d25c1b6bd8a28a0a1/> Log oma run result status

### Refactor

 - <csr-id-a68dda43bba8c2625c2f211d607bb580f0a37f39/> Refact install_handle_error and install_handle_error_retry

### Style

 - <csr-id-ccff288b2afad69b005399e87ee2a2c6f6f328ec/> Use cargo-fmt to format code
 - <csr-id-11868fe261de648e91bc1f38304ac0bb79a31c6f/> Remove useless line

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 6 calendar days.
 - 7 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update all deps ([`4395c37`](https://github.com/AOSC-Dev/oma/commit/4395c373c3ff9f230dc125216d6377d4350a5625))
    - Use cargo-fmt to format code ([`ccff288`](https://github.com/AOSC-Dev/oma/commit/ccff288b2afad69b005399e87ee2a2c6f6f328ec))
    - Refact install_handle_error and install_handle_error_retry ([`a68dda4`](https://github.com/AOSC-Dev/oma/commit/a68dda43bba8c2625c2f211d607bb580f0a37f39))
    - Remove useless line ([`11868fe`](https://github.com/AOSC-Dev/oma/commit/11868fe261de648e91bc1f38304ac0bb79a31c6f))
    - Log oma run result status ([`daf7992`](https://github.com/AOSC-Dev/oma/commit/daf799250993dc3c5a72133d25c1b6bd8a28a0a1))
</details>

## v0.43.2 (2023-06-11)

### New Features

 - <csr-id-1525de50d022646693239f58fd854dfb85c40c6c/> Use default clap style
 - <csr-id-3aa5d097fa342fb1ae89b151e183eda8a5ca2560/> Only action is non empty push to oma history undo list

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.43.2 ([`e6d5ce0`](https://github.com/AOSC-Dev/oma/commit/e6d5ce0b2c72c054c726b24225ac62dfe22f959d))
    - Use default clap style ([`1525de5`](https://github.com/AOSC-Dev/oma/commit/1525de50d022646693239f58fd854dfb85c40c6c))
    - Only action is non empty push to oma history undo list ([`3aa5d09`](https://github.com/AOSC-Dev/oma/commit/3aa5d097fa342fb1ae89b151e183eda8a5ca2560))
</details>

## v0.43.1 (2023-06-11)

<csr-id-8c5fe2fa6f9eea02530e8af34cdaa20f3826008e/>

### Chore

 - <csr-id-8c5fe2fa6f9eea02530e8af34cdaa20f3826008e/> Update all deps

### Bug Fixes

 - <csr-id-62395e8949da5d018c1f12556069846bc0bfe740/> Improve error message context in fetch local mirror (file://)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.43.1 ([`7ea54a3`](https://github.com/AOSC-Dev/oma/commit/7ea54a37236cc372968a0d98c3998dbede17418c))
    - Update all deps ([`8c5fe2f`](https://github.com/AOSC-Dev/oma/commit/8c5fe2fa6f9eea02530e8af34cdaa20f3826008e))
    - Improve error message context in fetch local mirror (file://) ([`62395e8`](https://github.com/AOSC-Dev/oma/commit/62395e8949da5d018c1f12556069846bc0bfe740))
</details>

## v0.43.0 (2023-06-10)

<csr-id-4bb7d618a4f44013c71a190b2beb3f65c4a1968e/>

### Chore

 - <csr-id-4bb7d618a4f44013c71a190b2beb3f65c4a1968e/> Update all deps

### New Features

 - <csr-id-923d761748ed310531ec41e1442b87591735cfaf/> Improve contents-may-not-be-accurate tips
 - <csr-id-e549ec48cf80349849ce923349ee021e4316ef26/> New line for oma history undo tips

### Bug Fixes

 - <csr-id-4c99f9f93acdf79ea40f05f5341e30b3e1598262/> Sometimes progressbar stdout eat oma message
 - <csr-id-a733c87727b784b52d9c9cb71faa0919de86206e/> Use modified() to get update time delta
 - <csr-id-f5153f30d785bd4303a29f826878f2446c385d5b/> Undo opration tips should display 'redo'
 - <csr-id-ec1aaf97fa34e68f11f67f1d13bfa134669d0dd6/> Do not display downloading package tips if user ctrlc pending ui

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.43.0 ([`3347287`](https://github.com/AOSC-Dev/oma/commit/3347287eac42d6086442a85f91581f0182e7eb18))
    - Update all deps ([`4bb7d61`](https://github.com/AOSC-Dev/oma/commit/4bb7d618a4f44013c71a190b2beb3f65c4a1968e))
    - Sometimes progressbar stdout eat oma message ([`4c99f9f`](https://github.com/AOSC-Dev/oma/commit/4c99f9f93acdf79ea40f05f5341e30b3e1598262))
    - Improve contents-may-not-be-accurate tips ([`923d761`](https://github.com/AOSC-Dev/oma/commit/923d761748ed310531ec41e1442b87591735cfaf))
    - Use modified() to get update time delta ([`a733c87`](https://github.com/AOSC-Dev/oma/commit/a733c87727b784b52d9c9cb71faa0919de86206e))
    - Undo opration tips should display 'redo' ([`f5153f3`](https://github.com/AOSC-Dev/oma/commit/f5153f30d785bd4303a29f826878f2446c385d5b))
    - New line for oma history undo tips ([`e549ec4`](https://github.com/AOSC-Dev/oma/commit/e549ec48cf80349849ce923349ee021e4316ef26))
    - Do not display downloading package tips if user ctrlc pending ui ([`ec1aaf9`](https://github.com/AOSC-Dev/oma/commit/ec1aaf97fa34e68f11f67f1d13bfa134669d0dd6))
</details>

## v0.42.0 (2023-06-09)

<csr-id-e176ec5d29beb33ce76fb4bf0690a22aa9e28f89/>
<csr-id-ec50be31b8390bd1d55a6f4a3144ab6440c0edf7/>

### Chore

 - <csr-id-e176ec5d29beb33ce76fb4bf0690a22aa9e28f89/> Update all deps

### New Features

 - <csr-id-2a739e3cf3dba998680a7c66f6a75a0bf3115362/> Improve redo logic
   redo is now generated relative to undo, and only undo operations are recorded in the redo list
   
   - Also improve history undo/redo output

### Style

 - <csr-id-ec50be31b8390bd1d55a6f4a3144ab6440c0edf7/> Use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.42.0 ([`cdfb973`](https://github.com/AOSC-Dev/oma/commit/cdfb973ec8ee168501a3fd131a064841a6f9e421))
    - Update all deps ([`e176ec5`](https://github.com/AOSC-Dev/oma/commit/e176ec5d29beb33ce76fb4bf0690a22aa9e28f89))
    - Use cargo-fmt to format code ([`ec50be3`](https://github.com/AOSC-Dev/oma/commit/ec50be31b8390bd1d55a6f4a3144ab6440c0edf7))
    - Improve redo logic ([`2a739e3`](https://github.com/AOSC-Dev/oma/commit/2a739e3cf3dba998680a7c66f6a75a0bf3115362))
</details>

## v0.41.1 (2023-06-08)

<csr-id-1db7f9e05ea27cf2697ddb3b574a511b002bbea7/>
<csr-id-e7d051f30ecaf57ba034527f3865d8e02c0a54f5/>
<csr-id-414d36259011d1f6b4fd069eb1f4ad08b0c64e6e/>
<csr-id-4b727fc3353a9a3952b50c2a82dee6902745e9ad/>

### Chore

 - <csr-id-1db7f9e05ea27cf2697ddb3b574a511b002bbea7/> Update all deps
 - <csr-id-e7d051f30ecaf57ba034527f3865d8e02c0a54f5/> Remove useless line in Cargo.toml
 - <csr-id-414d36259011d1f6b4fd069eb1f4ad08b0c64e6e/> Update all deps

### Bug Fixes

 - <csr-id-6c1d26de1dbe96153075b49a210af90adc9ff06e/> Sometimes progress bar println message not print to new line
 - <csr-id-c2c797783eae0cd0de0379d5b102507c6e95b4a3/> Add missing fish completions
 - <csr-id-028876bd245c1b37ebcb0f1204548647cc1bca38/> Use console::measure_text_width to calc string width to fix sometimes strip prefix will panic
 - <csr-id-984070707447aa7813485d9ce9de08845cd6e662/> Fix some subcommand packages name completion
 - <csr-id-4ceaad64f54ada9a86e00a29e6f70c656235fc34/> Add some missing oma bash completions

### Style

 - <csr-id-4b727fc3353a9a3952b50c2a82dee6902745e9ad/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.41.1 ([`0dec098`](https://github.com/AOSC-Dev/oma/commit/0dec09802bd057f50b22406c46a18f6210e9690e))
    - Run cargo clippy and cargo fmt to lint code ([`4b727fc`](https://github.com/AOSC-Dev/oma/commit/4b727fc3353a9a3952b50c2a82dee6902745e9ad))
    - Update all deps ([`1db7f9e`](https://github.com/AOSC-Dev/oma/commit/1db7f9e05ea27cf2697ddb3b574a511b002bbea7))
    - Remove useless line in Cargo.toml ([`e7d051f`](https://github.com/AOSC-Dev/oma/commit/e7d051f30ecaf57ba034527f3865d8e02c0a54f5))
    - Sometimes progress bar println message not print to new line ([`6c1d26d`](https://github.com/AOSC-Dev/oma/commit/6c1d26de1dbe96153075b49a210af90adc9ff06e))
    - Update all deps ([`414d362`](https://github.com/AOSC-Dev/oma/commit/414d36259011d1f6b4fd069eb1f4ad08b0c64e6e))
    - Add missing fish completions ([`c2c7977`](https://github.com/AOSC-Dev/oma/commit/c2c797783eae0cd0de0379d5b102507c6e95b4a3))
    - Use console::measure_text_width to calc string width to fix sometimes strip prefix will panic ([`028876b`](https://github.com/AOSC-Dev/oma/commit/028876bd245c1b37ebcb0f1204548647cc1bca38))
    - Fix some subcommand packages name completion ([`9840707`](https://github.com/AOSC-Dev/oma/commit/984070707447aa7813485d9ce9de08845cd6e662))
    - Add some missing oma bash completions ([`4ceaad6`](https://github.com/AOSC-Dev/oma/commit/4ceaad64f54ada9a86e00a29e6f70c656235fc34))
</details>

## v0.41.0 (2023-06-06)

<csr-id-f1000e691f6f864347399a5e62ce0e3d5e4ca788/>
<csr-id-dabaf02e65e8629f0a95776b43afbc3540d75e55/>
<csr-id-0d3effffc68041d3211e3e5acdc068d20d90b01d/>
<csr-id-8071d6834f9162d9e41fb6c071c9704b5556c0f2/>
<csr-id-a0ecccc92ebb6b0dc6273396c92b2fd163578505/>
<csr-id-565e0e037d0c6675af1f2f020b2dd92f92d18f53/>
<csr-id-49e63a18ff3b291f4514e9603bc0fe9120443b5e/>
<csr-id-20c5644a284e25cced38134606183be231f97d17/>
<csr-id-97bf425e5f1e1ff404f0b2ca490c4321c0ec3373/>
<csr-id-9f0a6a19373e93981272973a3b853972f87dbad0/>
<csr-id-93f71e7adff5d7e85c6a5af4f9f8b271e124c3e1/>
<csr-id-a2ca88dd53fa1898b22efd0ca5807f06485b9627/>
<csr-id-b2eb60d2fde1f32fb076ada021a53f706c464670/>
<csr-id-7c7a30c5c6a736e8d5b29d2efb718766d67947a5/>
<csr-id-f9e4f5681934724940a33beef4956801ef578eda/>
<csr-id-e929eb8c8571e53608ef639c537bc0e19946060b/>
<csr-id-b1690feaa325ba1168de1b746c6dbcd3314c5d57/>
<csr-id-fd232b39f97f001c74d4e0fed5e9f4c17b03ab61/>
<csr-id-21116def098438f8edca492861f4c9514a5e51db/>
<csr-id-1a67fa9e8a87f05ee214d97d262aa458557a9e71/>
<csr-id-256b84deb2478a27d6c513a525f246889042cf39/>
<csr-id-d958f44148c76e95eab149262a85164beebf2677/>
<csr-id-a62831f2a0a16fc4e4d24887e1f6a258c110ee4e/>
<csr-id-76932b61a9f607c3d955b03180d4a7e718f73454/>
<csr-id-16e7824302f8d9ed69d93093b3aacc760e2a7bb6/>
<csr-id-77e34ca606251225acf9b2113638acca86caa47a/>
<csr-id-dd966219220843ca12f9ae6e1657eeb7be7d1b53/>
<csr-id-d874e9171c05be88608125394b355f05277e0b44/>
<csr-id-3f300df031dd40b108c1e592b3d649338729cc25/>
<csr-id-ae446030b0b16de5da53c11b6dcf09752c65586f/>

### Chore

 - <csr-id-f1000e691f6f864347399a5e62ce0e3d5e4ca788/> Update all deps
 - <csr-id-dabaf02e65e8629f0a95776b43afbc3540d75e55/> Add not automatic build i18n method
 - <csr-id-0d3effffc68041d3211e3e5acdc068d20d90b01d/> If i18n translate updated re run build.rs script
   - Also lint code
 - <csr-id-8071d6834f9162d9e41fb6c071c9704b5556c0f2/> Remove git rebase comment

### New Features

<csr-id-27fa7a798cbd4172551b0ccfccd252019de557cc/>
<csr-id-d1187cadb15b381354849774e54340b608344227/>
<csr-id-ba9a16543092f5822579ca9c90ceb498162c76ab/>
<csr-id-c1690170900d505813c30aa6ce3e46436b109ca4/>
<csr-id-59f887d02d29d98cf432f8f1ee44bb321844e0a5/>
<csr-id-7e8d0eed46e2e8cba5553f7fd01499f468e2a8f3/>
<csr-id-6cc6b0b3af8952fbaa4e74ac224bb79df0cf9bad/>
<csr-id-ed226a0122f889755f05aed1837f81aa4d284b9f/>
<csr-id-8837159a29ca633d943285439ce7096d59f99844/>
<csr-id-5263de3401b98a7e543969074a43df40d0c333d5/>
<csr-id-1c547dd10072cb42193c0dc9858ad51e9c7b18c8/>
<csr-id-07b60b286723edc5ca111df8823b89aa26d99b47/>
<csr-id-728cfc7bb29cd18c042f1fed366e6b7dbdb83245/>
<csr-id-f5e0c95384ea51f22d5732bc34e242689a35b585/>
<csr-id-2e6c1fef66188c444dd010f8c9cd51b691a7d11b/>

 - <csr-id-678d28c75686650680596c0b2e16761b45195a94/> If action not empty display undo tips
 - <csr-id-a5aaaea5891b442e85e64d725f5a028fe640a4fd/> Add missing op done tips
 - <csr-id-5887f8314c14509440e0e3424ae8df472d950322/> Add oma optration done undo tips
 - <csr-id-c71116ede1e4e552a85fe051ac5e58ba5471a5be/> Add oma history feature
 - <csr-id-09d507e7b1cbb659a02500e63d1df83d7a3cc873/> Oma download add --with-deps flag to download package(s) with deps
   - Also fix path argument requires packages argument

### Bug Fixes

 - <csr-id-732e17ed2d517c1d38259a6a479edd607f8be6d3/> Fluent some need use string
 - <csr-id-ff4844c38c812bd7c28b972e17cdbde48c466376/> Use fluent new line format
 - <csr-id-18412e16c84125ebcc6abc0e9cd2b0f5359eede2/> Remove useless " in oma.ftl
 - <csr-id-2c05590feb6aa963b4c59f283d7f4224cec7077b/> Fix do-not-edit-topic-sources-list new line
 - <csr-id-44a0444dc706a292a4a8eea2cc63f246f3812555/> Remove useless entry in oma.ftl
 - <csr-id-243ce7a272ed6097968fa76e4f3c5137313a9dd0/> Fix some provide package order

### Other

 - <csr-id-a0ecccc92ebb6b0dc6273396c92b2fd163578505/> (zh-CN) complete localization
 - <csr-id-565e0e037d0c6675af1f2f020b2dd92f92d18f53/> (en-US) improve UI strings
 - <csr-id-49e63a18ff3b291f4514e9603bc0fe9120443b5e/> Sync en-US translate string to zh-CN
 - <csr-id-20c5644a284e25cced38134606183be231f97d17/> Add all history string to i18n
 - <csr-id-97bf425e5f1e1ff404f0b2ca490c4321c0ec3373/> Adapt some string to i18n; fix redo install package
 - <csr-id-9f0a6a19373e93981272973a3b853972f87dbad0/> (zh-CN) finish translation
 - <csr-id-93f71e7adff5d7e85c6a5af4f9f8b271e124c3e1/> (en-US) tweak wording and punctuation marks
 - <csr-id-a2ca88dd53fa1898b22efd0ca5807f06485b9627/> Fix scan-topic-is-removed name display
 - <csr-id-b2eb60d2fde1f32fb076ada021a53f706c464670/> Fix debug-symbol-available ui string issue
 - <csr-id-7c7a30c5c6a736e8d5b29d2efb718766d67947a5/> Fill zh-CN missing translate template
 - <csr-id-f9e4f5681934724940a33beef4956801ef578eda/> Delete repeated full comma
 - <csr-id-e929eb8c8571e53608ef639c537bc0e19946060b/> Add missing i18n string
 - <csr-id-b1690feaa325ba1168de1b746c6dbcd3314c5d57/> Add 'pick-tips' string
 - <csr-id-fd232b39f97f001c74d4e0fed5e9f4c17b03ab61/> Fix dep-error-desc desc
 - <csr-id-21116def098438f8edca492861f4c9514a5e51db/> Add colon symbol; adjust some output format
 - <csr-id-1a67fa9e8a87f05ee214d97d262aa458557a9e71/> (WIP) zh-CN localization
 - <csr-id-256b84deb2478a27d6c513a525f246889042cf39/> Fix typos in en-US
 - <csr-id-d958f44148c76e95eab149262a85164beebf2677/> Remove 'type to filter item' in topic tips
 - <csr-id-a62831f2a0a16fc4e4d24887e1f6a258c110ee4e/> Reword pid => PID
 - <csr-id-76932b61a9f607c3d955b03180d4a7e718f73454/> Make Omakase speak English

### Refactor

 - <csr-id-16e7824302f8d9ed69d93093b3aacc760e2a7bb6/> Add InstallOptions::default()
 - <csr-id-77e34ca606251225acf9b2113638acca86caa47a/> Refactor db.rs
 - <csr-id-dd966219220843ca12f9ae6e1657eeb7be7d1b53/> Refactor contents.rs
 - <csr-id-d874e9171c05be88608125394b355f05277e0b44/> Remove repeated string

### Style

 - <csr-id-3f300df031dd40b108c1e592b3d649338729cc25/> Run cargo clippy and cargo fmt to lint code
 - <csr-id-ae446030b0b16de5da53c11b6dcf09752c65586f/> Add missing new line symbol in zh-CN/oma.ftl

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 58 commits contributed to the release over the course of 10 calendar days.
 - 11 days passed between releases.
 - 56 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.41.0 ([`7a65d04`](https://github.com/AOSC-Dev/oma/commit/7a65d04a69349ac2e885277e63c1d7b763be7123))
    - Run cargo clippy and cargo fmt to lint code ([`3f300df`](https://github.com/AOSC-Dev/oma/commit/3f300df031dd40b108c1e592b3d649338729cc25))
    - Update all deps ([`f1000e6`](https://github.com/AOSC-Dev/oma/commit/f1000e691f6f864347399a5e62ce0e3d5e4ca788))
    - I18n improve total-download-size and change-storage-usage translate logic ([`26a5f59`](https://github.com/AOSC-Dev/oma/commit/26a5f59452295a237840941f602ac4659224fc52))
    - If action not empty display undo tips ([`678d28c`](https://github.com/AOSC-Dev/oma/commit/678d28c75686650680596c0b2e16761b45195a94))
    - Add missing op done tips ([`a5aaaea`](https://github.com/AOSC-Dev/oma/commit/a5aaaea5891b442e85e64d725f5a028fe640a4fd))
    - (zh-CN) complete localization ([`a0ecccc`](https://github.com/AOSC-Dev/oma/commit/a0ecccc92ebb6b0dc6273396c92b2fd163578505))
    - (en-US) improve UI strings ([`565e0e0`](https://github.com/AOSC-Dev/oma/commit/565e0e037d0c6675af1f2f020b2dd92f92d18f53))
    - Add oma optration done undo tips ([`5887f83`](https://github.com/AOSC-Dev/oma/commit/5887f8314c14509440e0e3424ae8df472d950322))
    - Add missing new line symbol in zh-CN/oma.ftl ([`ae44603`](https://github.com/AOSC-Dev/oma/commit/ae446030b0b16de5da53c11b6dcf09752c65586f))
    - Sync en-US translate string to zh-CN ([`49e63a1`](https://github.com/AOSC-Dev/oma/commit/49e63a18ff3b291f4514e9603bc0fe9120443b5e))
    - Add all history string to i18n ([`20c5644`](https://github.com/AOSC-Dev/oma/commit/20c5644a284e25cced38134606183be231f97d17))
    - Adapt some string to i18n; fix redo install package ([`97bf425`](https://github.com/AOSC-Dev/oma/commit/97bf425e5f1e1ff404f0b2ca490c4321c0ec3373))
    - Add InstallOptions::default() ([`16e7824`](https://github.com/AOSC-Dev/oma/commit/16e7824302f8d9ed69d93093b3aacc760e2a7bb6))
    - Add oma history feature ([`c71116e`](https://github.com/AOSC-Dev/oma/commit/c71116ede1e4e552a85fe051ac5e58ba5471a5be))
    - (zh-CN) finish translation ([`9f0a6a1`](https://github.com/AOSC-Dev/oma/commit/9f0a6a19373e93981272973a3b853972f87dbad0))
    - (en-US) tweak wording and punctuation marks ([`93f71e7`](https://github.com/AOSC-Dev/oma/commit/93f71e7adff5d7e85c6a5af4f9f8b271e124c3e1))
    - Oma download add --with-deps flag to download package(s) with deps ([`09d507e`](https://github.com/AOSC-Dev/oma/commit/09d507e7b1cbb659a02500e63d1df83d7a3cc873))
    - Refactor db.rs ([`77e34ca`](https://github.com/AOSC-Dev/oma/commit/77e34ca606251225acf9b2113638acca86caa47a))
    - Refactor contents.rs ([`dd96621`](https://github.com/AOSC-Dev/oma/commit/dd966219220843ca12f9ae6e1657eeb7be7d1b53))
    - Add not automatic build i18n method ([`dabaf02`](https://github.com/AOSC-Dev/oma/commit/dabaf02e65e8629f0a95776b43afbc3540d75e55))
    - If i18n translate updated re run build.rs script ([`0d3efff`](https://github.com/AOSC-Dev/oma/commit/0d3effffc68041d3211e3e5acdc068d20d90b01d))
    - Fix scan-topic-is-removed name display ([`a2ca88d`](https://github.com/AOSC-Dev/oma/commit/a2ca88dd53fa1898b22efd0ca5807f06485b9627))
    - Remove git rebase comment ([`8071d68`](https://github.com/AOSC-Dev/oma/commit/8071d6834f9162d9e41fb6c071c9704b5556c0f2))
    - Fix debug-symbol-available ui string issue ([`b2eb60d`](https://github.com/AOSC-Dev/oma/commit/b2eb60d2fde1f32fb076ada021a53f706c464670))
    - Fill zh-CN missing translate template ([`7c7a30c`](https://github.com/AOSC-Dev/oma/commit/7c7a30c5c6a736e8d5b29d2efb718766d67947a5))
    - Delete repeated full comma ([`f9e4f56`](https://github.com/AOSC-Dev/oma/commit/f9e4f5681934724940a33beef4956801ef578eda))
    - Add missing i18n string ([`e929eb8`](https://github.com/AOSC-Dev/oma/commit/e929eb8c8571e53608ef639c537bc0e19946060b))
    - Add 'pick-tips' string ([`b1690fe`](https://github.com/AOSC-Dev/oma/commit/b1690feaa325ba1168de1b746c6dbcd3314c5d57))
    - Fix dep-error-desc desc ([`fd232b3`](https://github.com/AOSC-Dev/oma/commit/fd232b39f97f001c74d4e0fed5e9f4c17b03ab61))
    - Add colon symbol; adjust some output format ([`21116de`](https://github.com/AOSC-Dev/oma/commit/21116def098438f8edca492861f4c9514a5e51db))
    - Fluent some need use string ([`732e17e`](https://github.com/AOSC-Dev/oma/commit/732e17ed2d517c1d38259a6a479edd607f8be6d3))
    - (WIP) zh-CN localization ([`1a67fa9`](https://github.com/AOSC-Dev/oma/commit/1a67fa9e8a87f05ee214d97d262aa458557a9e71))
    - Fix typos in en-US ([`256b84d`](https://github.com/AOSC-Dev/oma/commit/256b84deb2478a27d6c513a525f246889042cf39))
    - Remove 'type to filter item' in topic tips ([`d958f44`](https://github.com/AOSC-Dev/oma/commit/d958f44148c76e95eab149262a85164beebf2677))
    - Reword pid => PID ([`a62831f`](https://github.com/AOSC-Dev/oma/commit/a62831f2a0a16fc4e4d24887e1f6a258c110ee4e))
    - Make Omakase speak English ([`76932b6`](https://github.com/AOSC-Dev/oma/commit/76932b61a9f607c3d955b03180d4a7e718f73454))
    - Remove repeated string ([`d874e91`](https://github.com/AOSC-Dev/oma/commit/d874e9171c05be88608125394b355f05277e0b44))
    - Use fluent new line format ([`ff4844c`](https://github.com/AOSC-Dev/oma/commit/ff4844c38c812bd7c28b972e17cdbde48c466376))
    - Add scan topic to remove string ([`27fa7a7`](https://github.com/AOSC-Dev/oma/commit/27fa7a798cbd4172551b0ccfccd252019de557cc))
    - Remove useless " in oma.ftl ([`18412e1`](https://github.com/AOSC-Dev/oma/commit/18412e16c84125ebcc6abc0e9cd2b0f5359eede2))
    - Move help message from inquire to topics.rs to translate ([`d1187ca`](https://github.com/AOSC-Dev/oma/commit/d1187cadb15b381354849774e54340b608344227))
    - Fix do-not-edit-topic-sources-list new line ([`2c05590`](https://github.com/AOSC-Dev/oma/commit/2c05590feb6aa963b4c59f283d7f4224cec7077b))
    - Remove useless entry in oma.ftl ([`44a0444`](https://github.com/AOSC-Dev/oma/commit/44a0444dc706a292a4a8eea2cc63f246f3812555))
    - Add checksum.rs translate template ([`ba9a165`](https://github.com/AOSC-Dev/oma/commit/ba9a16543092f5822579ca9c90ceb498162c76ab))
    - Add contents.rs translate template ([`c169017`](https://github.com/AOSC-Dev/oma/commit/c1690170900d505813c30aa6ce3e46436b109ca4))
    - Add db.rs translate template ([`59f887d`](https://github.com/AOSC-Dev/oma/commit/59f887d02d29d98cf432f8f1ee44bb321844e0a5))
    - Add download.rs translate template ([`7e8d0ee`](https://github.com/AOSC-Dev/oma/commit/7e8d0eed46e2e8cba5553f7fd01499f468e2a8f3))
    - Add formatter.rs translate template ([`6cc6b0b`](https://github.com/AOSC-Dev/oma/commit/6cc6b0b3af8952fbaa4e74ac224bb79df0cf9bad))
    - Add main.rs translate template ([`ed226a0`](https://github.com/AOSC-Dev/oma/commit/ed226a0122f889755f05aed1837f81aa4d284b9f))
    - Add oma.rs translate template ([`8837159`](https://github.com/AOSC-Dev/oma/commit/8837159a29ca633d943285439ce7096d59f99844))
    - Add pager.rs translate template ([`5263de3`](https://github.com/AOSC-Dev/oma/commit/5263de3401b98a7e543969074a43df40d0c333d5))
    - Add tpkgrs translate template ([`1c547dd`](https://github.com/AOSC-Dev/oma/commit/1c547dd10072cb42193c0dc9858ad51e9c7b18c8))
    - Add topics.rs translate template ([`07b60b2`](https://github.com/AOSC-Dev/oma/commit/07b60b286723edc5ca111df8823b89aa26d99b47))
    - Add verify.rs translate template ([`728cfc7`](https://github.com/AOSC-Dev/oma/commit/728cfc7bb29cd18c042f1fed366e6b7dbdb83245))
    - Add utils.rs translate template ([`f5e0c95`](https://github.com/AOSC-Dev/oma/commit/f5e0c95384ea51f22d5732bc34e242689a35b585))
    - Fix some provide package order ([`243ce7a`](https://github.com/AOSC-Dev/oma/commit/243ce7a272ed6097968fa76e4f3c5137313a9dd0))
    - Use indicium search engine to oma search package ([`2e6c1fe`](https://github.com/AOSC-Dev/oma/commit/2e6c1fef66188c444dd010f8c9cd51b691a7d11b))
</details>

## v0.40.0 (2023-05-25)

<csr-id-4e5de74a34f9c127c125e7e9059d6c25651d6659/>
<csr-id-3768394d114bbd9c8636ab2def920763419bf915/>
<csr-id-2c742207b01c9fc4c686fd67022f0b794275abce/>
<csr-id-eb77b477c98d52aa28766c83b11d637ebb0e0c96/>
<csr-id-70f94f455206029d7105fcd85b7c30573a3c96f5/>
<csr-id-e6b0898ec36b9d753ae27eb9a0a623e702181185/>

### Chore

 - <csr-id-4e5de74a34f9c127c125e7e9059d6c25651d6659/> Update all deps
 - <csr-id-3768394d114bbd9c8636ab2def920763419bf915/> Remove useless dep

### Documentation

 - <csr-id-ab3de4aafe40fb66ba59babb4c454640402f07b3/> Add more code comment

### New Features

 - <csr-id-25b577c2c2a5b7d6c528d36e1ddefca3a911bee1/> Add i18n support framework
 - <csr-id-4c192e9bc94957d19880a3fb892ae57980a7ab1f/> Add oma provides/list-files --bin flag to only search bin files
 - <csr-id-ce536884da4af9c42fd12cd7469c41ea663093c2/> Oma contents bin search use strsim to filter result

### Bug Fixes

 - <csr-id-d085919b0049bd1d0922b91a0adfbbea9df2c400/> Fix only noarch topic enable
 - <csr-id-970b6b319c5c2dbf89a4214b2fd5c8878e512ba8/> (again) try to fix unicode width new line display issue
 - <csr-id-d3f59e8b2dd70cb11489ac924ba28184995fb8de/> Fix oma compile and add i18n example
 - <csr-id-19c36ada73231dec8cb2524923694ea681402db2/> Fix oma list-files --bin argument name

### Refactor

 - <csr-id-2c742207b01c9fc4c686fd67022f0b794275abce/> Box dyn Iterator can auto infer type
 - <csr-id-eb77b477c98d52aa28766c83b11d637ebb0e0c96/> No need to use Either

### Style

 - <csr-id-70f94f455206029d7105fcd85b7c30573a3c96f5/> Use cargo-fmt to format code
 - <csr-id-e6b0898ec36b9d753ae27eb9a0a623e702181185/> Run cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 10 calendar days.
 - 11 days passed between releases.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.40.0 ([`1acba8a`](https://github.com/AOSC-Dev/oma/commit/1acba8ad4505edbd98e8682359a7c714692557c0))
    - Use cargo-fmt to format code ([`70f94f4`](https://github.com/AOSC-Dev/oma/commit/70f94f455206029d7105fcd85b7c30573a3c96f5))
    - Update all deps ([`4e5de74`](https://github.com/AOSC-Dev/oma/commit/4e5de74a34f9c127c125e7e9059d6c25651d6659))
    - Fix only noarch topic enable ([`d085919`](https://github.com/AOSC-Dev/oma/commit/d085919b0049bd1d0922b91a0adfbbea9df2c400))
    - Add more code comment ([`ab3de4a`](https://github.com/AOSC-Dev/oma/commit/ab3de4aafe40fb66ba59babb4c454640402f07b3))
    - Remove useless dep ([`3768394`](https://github.com/AOSC-Dev/oma/commit/3768394d114bbd9c8636ab2def920763419bf915))
    - (again) try to fix unicode width new line display issue ([`970b6b3`](https://github.com/AOSC-Dev/oma/commit/970b6b319c5c2dbf89a4214b2fd5c8878e512ba8))
    - Fix oma compile and add i18n example ([`d3f59e8`](https://github.com/AOSC-Dev/oma/commit/d3f59e8b2dd70cb11489ac924ba28184995fb8de))
    - Add i18n support framework ([`25b577c`](https://github.com/AOSC-Dev/oma/commit/25b577c2c2a5b7d6c528d36e1ddefca3a911bee1))
    - Run cargo clippy to lint code ([`e6b0898`](https://github.com/AOSC-Dev/oma/commit/e6b0898ec36b9d753ae27eb9a0a623e702181185))
    - Box dyn Iterator can auto infer type ([`2c74220`](https://github.com/AOSC-Dev/oma/commit/2c742207b01c9fc4c686fd67022f0b794275abce))
    - No need to use Either ([`eb77b47`](https://github.com/AOSC-Dev/oma/commit/eb77b477c98d52aa28766c83b11d637ebb0e0c96))
    - Fix oma list-files --bin argument name ([`19c36ad`](https://github.com/AOSC-Dev/oma/commit/19c36ada73231dec8cb2524923694ea681402db2))
    - Add oma provides/list-files --bin flag to only search bin files ([`4c192e9`](https://github.com/AOSC-Dev/oma/commit/4c192e9bc94957d19880a3fb892ae57980a7ab1f))
    - Oma contents bin search use strsim to filter result ([`ce53688`](https://github.com/AOSC-Dev/oma/commit/ce536884da4af9c42fd12cd7469c41ea663093c2))
</details>

## v0.39.0 (2023-05-14)

<csr-id-4fa16e6cd6a604943ba6e1ce94090dfbb3f24389/>
<csr-id-fa0efe5bc4fd0d995ce180ddf99f5d719713a065/>

### Chore

 - <csr-id-4fa16e6cd6a604943ba6e1ce94090dfbb3f24389/> Update all deps
 - <csr-id-fa0efe5bc4fd0d995ce180ddf99f5d719713a065/> Use zlib-ng to improve performance

### New Features

 - <csr-id-9be10e84ba8467d0e83297e32f02481d5af318e6/> Oma search if strsim score is 1000 (max) display 'full match'
 - <csr-id-90584ac6a1f1f07a93d27355df0928563dc593ce/> Max terminal len limit to 150
 - <csr-id-5930c328df228c92a1dd5249324ed44c3e68fbe5/> Oma download success display downloaded package count
 - <csr-id-ad119c514dd8b8de069086681732c8f0825af522/> Always lowercase search word
 - <csr-id-6b681b4cc1c417dc7a6b38b1de6f20b715adbe9c/> Ignore case search word and pkg description

### Bug Fixes

 - <csr-id-e6ffcd566c2dda829546e40ba578b0c678b4aaf6/> Oma list only installed version display installed
 - <csr-id-b78cb133db492371ffa625f6c4e00d98c3671564/> Oma list glob support
 - <csr-id-f251869bd2d1529a859081fb7d680d74fa7a3f2d/> No need to unlock oma two twice

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.39.0 ([`444cf70`](https://github.com/AOSC-Dev/oma/commit/444cf70a2cc98818a826bbf4ea7ef6d80d3309cd))
    - Update all deps ([`4fa16e6`](https://github.com/AOSC-Dev/oma/commit/4fa16e6cd6a604943ba6e1ce94090dfbb3f24389))
    - Oma search if strsim score is 1000 (max) display 'full match' ([`9be10e8`](https://github.com/AOSC-Dev/oma/commit/9be10e84ba8467d0e83297e32f02481d5af318e6))
    - Max terminal len limit to 150 ([`90584ac`](https://github.com/AOSC-Dev/oma/commit/90584ac6a1f1f07a93d27355df0928563dc593ce))
    - Oma download success display downloaded package count ([`5930c32`](https://github.com/AOSC-Dev/oma/commit/5930c328df228c92a1dd5249324ed44c3e68fbe5))
    - Oma list only installed version display installed ([`e6ffcd5`](https://github.com/AOSC-Dev/oma/commit/e6ffcd566c2dda829546e40ba578b0c678b4aaf6))
    - Oma list glob support ([`b78cb13`](https://github.com/AOSC-Dev/oma/commit/b78cb133db492371ffa625f6c4e00d98c3671564))
    - Always lowercase search word ([`ad119c5`](https://github.com/AOSC-Dev/oma/commit/ad119c514dd8b8de069086681732c8f0825af522))
    - Ignore case search word and pkg description ([`6b681b4`](https://github.com/AOSC-Dev/oma/commit/6b681b4cc1c417dc7a6b38b1de6f20b715adbe9c))
    - No need to unlock oma two twice ([`f251869`](https://github.com/AOSC-Dev/oma/commit/f251869bd2d1529a859081fb7d680d74fa7a3f2d))
    - Use zlib-ng to improve performance ([`fa0efe5`](https://github.com/AOSC-Dev/oma/commit/fa0efe5bc4fd0d995ce180ddf99f5d719713a065))
</details>

## v0.38.2 (2023-05-12)

<csr-id-0d99d40254fbb946f43bb53ee140aeec2db9c1e0/>

### New Features

 - <csr-id-f17b335d8c8d42878de1a042b733826a1a732f1d/> Try to flushing file add progress spinner again
 - <csr-id-128b93057a1002d5ddb7c8761359b53edc20ab90/> Try to flushing file add progress spinner

### Bug Fixes

 - <csr-id-ae9aa241f9e12ccec30e25ff84290c94d66edbe7/> Use tokio::io::copy replaced tokio::fs::copy

### Style

 - <csr-id-0d99d40254fbb946f43bb53ee140aeec2db9c1e0/> Use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.38.2 ([`d84fb3e`](https://github.com/AOSC-Dev/oma/commit/d84fb3ebf58f76fcf3f363b98639f0334bd90d93))
    - Use cargo-fmt to format code ([`0d99d40`](https://github.com/AOSC-Dev/oma/commit/0d99d40254fbb946f43bb53ee140aeec2db9c1e0))
    - Try to flushing file add progress spinner again ([`f17b335`](https://github.com/AOSC-Dev/oma/commit/f17b335d8c8d42878de1a042b733826a1a732f1d))
    - Try to flushing file add progress spinner ([`128b930`](https://github.com/AOSC-Dev/oma/commit/128b93057a1002d5ddb7c8761359b53edc20ab90))
    - Use tokio::io::copy replaced tokio::fs::copy ([`ae9aa24`](https://github.com/AOSC-Dev/oma/commit/ae9aa241f9e12ccec30e25ff84290c94d66edbe7))
</details>

## v0.38.1 (2023-05-12)

<csr-id-ef92fe7328ad1a6df3f5cff77e1be7101b96d25e/>
<csr-id-36acc15bf8a3e58c05f0b13696d6c9053a33c8e6/>
<csr-id-dfb0ea73555f26f993b7dd9f61e95cd884459632/>

### Chore

 - <csr-id-ef92fe7328ad1a6df3f5cff77e1be7101b96d25e/> Update all deps

### New Features

 - <csr-id-abea1ec032fd8b654ce5d0d7ca365038d2790467/> Copy file use fs::copy to improve preforence; use ProgressSpinner to display fetch local source progress

### Bug Fixes

 - <csr-id-94ac90d9cf09580857daf85e39dccee1bb97d12f/> Download global bar reset position if checksum fail and not allow resume
 - <csr-id-e712031bc4aa558dcc922d0878a0c8dcb877c766/> Fix mirror is updated oma refresh will checksum mismatch
 - <csr-id-8fe73f0cae9af09c11cff6e5a7a8875c2732c294/> Half-configure do not mark pkg as reinstall status
 - <csr-id-019878fdd7bc59745c54cd14a0a3cfb4365512af/> Fetch local source inc global bar

### Style

 - <csr-id-36acc15bf8a3e58c05f0b13696d6c9053a33c8e6/> Use cargo clippy to lint code
 - <csr-id-dfb0ea73555f26f993b7dd9f61e95cd884459632/> Use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.38.1 ([`5dca1fc`](https://github.com/AOSC-Dev/oma/commit/5dca1fc5e6751484bb981714a303fdcd7de9348a))
    - Use cargo clippy to lint code ([`36acc15`](https://github.com/AOSC-Dev/oma/commit/36acc15bf8a3e58c05f0b13696d6c9053a33c8e6))
    - Use cargo-fmt to format code ([`dfb0ea7`](https://github.com/AOSC-Dev/oma/commit/dfb0ea73555f26f993b7dd9f61e95cd884459632))
    - Update all deps ([`ef92fe7`](https://github.com/AOSC-Dev/oma/commit/ef92fe7328ad1a6df3f5cff77e1be7101b96d25e))
    - Download global bar reset position if checksum fail and not allow resume ([`94ac90d`](https://github.com/AOSC-Dev/oma/commit/94ac90d9cf09580857daf85e39dccee1bb97d12f))
    - Fix mirror is updated oma refresh will checksum mismatch ([`e712031`](https://github.com/AOSC-Dev/oma/commit/e712031bc4aa558dcc922d0878a0c8dcb877c766))
    - Half-configure do not mark pkg as reinstall status ([`8fe73f0`](https://github.com/AOSC-Dev/oma/commit/8fe73f0cae9af09c11cff6e5a7a8875c2732c294))
    - Fetch local source inc global bar ([`019878f`](https://github.com/AOSC-Dev/oma/commit/019878fdd7bc59745c54cd14a0a3cfb4365512af))
    - Copy file use fs::copy to improve preforence; use ProgressSpinner to display fetch local source progress ([`abea1ec`](https://github.com/AOSC-Dev/oma/commit/abea1ec032fd8b654ce5d0d7ca365038d2790467))
</details>

## v0.38.0 (2023-05-11)

<csr-id-9ea49733edea7d73317b8fafb08b28b6978677dd/>
<csr-id-8d4d28cdcb512fb29f3a0700cccbdd167436e13e/>
<csr-id-8a1b4b6146d637c2954cd8888e2d2914c504a03b/>

### Chore

 - <csr-id-9ea49733edea7d73317b8fafb08b28b6978677dd/> Update all deps

### New Features

<csr-id-dbccc7267b56ebf1cf87585c9eea0570e61fc0dd/>
<csr-id-a4c0f68dd5bfe43b28fbedc448f03bfb71c3fda6/>
<csr-id-a5051258c2bcb5e022ce20333ccc40ac8916b0fd/>

 - <csr-id-f3b31817a4b430a92c4dadbbf1236e62e83a6154/> Improve oma show APT-Source info output
 - <csr-id-d4303a272abb3dc854c84caec0767a9856ca0e2d/> Add more debug output for fetch local source
   - Also cargo-fmt

### Bug Fixes

 - <csr-id-6f7941995a45ed1b1efdaeb6b7e6082bb0feccc2/> Use right method to get apt style source
 - <csr-id-e605c6b57010dbdfbf221863b791248d84de6041/> Fetch local source InRelease inc progress
 - <csr-id-2ed87f681355f811c833d0b095b17ed38363cfcf/> Oma refresh progress bar inc
 - <csr-id-b79395914912e2ff60a8a16d14708197f6b593d9/> Fix run decompress file
 - <csr-id-95c2ee6e565ea27ed1a6831bf2467e9c9fd7c61f/> Fetch local source do not uncompress in local source (from) directory
 - <csr-id-1f72ff0a44cd5c2bc16c71a2a311f82a0b9e5305/> Fetch local source pkg use oma style progress bar

### Refactor

 - <csr-id-8d4d28cdcb512fb29f3a0700cccbdd167436e13e/> Do not read buf all to memory in download_and_extract_db_local method

### Style

 - <csr-id-8a1b4b6146d637c2954cd8888e2d2914c504a03b/> Use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.38.0 ([`1559648`](https://github.com/AOSC-Dev/oma/commit/155964844709b5dc41624fbbb92c986952d0c9db))
    - Update all deps ([`9ea4973`](https://github.com/AOSC-Dev/oma/commit/9ea49733edea7d73317b8fafb08b28b6978677dd))
    - Use cargo clippy to lint code ([`8a1b4b6`](https://github.com/AOSC-Dev/oma/commit/8a1b4b6146d637c2954cd8888e2d2914c504a03b))
    - Use right method to get apt style source ([`6f79419`](https://github.com/AOSC-Dev/oma/commit/6f7941995a45ed1b1efdaeb6b7e6082bb0feccc2))
    - Improve oma show APT-Source info output ([`f3b3181`](https://github.com/AOSC-Dev/oma/commit/f3b31817a4b430a92c4dadbbf1236e62e83a6154))
    - Add more debug output for fetch local source ([`d4303a2`](https://github.com/AOSC-Dev/oma/commit/d4303a272abb3dc854c84caec0767a9856ca0e2d))
    - Revert "feat: use current thread to fetch local source" ([`e854b81`](https://github.com/AOSC-Dev/oma/commit/e854b81d182e27d714751ee83c072c245b8af780))
    - Use current thread to fetch local source ([`dbccc72`](https://github.com/AOSC-Dev/oma/commit/dbccc7267b56ebf1cf87585c9eea0570e61fc0dd))
    - Fetch local source InRelease inc progress ([`e605c6b`](https://github.com/AOSC-Dev/oma/commit/e605c6b57010dbdfbf221863b791248d84de6041))
    - Oma refresh progress bar inc ([`2ed87f6`](https://github.com/AOSC-Dev/oma/commit/2ed87f681355f811c833d0b095b17ed38363cfcf))
    - Fix run decompress file ([`b793959`](https://github.com/AOSC-Dev/oma/commit/b79395914912e2ff60a8a16d14708197f6b593d9))
    - Fetch local source do not uncompress in local source (from) directory ([`95c2ee6`](https://github.com/AOSC-Dev/oma/commit/95c2ee6e565ea27ed1a6831bf2467e9c9fd7c61f))
    - Do not read buf all to memory in download_and_extract_db_local method ([`8d4d28c`](https://github.com/AOSC-Dev/oma/commit/8d4d28cdcb512fb29f3a0700cccbdd167436e13e))
    - Fetch local source pkg use oma style progress bar ([`1f72ff0`](https://github.com/AOSC-Dev/oma/commit/1f72ff0a44cd5c2bc16c71a2a311f82a0b9e5305))
    - Add fetch local source package progress bar ([`a4c0f68`](https://github.com/AOSC-Dev/oma/commit/a4c0f68dd5bfe43b28fbedc448f03bfb71c3fda6))
    - Add some update database debug message ([`a505125`](https://github.com/AOSC-Dev/oma/commit/a5051258c2bcb5e022ce20333ccc40ac8916b0fd))
</details>

## v0.37.1 (2023-05-11)

<csr-id-6af62b0cbd78e1132ffb03ef58c91b395bd840ff/>
<csr-id-d1039634ba86a8c45e3ba3bb617269d792482467/>
<csr-id-7a9a1c1b8ae48e1f56704c2b2802a52dc079cb22/>

### Chore

 - <csr-id-6af62b0cbd78e1132ffb03ef58c91b395bd840ff/> Update all deps

### Bug Fixes

 - <csr-id-cfebaf569cf561905ff1d96971932c9c25074d0d/> Fix fetch local source database filename
 - <csr-id-798fe7964960f61ac876635b3dce5eb080e77cc2/> Check file is exist in download_and_extract_db_local
 - <csr-id-cc6f289d9072db61c724bf9b9c5dc70765981cb5/> Fix fetch local source database file

### Style

 - <csr-id-d1039634ba86a8c45e3ba3bb617269d792482467/> Use cargo fmt to format code
 - <csr-id-7a9a1c1b8ae48e1f56704c2b2802a52dc079cb22/> Use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.37.1 ([`3c3ccc0`](https://github.com/AOSC-Dev/oma/commit/3c3ccc061e6500f19999886214f7485e47b5491d))
    - Fix fetch local source database filename ([`cfebaf5`](https://github.com/AOSC-Dev/oma/commit/cfebaf569cf561905ff1d96971932c9c25074d0d))
    - Check file is exist in download_and_extract_db_local ([`798fe79`](https://github.com/AOSC-Dev/oma/commit/798fe7964960f61ac876635b3dce5eb080e77cc2))
    - Fix fetch local source database file ([`cc6f289`](https://github.com/AOSC-Dev/oma/commit/cc6f289d9072db61c724bf9b9c5dc70765981cb5))
    - Use cargo fmt to format code ([`d103963`](https://github.com/AOSC-Dev/oma/commit/d1039634ba86a8c45e3ba3bb617269d792482467))
    - Use cargo clippy to lint code ([`7a9a1c1`](https://github.com/AOSC-Dev/oma/commit/7a9a1c1b8ae48e1f56704c2b2802a52dc079cb22))
    - Update all deps ([`6af62b0`](https://github.com/AOSC-Dev/oma/commit/6af62b0cbd78e1132ffb03ef58c91b395bd840ff))
</details>

## v0.37.0 (2023-05-11)

### New Features

 - <csr-id-8c5b1a88f5d8830a6406aabbe0b5b82699e74557/> Improve try_download error output
 - <csr-id-d5f6aef89ab7ca0b5fc4fedad1100e66dfe7ab0a/> More precise handling of IOError in the try_download function
 - <csr-id-ca7099ab7129afdee76927cc30680c7fb8e68b11/> Improve download dir not exist error output
 - <csr-id-bdc2ecced9900237bd461384c11d0344487fd331/> Tips user virtual package didn't mark
 - <csr-id-95cf354f9b262eab3d19b69719eb35e89c3d705f/> Search order move package to top if pkg.name == input and installed

### Bug Fixes

<csr-id-cd2ba53f198d2dcacdf4f8c9cd32848a02078067/>
<csr-id-13c0a6fb168b3932b45a29d3a3c7fec9d757127b/>
<csr-id-baeea2a82ab6c3370f011cbbc852cb4567cb10f5/>

 - <csr-id-1ee0b2f9ead57f2056a8d9271cd9c82ec0ce4d22/> Cli::writeln do not output next empty line
   - Also, download method return Reqwest::Error, do not return anyhow

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.37.0 ([`ec53fdc`](https://github.com/AOSC-Dev/oma/commit/ec53fdc10cb8335b5192c873be481123f823db06))
    - Improve try_download error output ([`8c5b1a8`](https://github.com/AOSC-Dev/oma/commit/8c5b1a88f5d8830a6406aabbe0b5b82699e74557))
    - Cli::writeln do not output next empty line ([`1ee0b2f`](https://github.com/AOSC-Dev/oma/commit/1ee0b2f9ead57f2056a8d9271cd9c82ec0ce4d22))
    - More precise handling of IOError in the try_download function ([`d5f6aef`](https://github.com/AOSC-Dev/oma/commit/d5f6aef89ab7ca0b5fc4fedad1100e66dfe7ab0a))
    - Improve download dir not exist error output ([`ca7099a`](https://github.com/AOSC-Dev/oma/commit/ca7099ab7129afdee76927cc30680c7fb8e68b11))
    - Fix run oma install --install-dbg ([`cd2ba53`](https://github.com/AOSC-Dev/oma/commit/cd2ba53f198d2dcacdf4f8c9cd32848a02078067))
    - Tips user virtual package didn't mark ([`bdc2ecc`](https://github.com/AOSC-Dev/oma/commit/bdc2ecced9900237bd461384c11d0344487fd331))
    - Fix run oma fix-broken ([`13c0a6f`](https://github.com/AOSC-Dev/oma/commit/13c0a6fb168b3932b45a29d3a3c7fec9d757127b))
    - Do not display error to due_to in oma topics ([`baeea2a`](https://github.com/AOSC-Dev/oma/commit/baeea2a82ab6c3370f011cbbc852cb4567cb10f5))
    - Search order move package to top if pkg.name == input and installed ([`95cf354`](https://github.com/AOSC-Dev/oma/commit/95cf354f9b262eab3d19b69719eb35e89c3d705f))
</details>

## v0.36.3 (2023-05-09)

<csr-id-53bf31b428bc6565b0063a2a340d5c70b6df2e2c/>
<csr-id-6338d51de8d189fac1d03c2b34373ff91b95e1bb/>
<csr-id-cdc9c38dd30339ae8a34a2df89df3bdc9ae06eef/>

### Chore

 - <csr-id-53bf31b428bc6565b0063a2a340d5c70b6df2e2c/> Update rust-apt version and adapt it

### Bug Fixes

 - <csr-id-e2bf48b14a67e3637fec635e15d02dad4c858458/> Error and due_to to right order

### Refactor

 - <csr-id-6338d51de8d189fac1d03c2b34373ff91b95e1bb/> Abtsract error_due_to method
 - <csr-id-cdc9c38dd30339ae8a34a2df89df3bdc9ae06eef/> Use error_due_to function to easily handle the due_to case

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.36.3 ([`975e78d`](https://github.com/AOSC-Dev/oma/commit/975e78de0991c4391b1a1dcfa0230226bc08602d))
    - Abtsract error_due_to method ([`6338d51`](https://github.com/AOSC-Dev/oma/commit/6338d51de8d189fac1d03c2b34373ff91b95e1bb))
    - Update rust-apt version and adapt it ([`53bf31b`](https://github.com/AOSC-Dev/oma/commit/53bf31b428bc6565b0063a2a340d5c70b6df2e2c))
    - Use error_due_to function to easily handle the due_to case ([`cdc9c38`](https://github.com/AOSC-Dev/oma/commit/cdc9c38dd30339ae8a34a2df89df3bdc9ae06eef))
    - Error and due_to to right order ([`e2bf48b`](https://github.com/AOSC-Dev/oma/commit/e2bf48b14a67e3637fec635e15d02dad4c858458))
</details>

## v0.36.2 (2023-05-09)

<csr-id-61bf4767f4a6b6df3ea9ebeba2d53d9e9b638b36/>

### New Features

 - <csr-id-aaeb44e15374f85b942f332598a673f980312d6f/> Try_download return error display due_to

### Bug Fixes

 - <csr-id-e886c87864ba235145c9e0da6b4d10458de1f7b8/> Do not decompress BinContents

### Style

 - <csr-id-61bf4767f4a6b6df3ea9ebeba2d53d9e9b638b36/> Use cargo clippy and fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.36.2 ([`8def847`](https://github.com/AOSC-Dev/oma/commit/8def847aaa956305fb0ad2aac765b5a42489b927))
    - Use cargo clippy and fmt to lint code ([`61bf476`](https://github.com/AOSC-Dev/oma/commit/61bf4767f4a6b6df3ea9ebeba2d53d9e9b638b36))
    - Do not decompress BinContents ([`e886c87`](https://github.com/AOSC-Dev/oma/commit/e886c87864ba235145c9e0da6b4d10458de1f7b8))
    - Try_download return error display due_to ([`aaeb44e`](https://github.com/AOSC-Dev/oma/commit/aaeb44e15374f85b942f332598a673f980312d6f))
</details>

## v0.36.1 (2023-05-09)

### Bug Fixes

 - <csr-id-ea2ae64d144c1b72f3d790492b167f43304dd32a/> Packages argument after add some argument flag to wrong result

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.36.1 ([`0e1f604`](https://github.com/AOSC-Dev/oma/commit/0e1f604fd1dbd59d6ac1ca3aae7bb667465f631a))
    - Packages argument after add some argument flag to wrong result ([`ea2ae64`](https://github.com/AOSC-Dev/oma/commit/ea2ae64d144c1b72f3d790492b167f43304dd32a))
</details>

## v0.36.0 (2023-05-09)

<csr-id-fc8c14b178067c9dc16c8bbc00dc93ced9a26d3c/>
<csr-id-d1498032a44581c9008723d1b41ffe273abb91e6/>
<csr-id-983f0a5868e439d192c94dedde8315626b8317af/>
<csr-id-8615f21c2ec8bee7e20b11926787240f95dec742/>
<csr-id-52091d27e0522ab36a679b9c05c10be3888afad5/>

### Chore

 - <csr-id-fc8c14b178067c9dc16c8bbc00dc93ced9a26d3c/> update all deps
 - <csr-id-d1498032a44581c9008723d1b41ffe273abb91e6/> update some deps and adapt new rust-apt version

### New Features

 - <csr-id-9b214028c22feee75b09fd8195e01314ab4cc388/> Add more error output in try_download method
 - <csr-id-51df071218161ee9745544b17d0d8684fbfa50f0/> Improve oma repends output

### Bug Fixes

<csr-id-4343d1dfe5a247ca96aa1a9159ebca8ecf5a3380/>
<csr-id-eb87d41fd058c579cac061007aa793fd2dd6a4d4/>

 - <csr-id-866d89f6dddeaea3cf20b1af6e9575a44e78a64b/> Download success break loop in try_download method
 - <csr-id-98838f200b7a164813885f44b0792b56f45bd441/> This loop never actually loops in try_download method
   - Also use cargo clippy

### Refactor

 - <csr-id-983f0a5868e439d192c94dedde8315626b8317af/> use true/false replaced Ok/Err in try_download method
 - <csr-id-8615f21c2ec8bee7e20b11926787240f95dec742/> optimize try_download logic

### Style

 - <csr-id-52091d27e0522ab36a679b9c05c10be3888afad5/> use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release.
 - 2 days passed between releases.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.36.0 ([`9eef033`](https://github.com/AOSC-Dev/oma/commit/9eef0331274e67dfead7c3560b9267a9cfc6ee55))
    - Update all deps ([`fc8c14b`](https://github.com/AOSC-Dev/oma/commit/fc8c14b178067c9dc16c8bbc00dc93ced9a26d3c))
    - Use cargo-fmt to format code ([`52091d2`](https://github.com/AOSC-Dev/oma/commit/52091d27e0522ab36a679b9c05c10be3888afad5))
    - Use true/false replaced Ok/Err in try_download method ([`983f0a5`](https://github.com/AOSC-Dev/oma/commit/983f0a5868e439d192c94dedde8315626b8317af))
    - Add more error output in try_download method ([`9b21402`](https://github.com/AOSC-Dev/oma/commit/9b214028c22feee75b09fd8195e01314ab4cc388))
    - Download success break loop in try_download method ([`866d89f`](https://github.com/AOSC-Dev/oma/commit/866d89f6dddeaea3cf20b1af6e9575a44e78a64b))
    - This loop never actually loops in try_download method ([`98838f2`](https://github.com/AOSC-Dev/oma/commit/98838f200b7a164813885f44b0792b56f45bd441))
    - Do not append decompress file ([`4343d1d`](https://github.com/AOSC-Dev/oma/commit/4343d1dfe5a247ca96aa1a9159ebca8ecf5a3380))
    - Optimize try_download logic ([`8615f21`](https://github.com/AOSC-Dev/oma/commit/8615f21c2ec8bee7e20b11926787240f95dec742))
    - Do not download package success download next package ([`eb87d41`](https://github.com/AOSC-Dev/oma/commit/eb87d41fd058c579cac061007aa793fd2dd6a4d4))
    - Improve oma repends output ([`51df071`](https://github.com/AOSC-Dev/oma/commit/51df071218161ee9745544b17d0d8684fbfa50f0))
    - Update some deps and adapt new rust-apt version ([`d149803`](https://github.com/AOSC-Dev/oma/commit/d1498032a44581c9008723d1b41ffe273abb91e6))
</details>

## v0.35.0 (2023-05-06)

<csr-id-be95c599c7e36fe542e7b4aa3eebce602842d56c/>
<csr-id-71b844d32bdba7ed4df35e7de16c5e7e210ef5dc/>

### New Features

 - <csr-id-5b2006a1bf92580d6bb290752adb72aa5b02305a/> Add oma install --no-install-recommends and --no-install-suggests
 - <csr-id-aab59d4c1e1ceef3fae33882db010ca25b6d4b3e/> Recommend -> recommends, suggest -> suggests in oma install [ARGS]

### Bug Fixes

 - <csr-id-63c1c36fc7ef11e594fbe5ef7818568e70d29c3a/> Fix force-yes, no-install-{recommends,suggests} argument

### Refactor

 - <csr-id-be95c599c7e36fe542e7b4aa3eebce602842d56c/> set Config struct name as AptConfig

### Style

 - <csr-id-71b844d32bdba7ed4df35e7de16c5e7e210ef5dc/> use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.35.0 ([`1a9304f`](https://github.com/AOSC-Dev/oma/commit/1a9304fa3212733f2b6ef6a3f4e453249a0cc121))
    - Use cargo-fmt to format code ([`71b844d`](https://github.com/AOSC-Dev/oma/commit/71b844d32bdba7ed4df35e7de16c5e7e210ef5dc))
    - Set Config struct name as AptConfig ([`be95c59`](https://github.com/AOSC-Dev/oma/commit/be95c599c7e36fe542e7b4aa3eebce602842d56c))
    - Fix force-yes, no-install-{recommends,suggests} argument ([`63c1c36`](https://github.com/AOSC-Dev/oma/commit/63c1c36fc7ef11e594fbe5ef7818568e70d29c3a))
    - Add oma install --no-install-recommends and --no-install-suggests ([`5b2006a`](https://github.com/AOSC-Dev/oma/commit/5b2006a1bf92580d6bb290752adb72aa5b02305a))
    - Recommend -> recommends, suggest -> suggests in oma install [ARGS] ([`aab59d4`](https://github.com/AOSC-Dev/oma/commit/aab59d4c1e1ceef3fae33882db010ca25b6d4b3e))
</details>

## v0.34.0 (2023-05-06)

<csr-id-2b270aec994b01b0fbeb6a4948bde259528edc84/>
<csr-id-84e4ca451e167c79a44412790e2b128c353db8f0/>
<csr-id-0de4fcd240c3f61bbcf89fe0bf5e6dc73e2fe6ec/>
<csr-id-58e2913c9852074c040681d1dc6499f7c7e4bdb4/>
<csr-id-da0655a4ffa0e845cec23c57d854e0675eb03c1f/>

### Chore

 - <csr-id-2b270aec994b01b0fbeb6a4948bde259528edc84/> update all deps

### New Features

<csr-id-0a1bcbb270cd78804a3ae12c1ab615b4e62347fb/>
<csr-id-91acc4314a7558aef5c9a4506e31f92a6417e2bb/>

 - <csr-id-426c6fc1dd6b2a35e788860f5cb6a9d5b5c73af0/> Display command not found error if oma command-not-found no results found
 - <csr-id-84464c26a063e8c8188987af7e2162d39ddf30dc/> Oma install/remove/upgrade -y should display review message
 - <csr-id-eb2af8149e62e5215eac88987bf3941eae3e4313/> Add oma systemd service
 - <csr-id-e00e9ccbd7c99d9ff5e82a2412f3cc2db7366bb4/> Support fish completion
 - <csr-id-8b15ee13c5158f47b370feebe1dfd42fcb7b0b23/> Add shell competions feature
   - Also fix pengding ui display

### Bug Fixes

<csr-id-73f1ce593b7fb4edcc4b9502cb7a147cbf63a821/>

 - <csr-id-9e56acaa1fdbfbc33a2aa41c74d88e8da68ab92c/> Fix wrong oma pkgnames parameter name ...
   ...Wrong parameter name causes the pkgnames method to always pass in a None parameter, which always completes all packages in the database
 - <csr-id-9b1d8b9d417ef81001391d8c59f99ef539902dce/> Fetch database global progress bar overflow
 - <csr-id-4c7174e528ce117dd918acd5f5fbffa09d0ddee0/> Retry 3 times, not 4 (again)
 - <csr-id-ee8c562777b0e3d1c137d55432d10fdf84f836cb/> Retry 3 times, not 4
 - <csr-id-300c9a3f9ebfbab4789db18c3e6fb53223cf99cc/> Apt_lock_inner failed do not retry
   - Also set some error message due to

### Refactor

 - <csr-id-84e4ca451e167c79a44412790e2b128c353db8f0/> no need to collect package list in oma list method
 - <csr-id-0de4fcd240c3f61bbcf89fe0bf5e6dc73e2fe6ec/> optimize main logic
 - <csr-id-58e2913c9852074c040681d1dc6499f7c7e4bdb4/> oma args function return exit code

### Style

 - <csr-id-da0655a4ffa0e845cec23c57d854e0675eb03c1f/> use cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.34.0 ([`4efff56`](https://github.com/AOSC-Dev/oma/commit/4efff56490c77d138302ed39e6912a6fec9f1685))
    - Update all deps ([`2b270ae`](https://github.com/AOSC-Dev/oma/commit/2b270aec994b01b0fbeb6a4948bde259528edc84))
    - Display command not found error if oma command-not-found no results found ([`426c6fc`](https://github.com/AOSC-Dev/oma/commit/426c6fc1dd6b2a35e788860f5cb6a9d5b5c73af0))
    - Use cargo clippy and cargo fmt to lint code ([`da0655a`](https://github.com/AOSC-Dev/oma/commit/da0655a4ffa0e845cec23c57d854e0675eb03c1f))
    - Oma install/remove/upgrade -y should display review message ([`84464c2`](https://github.com/AOSC-Dev/oma/commit/84464c26a063e8c8188987af7e2162d39ddf30dc))
    - Add oma systemd service ([`eb2af81`](https://github.com/AOSC-Dev/oma/commit/eb2af8149e62e5215eac88987bf3941eae3e4313))
    - Fix wrong oma pkgnames parameter name ... ([`9e56aca`](https://github.com/AOSC-Dev/oma/commit/9e56acaa1fdbfbc33a2aa41c74d88e8da68ab92c))
    - No need to collect package list in oma list method ([`84e4ca4`](https://github.com/AOSC-Dev/oma/commit/84e4ca451e167c79a44412790e2b128c353db8f0))
    - Support fish completion ([`e00e9cc`](https://github.com/AOSC-Dev/oma/commit/e00e9ccbd7c99d9ff5e82a2412f3cc2db7366bb4))
    - Add shell competions feature ([`8b15ee1`](https://github.com/AOSC-Dev/oma/commit/8b15ee13c5158f47b370feebe1dfd42fcb7b0b23))
    - Fetch database global progress bar overflow ([`9b1d8b9`](https://github.com/AOSC-Dev/oma/commit/9b1d8b9d417ef81001391d8c59f99ef539902dce))
    - Retry 3 times, not 4 (again) ([`4c7174e`](https://github.com/AOSC-Dev/oma/commit/4c7174e528ce117dd918acd5f5fbffa09d0ddee0))
    - Retry 3 times, not 4 ([`ee8c562`](https://github.com/AOSC-Dev/oma/commit/ee8c562777b0e3d1c137d55432d10fdf84f836cb))
    - Apt_lock_inner failed do not retry ([`300c9a3`](https://github.com/AOSC-Dev/oma/commit/300c9a3f9ebfbab4789db18c3e6fb53223cf99cc))
    - Optimize main logic ([`0de4fcd`](https://github.com/AOSC-Dev/oma/commit/0de4fcd240c3f61bbcf89fe0bf5e6dc73e2fe6ec))
    - Add oma pkgnames for shell completion ([`0a1bcbb`](https://github.com/AOSC-Dev/oma/commit/0a1bcbb270cd78804a3ae12c1ab615b4e62347fb))
    - Return 1 if oma show pkgs result is empty ([`91acc43`](https://github.com/AOSC-Dev/oma/commit/91acc4314a7558aef5c9a4506e31f92a6417e2bb))
    - Oma args function return exit code ([`58e2913`](https://github.com/AOSC-Dev/oma/commit/58e2913c9852074c040681d1dc6499f7c7e4bdb4))
    - Improve UI strings for oma pending ui output ([`73f1ce5`](https://github.com/AOSC-Dev/oma/commit/73f1ce593b7fb4edcc4b9502cb7a147cbf63a821))
</details>

## v0.33.1 (2023-05-04)

<csr-id-22d449d555ff0b3e89cbfd9a8e4048df8d0e9b53/>
<csr-id-9a540fef0793c300c09efc7bbcd8f08320978bc5/>

### Documentation

 - <csr-id-3923169a3183158ffbbc9fb98f466196dcbbb1db/> Improve oma install --install-recommend and --install-suggest help message

### New Features

 - <csr-id-6dd5327f86955c1ebfd62ded1a85b96bffb5cd64/> Add Shell integrations
   Currently available for Bash/Zsh (command-not-found.sh) and Fish (command-not-found.fish).

### Bug Fixes

 - <csr-id-b0d14fb0897be29155859e6cfbe49a71d15e53b8/> Improve command-not-found directions
 - <csr-id-070dd360a4a8c9b2a0d14bf1879f7a7383308aa8/> Push missing fish command-not-found commit
 - <csr-id-261d35a12a213af973ce914c0124e50d1c9a49b0/> Oma command-not-found always return 127
 - <csr-id-2b89d1c356d95c8be17f6f67f56abd2c7ee36739/> Oma command-not-fould should return 127
 - <csr-id-cea61077fe0c70a327b3d644a3dacbb749a8b9fc/> Improve UI strings for oma refresh output

### Other

 - <csr-id-22d449d555ff0b3e89cbfd9a8e4048df8d0e9b53/> license code under GPLv3
   A key dependency, rust-apt, is licensed under GPLv3.
 - <csr-id-9a540fef0793c300c09efc7bbcd8f08320978bc5/> move PolicyKit rules to /data/policykit

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.33.1 ([`ddcffd4`](https://github.com/AOSC-Dev/oma/commit/ddcffd4793c3cee84afbcb7d2fa259a7b77394b9))
    - Improve command-not-found directions ([`b0d14fb`](https://github.com/AOSC-Dev/oma/commit/b0d14fb0897be29155859e6cfbe49a71d15e53b8))
    - Push missing fish command-not-found commit ([`070dd36`](https://github.com/AOSC-Dev/oma/commit/070dd360a4a8c9b2a0d14bf1879f7a7383308aa8))
    - Oma command-not-found always return 127 ([`261d35a`](https://github.com/AOSC-Dev/oma/commit/261d35a12a213af973ce914c0124e50d1c9a49b0))
    - License code under GPLv3 ([`22d449d`](https://github.com/AOSC-Dev/oma/commit/22d449d555ff0b3e89cbfd9a8e4048df8d0e9b53))
    - Move PolicyKit rules to /data/policykit ([`9a540fe`](https://github.com/AOSC-Dev/oma/commit/9a540fef0793c300c09efc7bbcd8f08320978bc5))
    - Add Shell integrations ([`6dd5327`](https://github.com/AOSC-Dev/oma/commit/6dd5327f86955c1ebfd62ded1a85b96bffb5cd64))
    - Oma command-not-fould should return 127 ([`2b89d1c`](https://github.com/AOSC-Dev/oma/commit/2b89d1c356d95c8be17f6f67f56abd2c7ee36739))
    - Improve UI strings for oma refresh output ([`cea6107`](https://github.com/AOSC-Dev/oma/commit/cea61077fe0c70a327b3d644a3dacbb749a8b9fc))
    - Improve oma install --install-recommend and --install-suggest help message ([`3923169`](https://github.com/AOSC-Dev/oma/commit/3923169a3183158ffbbc9fb98f466196dcbbb1db))
</details>

## v0.33.0 (2023-05-04)

<csr-id-b5faa6722eac78f1075095c237fd743249ebd059/>
<csr-id-ebf8d735634625b2fc056b32d9fdae9716657858/>
<csr-id-f5a557166d5d766fbd4bf59aa17006be7f77e3cd/>
<csr-id-478c2e8c9145e6f8d9847358437d09f969ff444d/>
<csr-id-4f8da4aec365ed92e6cc9bb33e10b0bf71f94bdd/>
<csr-id-64d88bf059bf7921b22caeeb947de9e71b7c1a54/>
<csr-id-d08e903866fe01e35b08272ed37b2096f3273a28/>
<csr-id-86432a7f9f8a593136f1caabc416a5f647b3b81d/>
<csr-id-04133e0a6a734bc1170965a4fe837a4e2768c685/>
<csr-id-bbee12732e7357b52c42fa89e07e07a4f613fb31/>
<csr-id-9296db314338057472951a87edc664c0f0cd87e5/>
<csr-id-01e8cfd45cb4602f80c06f427409b6dade1096ce/>
<csr-id-50bca89128856e2c1cea464da64f924fea28e682/>
<csr-id-03ca22e086e8c68dfb3cb0079a4682e19e7ce194/>
<csr-id-fe1d5ffc83de72ca3eed03d9a7fe96f5be4c2ab7/>

### Chore

 - <csr-id-b5faa6722eac78f1075095c237fd743249ebd059/> update all deps
 - <csr-id-ebf8d735634625b2fc056b32d9fdae9716657858/> update all deps

### New Features

 - <csr-id-2ee74ed227972b76ff061a788dae80db938ba72d/> Add terminal bell if oma operation is done
 - <csr-id-a50f65ecd9e79cb5e012b0e2b7a2a6d08e50855c/> Add query upgadable packages progress spinner
 - <csr-id-31634d3338ffadbb636cc0cde70a87617dac1120/> Add more debug for download method
 - <csr-id-b627c8a0b3d9d2171223700e3243c12bb92d5de0/> Oma install add --install-recommend and --install-suggest argument
 - <csr-id-a5e6e1f8e349e17c5fc63f92c558423af9135577/> Add more debug message for needs_fix_system method
 - <csr-id-537d428c0902aa8661b8b3566b99931b248cf104/> Handle if pkg current_state == 4 or 2 (half-install or half-configure)

### Bug Fixes

 - <csr-id-34a190b4a53490e152ffac643e15130729238cde/> Do not panic with display CJK message
 - <csr-id-4cec322874e9fd46d467219a684a36036274802c/> Both contents-all and contents-ARCH must be downloaded

### Refactor

 - <csr-id-f5a557166d5d766fbd4bf59aa17006be7f77e3cd/> no need to collect in search_pkgs method
 - <csr-id-478c2e8c9145e6f8d9847358437d09f969ff444d/> use Box to optimize logic in decompress method
 - <csr-id-4f8da4aec365ed92e6cc9bb33e10b0bf71f94bdd/> use BufReader to decompress package database
 - <csr-id-64d88bf059bf7921b22caeeb947de9e71b7c1a54/> use BinContents to command-not-found feature
   Twice as fast
 - <csr-id-d08e903866fe01e35b08272ed37b2096f3273a28/> no need to download multi contents
 - <csr-id-86432a7f9f8a593136f1caabc416a5f647b3b81d/> no need to collect checksum entry to parse
 - <csr-id-04133e0a6a734bc1170965a4fe837a4e2768c685/> optmize search_pkgs filter logic
   Remove stupid sort after reverse
 - <csr-id-bbee12732e7357b52c42fa89e07e07a4f613fb31/> optmize search_pkgs filter logic again
 - <csr-id-9296db314338057472951a87edc664c0f0cd87e5/> optmize search_pkgs filter logic
 - <csr-id-01e8cfd45cb4602f80c06f427409b6dade1096ce/> no need to collect upgrade package in update_inner method
 - <csr-id-50bca89128856e2c1cea464da64f924fea28e682/> abstract install_other logic

### Style

 - <csr-id-03ca22e086e8c68dfb3cb0079a4682e19e7ce194/> use cargo clippy and cargo fmt to lint code
 - <csr-id-fe1d5ffc83de72ca3eed03d9a7fe96f5be4c2ab7/> use cargo-fmt and cargo-clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.33.0 ([`607dc68`](https://github.com/AOSC-Dev/oma/commit/607dc681251588aa9653b302b657f4fc73cbb0ea))
    - Update all deps ([`b5faa67`](https://github.com/AOSC-Dev/oma/commit/b5faa6722eac78f1075095c237fd743249ebd059))
    - Do not panic with display CJK message ([`34a190b`](https://github.com/AOSC-Dev/oma/commit/34a190b4a53490e152ffac643e15130729238cde))
    - Revert "fix: do not panic with display CJK message" ([`f0da2c3`](https://github.com/AOSC-Dev/oma/commit/f0da2c3ba1c2eed6227f562adc701a62a005c0e5))
    - No need to collect in search_pkgs method ([`f5a5571`](https://github.com/AOSC-Dev/oma/commit/f5a557166d5d766fbd4bf59aa17006be7f77e3cd))
    - Use cargo clippy and cargo fmt to lint code ([`03ca22e`](https://github.com/AOSC-Dev/oma/commit/03ca22e086e8c68dfb3cb0079a4682e19e7ce194))
    - Use Box to optimize logic in decompress method ([`478c2e8`](https://github.com/AOSC-Dev/oma/commit/478c2e8c9145e6f8d9847358437d09f969ff444d))
    - Use BufReader to decompress package database ([`4f8da4a`](https://github.com/AOSC-Dev/oma/commit/4f8da4aec365ed92e6cc9bb33e10b0bf71f94bdd))
    - Use BinContents to command-not-found feature ([`64d88bf`](https://github.com/AOSC-Dev/oma/commit/64d88bf059bf7921b22caeeb947de9e71b7c1a54))
    - Revert "refactor: no need to download multi contents" ([`4e98e2f`](https://github.com/AOSC-Dev/oma/commit/4e98e2fe3df4a934da6c51aa8e8ceefb314d0d52))
    - No need to download multi contents ([`d08e903`](https://github.com/AOSC-Dev/oma/commit/d08e903866fe01e35b08272ed37b2096f3273a28))
    - No need to collect checksum entry to parse ([`86432a7`](https://github.com/AOSC-Dev/oma/commit/86432a7f9f8a593136f1caabc416a5f647b3b81d))
    - Both contents-all and contents-ARCH must be downloaded ([`4cec322`](https://github.com/AOSC-Dev/oma/commit/4cec322874e9fd46d467219a684a36036274802c))
    - Optmize search_pkgs filter logic ([`04133e0`](https://github.com/AOSC-Dev/oma/commit/04133e0a6a734bc1170965a4fe837a4e2768c685))
    - Optmize search_pkgs filter logic again ([`bbee127`](https://github.com/AOSC-Dev/oma/commit/bbee12732e7357b52c42fa89e07e07a4f613fb31))
    - Optmize search_pkgs filter logic ([`9296db3`](https://github.com/AOSC-Dev/oma/commit/9296db314338057472951a87edc664c0f0cd87e5))
    - Update all deps ([`ebf8d73`](https://github.com/AOSC-Dev/oma/commit/ebf8d735634625b2fc056b32d9fdae9716657858))
    - Use cargo-fmt and cargo-clippy to lint code ([`fe1d5ff`](https://github.com/AOSC-Dev/oma/commit/fe1d5ffc83de72ca3eed03d9a7fe96f5be4c2ab7))
    - Add terminal bell if oma operation is done ([`2ee74ed`](https://github.com/AOSC-Dev/oma/commit/2ee74ed227972b76ff061a788dae80db938ba72d))
    - Add query upgadable packages progress spinner ([`a50f65e`](https://github.com/AOSC-Dev/oma/commit/a50f65ecd9e79cb5e012b0e2b7a2a6d08e50855c))
    - No need to collect upgrade package in update_inner method ([`01e8cfd`](https://github.com/AOSC-Dev/oma/commit/01e8cfd45cb4602f80c06f427409b6dade1096ce))
    - Abstract install_other logic ([`50bca89`](https://github.com/AOSC-Dev/oma/commit/50bca89128856e2c1cea464da64f924fea28e682))
    - Add more debug for download method ([`31634d3`](https://github.com/AOSC-Dev/oma/commit/31634d3338ffadbb636cc0cde70a87617dac1120))
    - Oma install add --install-recommend and --install-suggest argument ([`b627c8a`](https://github.com/AOSC-Dev/oma/commit/b627c8a0b3d9d2171223700e3243c12bb92d5de0))
    - Add more debug message for needs_fix_system method ([`a5e6e1f`](https://github.com/AOSC-Dev/oma/commit/a5e6e1f8e349e17c5fc63f92c558423af9135577))
    - Handle if pkg current_state == 4 or 2 (half-install or half-configure) ([`537d428`](https://github.com/AOSC-Dev/oma/commit/537d428c0902aa8661b8b3566b99931b248cf104))
</details>

## v0.32.2 (2023-05-02)

### Bug Fixes

 - <csr-id-0a04b48961228b8b1707e3eaa174d203a4940101/> Truncate file and set file length == 0 if file_size >= download total_size
 - <csr-id-978173eeb9c6f0a02bf30d8664953f39a7d33f70/> Fetch inrelease return checksum mismatch error if mirror inrelease is updated

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.32.2 ([`6522355`](https://github.com/AOSC-Dev/oma/commit/6522355b898ca9f1b48d6dfc5a8ddfe4a896aff9))
    - Truncate file and set file length == 0 if file_size >= download total_size ([`0a04b48`](https://github.com/AOSC-Dev/oma/commit/0a04b48961228b8b1707e3eaa174d203a4940101))
    - Fetch inrelease return checksum mismatch error if mirror inrelease is updated ([`978173e`](https://github.com/AOSC-Dev/oma/commit/978173eeb9c6f0a02bf30d8664953f39a7d33f70))
</details>

## v0.32.1 (2023-05-02)

<csr-id-49b899d8ab92e65ec58bf415f8f10602e7db17f7/>
<csr-id-90e16b1aec960b0e0b93df2ebb59fc24b0120e70/>
<csr-id-bfb455e98dcc16a35cf21003603f9e17f1448f91/>

### Chore

 - <csr-id-49b899d8ab92e65ec58bf415f8f10602e7db17f7/> update anstream to 0.3.2

### New Features

 - <csr-id-ba2c3a9b70228e8bcdc441e9dbb5f60f8384bfef/> Return 0 if operation allow ctrlc
 - <csr-id-277deccd77b17033f15f04b3aaa779df8af27baa/> Open new thread to check contents file metadata

### Bug Fixes

 - <csr-id-6fa58bdb10707c00d6a2ab05bff59053d74bb45f/> Oma mark needs root

### Refactor

 - <csr-id-90e16b1aec960b0e0b93df2ebb59fc24b0120e70/> optmize local mirror download and extract logic
 - <csr-id-bfb455e98dcc16a35cf21003603f9e17f1448f91/> optmize download db logic again
   - Do not checksum multi times
   - Do not download contents compress file multi times

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.32.1 ([`d853a23`](https://github.com/AOSC-Dev/oma/commit/d853a238c275f9314ca5ae9fae96cab39dbfb4e7))
    - Update anstream to 0.3.2 ([`49b899d`](https://github.com/AOSC-Dev/oma/commit/49b899d8ab92e65ec58bf415f8f10602e7db17f7))
    - Optmize local mirror download and extract logic ([`90e16b1`](https://github.com/AOSC-Dev/oma/commit/90e16b1aec960b0e0b93df2ebb59fc24b0120e70))
    - Return 0 if operation allow ctrlc ([`ba2c3a9`](https://github.com/AOSC-Dev/oma/commit/ba2c3a9b70228e8bcdc441e9dbb5f60f8384bfef))
    - Optmize download db logic again ([`bfb455e`](https://github.com/AOSC-Dev/oma/commit/bfb455e98dcc16a35cf21003603f9e17f1448f91))
    - Oma mark needs root ([`6fa58bd`](https://github.com/AOSC-Dev/oma/commit/6fa58bdb10707c00d6a2ab05bff59053d74bb45f))
    - Open new thread to check contents file metadata ([`277decc`](https://github.com/AOSC-Dev/oma/commit/277deccd77b17033f15f04b3aaa779df8af27baa))
</details>

## v0.32.0 (2023-05-01)

### New Features

 - <csr-id-6307688b3e1aee0bf812deecdec4ebe094da310d/> Adjust terminal width < 90 progress bar style

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.32.0 ([`4e98d70`](https://github.com/AOSC-Dev/oma/commit/4e98d70c098c0eb787be0d92609b9cd2afde2f84))
    - Adjust terminal width < 90 progress bar style ([`6307688`](https://github.com/AOSC-Dev/oma/commit/6307688b3e1aee0bf812deecdec4ebe094da310d))
</details>

## v0.31.1 (2023-05-01)

<csr-id-beed5ad4733033ea1281b6050f9b7e6818bb6a45/>

### New Features

 - <csr-id-e74d9aa3a0921fcf08956d1fff745bbf8700b4d0/> Check contents create time to tell user contents file may not be accurate
 - <csr-id-dc413adb87c15882ceb005c1ede88ff62b73a1dd/> Display searching contents message if match is empty

### Bug Fixes

 - <csr-id-dc3e49db2b5e6860d9c8a36215898507d77fbf99/> Do not panic with display CJK message

### Refactor

 - <csr-id-beed5ad4733033ea1281b6050f9b7e6818bb6a45/> download progress spinner no need to use new thread wait request send

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.31.1 ([`9a9d1e4`](https://github.com/AOSC-Dev/oma/commit/9a9d1e43ad7dc9d4479c0426b3ea0df7eb48c050))
    - Do not panic with display CJK message ([`dc3e49d`](https://github.com/AOSC-Dev/oma/commit/dc3e49db2b5e6860d9c8a36215898507d77fbf99))
    - Check contents create time to tell user contents file may not be accurate ([`e74d9aa`](https://github.com/AOSC-Dev/oma/commit/e74d9aa3a0921fcf08956d1fff745bbf8700b4d0))
    - Display searching contents message if match is empty ([`dc413ad`](https://github.com/AOSC-Dev/oma/commit/dc413adb87c15882ceb005c1ede88ff62b73a1dd))
    - Download progress spinner no need to use new thread wait request send ([`beed5ad`](https://github.com/AOSC-Dev/oma/commit/beed5ad4733033ea1281b6050f9b7e6818bb6a45))
</details>

## v0.31.0 (2023-04-30)

<csr-id-234ef60de4c0ce7760a9cb9dd5b3cc47e2426a42/>
<csr-id-aa21dda2098c5e13d5d1f42427a53f50a8967332/>
<csr-id-7e710d224c977242dbe17185420fdb00e094d2d2/>
<csr-id-da89ed6a92c0560ae0bce410a7a382adc90d3eef/>
<csr-id-3843cbd58f893b84877ae34b4671e44fb8c0a1fb/>
<csr-id-c8dbd5a7f2302460f368dd62b65d04d5370cbe46/>

### Documentation

 - <csr-id-a9b8e62bc8a6a6d2b0fde904af4185f0b0afc5d5/> Add some comment in download method

### New Features

 - <csr-id-cffa40f1a9aaf94c0353c10fd195a89ed135fc43/> Display resume info
 - <csr-id-164a1343c172f2461310e0d9edb5809955c0055a/> Improve ui string
 - <csr-id-e7d65922b5ea92f96a0be3b0a58e4d9defd192af/> Do not inc global bar if file exist and running checksum

### Refactor

 - <csr-id-234ef60de4c0ce7760a9cb9dd5b3cc47e2426a42/> re use validator to improve checksum
 - <csr-id-aa21dda2098c5e13d5d1f42427a53f50a8967332/> improve download methold open file times
 - <csr-id-7e710d224c977242dbe17185420fdb00e094d2d2/> use validator to verify integrity while downloading
 - <csr-id-da89ed6a92c0560ae0bce410a7a382adc90d3eef/> improve get file_size logic

### Style

 - <csr-id-3843cbd58f893b84877ae34b4671e44fb8c0a1fb/> use cargo-clippy to lint code
 - <csr-id-c8dbd5a7f2302460f368dd62b65d04d5370cbe46/> inline function in download method

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.31.0 ([`036a04e`](https://github.com/AOSC-Dev/oma/commit/036a04e464ffb263adc5b35e075eee01ccf2eb41))
    - Use cargo-clippy to lint code ([`3843cbd`](https://github.com/AOSC-Dev/oma/commit/3843cbd58f893b84877ae34b4671e44fb8c0a1fb))
    - Revert "feat: display resume info" ([`698e983`](https://github.com/AOSC-Dev/oma/commit/698e98393d154301cb66b85f0f21a9ef913fade5))
    - Display resume info ([`cffa40f`](https://github.com/AOSC-Dev/oma/commit/cffa40f1a9aaf94c0353c10fd195a89ed135fc43))
    - Improve ui string ([`164a134`](https://github.com/AOSC-Dev/oma/commit/164a1343c172f2461310e0d9edb5809955c0055a))
    - Inline function in download method ([`c8dbd5a`](https://github.com/AOSC-Dev/oma/commit/c8dbd5a7f2302460f368dd62b65d04d5370cbe46))
    - Re use validator to improve checksum ([`234ef60`](https://github.com/AOSC-Dev/oma/commit/234ef60de4c0ce7760a9cb9dd5b3cc47e2426a42))
    - Revert "feat: do not inc global bar if file exist and running checksum" ([`2b20204`](https://github.com/AOSC-Dev/oma/commit/2b202043a80c6330f67b3e4878bb86df94d57924))
    - Do not inc global bar if file exist and running checksum ([`e7d6592`](https://github.com/AOSC-Dev/oma/commit/e7d65922b5ea92f96a0be3b0a58e4d9defd192af))
    - Improve download methold open file times ([`aa21dda`](https://github.com/AOSC-Dev/oma/commit/aa21dda2098c5e13d5d1f42427a53f50a8967332))
    - Add some comment in download method ([`a9b8e62`](https://github.com/AOSC-Dev/oma/commit/a9b8e62bc8a6a6d2b0fde904af4185f0b0afc5d5))
    - Use validator to verify integrity while downloading ([`7e710d2`](https://github.com/AOSC-Dev/oma/commit/7e710d224c977242dbe17185420fdb00e094d2d2))
    - Revert "refactor: improve get file_size logic" ([`568eae9`](https://github.com/AOSC-Dev/oma/commit/568eae9de08906b2ab8e61a3bb21e4e1c573cfe5))
    - Improve get file_size logic ([`da89ed6`](https://github.com/AOSC-Dev/oma/commit/da89ed6a92c0560ae0bce410a7a382adc90d3eef))
</details>

## v0.30.3 (2023-04-30)

<csr-id-ecaa4ba7fb203e6ef62e8053aeba97183a6aa09f/>
<csr-id-54ed3e3c6f90ac61e95b65bc55a13bc05a081c3a/>
<csr-id-f243e54d793662af8cf1fa621f4b24365274de61/>
<csr-id-c621f3af74c784129c084d0f926d1a273abed379/>

### Chore

 - <csr-id-ecaa4ba7fb203e6ef62e8053aeba97183a6aa09f/> update all deps
 - <csr-id-54ed3e3c6f90ac61e95b65bc55a13bc05a081c3a/> remove useless test

### Refactor

 - <csr-id-f243e54d793662af8cf1fa621f4b24365274de61/> improve resume download logic

### Style

 - <csr-id-c621f3af74c784129c084d0f926d1a273abed379/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.30.3 ([`4023612`](https://github.com/AOSC-Dev/oma/commit/4023612dc4f471dffc0d80192e0e31edb019366f))
    - Update all deps ([`ecaa4ba`](https://github.com/AOSC-Dev/oma/commit/ecaa4ba7fb203e6ef62e8053aeba97183a6aa09f))
    - Remove useless test ([`54ed3e3`](https://github.com/AOSC-Dev/oma/commit/54ed3e3c6f90ac61e95b65bc55a13bc05a081c3a))
    - Improve resume download logic ([`f243e54`](https://github.com/AOSC-Dev/oma/commit/f243e54d793662af8cf1fa621f4b24365274de61))
    - Use cargo clippy to lint code ([`c621f3a`](https://github.com/AOSC-Dev/oma/commit/c621f3af74c784129c084d0f926d1a273abed379))
</details>

## v0.30.2 (2023-04-29)

<csr-id-4af6ab474a6f3ea5a68a934f637420fd171b94a7/>

### Chore

 - <csr-id-4af6ab474a6f3ea5a68a934f637420fd171b94a7/> update all deps

### Bug Fixes

 - <csr-id-30f20fd8f8183d1f3d5e307dbe2b2b78de3c350c/> Revert retry 2 times start dpkg-force-all mode
 - <csr-id-38262ff93c8742e188f41aa0200cf29c338cb2b5/> Download again when checksum does not match and returns 416

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.30.2 ([`b6a188d`](https://github.com/AOSC-Dev/oma/commit/b6a188df8c045e402d81319f52af8e69555dbd82))
    - Revert retry 2 times start dpkg-force-all mode ([`30f20fd`](https://github.com/AOSC-Dev/oma/commit/30f20fd8f8183d1f3d5e307dbe2b2b78de3c350c))
    - Download again when checksum does not match and returns 416 ([`38262ff`](https://github.com/AOSC-Dev/oma/commit/38262ff93c8742e188f41aa0200cf29c338cb2b5))
    - Update all deps ([`4af6ab4`](https://github.com/AOSC-Dev/oma/commit/4af6ab474a6f3ea5a68a934f637420fd171b94a7))
</details>

## v0.30.1 (2023-04-29)

<csr-id-22ce5aae9446e5eae07d7d8fc41dc395ce6a3a5e/>
<csr-id-fed5365c2e4e84f8a1ebdb1d82d63425ac7f7aa5/>

### Chore

 - <csr-id-22ce5aae9446e5eae07d7d8fc41dc395ce6a3a5e/> remove uselses test

### Bug Fixes

 - <csr-id-dca23ff2178e3bd7eb4885bce1495d9f0f3e84d0/> Reson => Reason
 - <csr-id-e3784238b362c1a88f512b72f42188cad76e50a6/> Add missing ! to fix wrong logic in scan_closed_topic

### Refactor

 - <csr-id-fed5365c2e4e84f8a1ebdb1d82d63425ac7f7aa5/> improve auto close topic

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 2 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.30.1 ([`832d4b0`](https://github.com/AOSC-Dev/oma/commit/832d4b0af75f32defbf4cbfb90d3e8f4809acb09))
    - Remove uselses test ([`22ce5aa`](https://github.com/AOSC-Dev/oma/commit/22ce5aae9446e5eae07d7d8fc41dc395ce6a3a5e))
    - Reson => Reason ([`dca23ff`](https://github.com/AOSC-Dev/oma/commit/dca23ff2178e3bd7eb4885bce1495d9f0f3e84d0))
    - Add missing ! to fix wrong logic in scan_closed_topic ([`e378423`](https://github.com/AOSC-Dev/oma/commit/e3784238b362c1a88f512b72f42188cad76e50a6))
    - Improve auto close topic ([`fed5365`](https://github.com/AOSC-Dev/oma/commit/fed5365c2e4e84f8a1ebdb1d82d63425ac7f7aa5))
</details>

## v0.30.0 (2023-04-27)

<csr-id-34c4013fbb3fd588355d66186291140b87c71195/>
<csr-id-7a208b756c73cd6203dabeab7dcbac6f9adc7d97/>
<csr-id-df11fbc07c64c57e263c130bcd4894eda1ac4fe4/>
<csr-id-df4fdf600828e6dc1256438c11544d4718e44431/>
<csr-id-67f20466321260f59b5769dec969ffd1888c7ced/>

### Chore

 - <csr-id-34c4013fbb3fd588355d66186291140b87c71195/> update all deps

### New Features

 - <csr-id-8c8f5e97da8b1785637decc19c11b9dca73df0b8/> Drop inquire searcher curosr
 - <csr-id-324429279ff07a8e21e0656c3bef5165fe193036/> Drop inquire searcher
 - <csr-id-70190fcae1c25683c6994749c7674dff4189a3c5/> Update_db if url is closed topic, remove url from apt sources.list
 - <csr-id-46d340a2dea03cad8d9dac39ef75cdd0431bf92e/> Add topics feature
   - Also fix only exist in local packages can not get filename record issue

### Bug Fixes

 - <csr-id-55d15ef692f3781360552bfe8f0ecb7ee09b2063/> Do not save file with download failed; return error if 404 not found url is not closed topic
 - <csr-id-9b424b81b25c48ae12ab3158f713ef781afd83fe/> Don't let the progress spinner thread dead loop if the download has errors
 - <csr-id-afeae4adaa65585d307b7564b72d9a8c44319b6c/> If package newest version in other enabled topics, downgrade to stable version

### Refactor

 - <csr-id-7a208b756c73cd6203dabeab7dcbac6f9adc7d97/> use spawn_blocking to execute rm_topic method

### Style

 - <csr-id-df11fbc07c64c57e263c130bcd4894eda1ac4fe4/> use cargo-fmt to lint code
 - <csr-id-df4fdf600828e6dc1256438c11544d4718e44431/> use cargo clippy to lint code again
 - <csr-id-67f20466321260f59b5769dec969ffd1888c7ced/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 4 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.30.0 ([`a556238`](https://github.com/AOSC-Dev/oma/commit/a556238b6367af5a767e618feee54504a5249949))
    - Use cargo-fmt to lint code ([`df11fbc`](https://github.com/AOSC-Dev/oma/commit/df11fbc07c64c57e263c130bcd4894eda1ac4fe4))
    - Do not save file with download failed; return error if 404 not found url is not closed topic ([`55d15ef`](https://github.com/AOSC-Dev/oma/commit/55d15ef692f3781360552bfe8f0ecb7ee09b2063))
    - Don't let the progress spinner thread dead loop if the download has errors ([`9b424b8`](https://github.com/AOSC-Dev/oma/commit/9b424b81b25c48ae12ab3158f713ef781afd83fe))
    - If package newest version in other enabled topics, downgrade to stable version ([`afeae4a`](https://github.com/AOSC-Dev/oma/commit/afeae4adaa65585d307b7564b72d9a8c44319b6c))
    - Drop inquire searcher curosr ([`8c8f5e9`](https://github.com/AOSC-Dev/oma/commit/8c8f5e97da8b1785637decc19c11b9dca73df0b8))
    - Drop inquire searcher ([`3244292`](https://github.com/AOSC-Dev/oma/commit/324429279ff07a8e21e0656c3bef5165fe193036))
    - Use spawn_blocking to execute rm_topic method ([`7a208b7`](https://github.com/AOSC-Dev/oma/commit/7a208b756c73cd6203dabeab7dcbac6f9adc7d97))
    - Update_db if url is closed topic, remove url from apt sources.list ([`70190fc`](https://github.com/AOSC-Dev/oma/commit/70190fcae1c25683c6994749c7674dff4189a3c5))
    - Use cargo clippy to lint code again ([`df4fdf6`](https://github.com/AOSC-Dev/oma/commit/df4fdf600828e6dc1256438c11544d4718e44431))
    - Use cargo clippy to lint code ([`67f2046`](https://github.com/AOSC-Dev/oma/commit/67f20466321260f59b5769dec969ffd1888c7ced))
    - Update all deps ([`34c4013`](https://github.com/AOSC-Dev/oma/commit/34c4013fbb3fd588355d66186291140b87c71195))
    - Add topics feature ([`46d340a`](https://github.com/AOSC-Dev/oma/commit/46d340a2dea03cad8d9dac39ef75cdd0431bf92e))
</details>

## v0.29.1 (2023-04-23)

<csr-id-a29f7954eba270fc18e942eb5fca32c5458e6d0f/>
<csr-id-92b729efa537780bffa1cfc8e5a8e057bafa83c7/>
<csr-id-fe7979ca1655af0df2ae7d11325ff1fb19bbd6df/>
<csr-id-000570bb20ad22ad90515791b3202ed038548f36/>

### Chore

 - <csr-id-a29f7954eba270fc18e942eb5fca32c5458e6d0f/> update all deps

### New Features

<csr-id-2015c43a1a348dd847b3028daea869bd5acda589/>

 - <csr-id-aef283d66c4ac9be766762b54d60f97c16305693/> Check InRelaese date and vaild-until
 - <csr-id-25badde57954f9049c371f9f045627fdf6955a4e/> Improve clap oma style theme ...
   - Fix a typo

### Bug Fixes

 - <csr-id-47cdbc4ce8d5673ab60cd10490d999f185731e3c/> Download doesn exist file will return error
 - <csr-id-9f5da29cc0483cad628fcbeb881ca340f7637d91/> Not allow_resume file wrong reset length

### Refactor

 - <csr-id-92b729efa537780bffa1cfc8e5a8e057bafa83c7/> improve download method logic

### Style

 - <csr-id-fe7979ca1655af0df2ae7d11325ff1fb19bbd6df/> use cargo clippy to lint code
 - <csr-id-000570bb20ad22ad90515791b3202ed038548f36/> remove useless refrence flag

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.29.1 ([`f352de2`](https://github.com/AOSC-Dev/oma/commit/f352de2d580318f67f00d015d83d76ccea34ecfb))
    - Update all deps ([`a29f795`](https://github.com/AOSC-Dev/oma/commit/a29f7954eba270fc18e942eb5fca32c5458e6d0f))
    - Use cargo clippy to lint code ([`fe7979c`](https://github.com/AOSC-Dev/oma/commit/fe7979ca1655af0df2ae7d11325ff1fb19bbd6df))
    - Improve download method logic ([`92b729e`](https://github.com/AOSC-Dev/oma/commit/92b729efa537780bffa1cfc8e5a8e057bafa83c7))
    - Download doesn exist file will return error ([`47cdbc4`](https://github.com/AOSC-Dev/oma/commit/47cdbc4ce8d5673ab60cd10490d999f185731e3c))
    - Not allow_resume file wrong reset length ([`9f5da29`](https://github.com/AOSC-Dev/oma/commit/9f5da29cc0483cad628fcbeb881ca340f7637d91))
    - Remove useless refrence flag ([`000570b`](https://github.com/AOSC-Dev/oma/commit/000570bb20ad22ad90515791b3202ed038548f36))
    - Check InRelaese date and vaild-until ([`aef283d`](https://github.com/AOSC-Dev/oma/commit/aef283d66c4ac9be766762b54d60f97c16305693))
    - Improve clap oma style theme ... ([`25badde`](https://github.com/AOSC-Dev/oma/commit/25badde57954f9049c371f9f045627fdf6955a4e))
    - Set clap help header and usage color as bright blue ([`2015c43`](https://github.com/AOSC-Dev/oma/commit/2015c43a1a348dd847b3028daea869bd5acda589))
</details>

## v0.29.0 (2023-04-19)

<csr-id-8507728ae35419300d29fecea98f88ffca95dec8/>

### New Features

 - <csr-id-1fd75d92d95db04209a3216a00bb02c6b1d99de4/> Sort oma search order, UPGRADE > AVAIL > INSTALLED

### Refactor

 - <csr-id-8507728ae35419300d29fecea98f88ffca95dec8/> use trait to get prefix string

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.29.0 ([`f0deeca`](https://github.com/AOSC-Dev/oma/commit/f0deeca35c5453add4a8d9619c49eef13a4a2117))
    - Use trait to get prefix string ([`8507728`](https://github.com/AOSC-Dev/oma/commit/8507728ae35419300d29fecea98f88ffca95dec8))
    - Sort oma search order, UPGRADE > AVAIL > INSTALLED ([`1fd75d9`](https://github.com/AOSC-Dev/oma/commit/1fd75d92d95db04209a3216a00bb02c6b1d99de4))
</details>

## v0.28.2 (2023-04-19)

<csr-id-55acee4afd11e703deb204fadda2c0e9bb179aee/>

### New Features

 - <csr-id-71e0267e96cf3fd1bca19930ee4cb1f066dd2ea2/> Command-not-found do not display progress spinner

### Style

 - <csr-id-55acee4afd11e703deb204fadda2c0e9bb179aee/> lint code use myself brain and cargo-clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.28.2 ([`a64057b`](https://github.com/AOSC-Dev/oma/commit/a64057b13ee959091351f4b70eb8428b2dc71543))
    - Lint code use myself brain and cargo-clippy ([`55acee4`](https://github.com/AOSC-Dev/oma/commit/55acee4afd11e703deb204fadda2c0e9bb179aee))
    - Command-not-found do not display progress spinner ([`71e0267`](https://github.com/AOSC-Dev/oma/commit/71e0267e96cf3fd1bca19930ee4cb1f066dd2ea2))
</details>

## v0.28.1 (2023-04-19)

<csr-id-cf50ed1f0cc0eb96e17c387a7f1bec80deb65371/>

### Chore

 - <csr-id-cf50ed1f0cc0eb96e17c387a7f1bec80deb65371/> update all deps

### Bug Fixes

 - <csr-id-f140377d12c0f06e533f77485f4b82889d434c24/> Fix-broken no need to do anything useless to run apt_install method

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.28.1 ([`850560b`](https://github.com/AOSC-Dev/oma/commit/850560bdc9a091f5a320876fd0264b54dcb042ad))
    - Update all deps ([`cf50ed1`](https://github.com/AOSC-Dev/oma/commit/cf50ed1f0cc0eb96e17c387a7f1bec80deb65371))
    - Fix-broken no need to do anything useless to run apt_install method ([`f140377`](https://github.com/AOSC-Dev/oma/commit/f140377d12c0f06e533f77485f4b82889d434c24))
</details>

## v0.28.0 (2023-04-18)

<csr-id-5fb2dc434dae4b7d4856b044f62b404c22560d69/>
<csr-id-7ac3114e8503b5e5127956bc8992012a59a9e30b/>

### Chore

 - <csr-id-5fb2dc434dae4b7d4856b044f62b404c22560d69/> update h2 to v0.3.18
 - <csr-id-7ac3114e8503b5e5127956bc8992012a59a9e30b/> update all deps

### Documentation

 - <csr-id-d8a38a37cd1b9afc1742d40e11a7e7bcd842c1c5/> Afixcurrent_state comment a typo
 - <csr-id-9b929a8ae679ea294ec66b74afcf3d5f18287380/> Add current_state comment

### New Features

 - <csr-id-8d080aa9d17f798ede44aaf6d2458ea3b1559de6/> Check system needs fix status in oma {upgrade,fix-brokeen}
 - <csr-id-d277409c1c7edd8a24b4ca7f4ca9ad82bfdabd7d/> Check system needs fix status
 - <csr-id-f53dbb2dcf71972e25d8b3d3d8fc6cefd34ffcd5/> Oma download do not display downloaded specific pkgs

### Bug Fixes

 - <csr-id-d7473c4b5f7d15d71cf19c18c7284340ae19e4b9/> Oma download path maybe return error

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.28.0 ([`e69fcb6`](https://github.com/AOSC-Dev/oma/commit/e69fcb677009026984ba2fb67eb5708328fd8bea))
    - Update h2 to v0.3.18 ([`5fb2dc4`](https://github.com/AOSC-Dev/oma/commit/5fb2dc434dae4b7d4856b044f62b404c22560d69))
    - Check system needs fix status in oma {upgrade,fix-brokeen} ([`8d080aa`](https://github.com/AOSC-Dev/oma/commit/8d080aa9d17f798ede44aaf6d2458ea3b1559de6))
    - Afixcurrent_state comment a typo ([`d8a38a3`](https://github.com/AOSC-Dev/oma/commit/d8a38a37cd1b9afc1742d40e11a7e7bcd842c1c5))
    - Add current_state comment ([`9b929a8`](https://github.com/AOSC-Dev/oma/commit/9b929a8ae679ea294ec66b74afcf3d5f18287380))
    - Check system needs fix status ([`d277409`](https://github.com/AOSC-Dev/oma/commit/d277409c1c7edd8a24b4ca7f4ca9ad82bfdabd7d))
    - Oma download path maybe return error ([`d7473c4`](https://github.com/AOSC-Dev/oma/commit/d7473c4b5f7d15d71cf19c18c7284340ae19e4b9))
    - Oma download do not display downloaded specific pkgs ([`f53dbb2`](https://github.com/AOSC-Dev/oma/commit/f53dbb2dcf71972e25d8b3d3d8fc6cefd34ffcd5))
    - Update all deps ([`7ac3114`](https://github.com/AOSC-Dev/oma/commit/7ac3114e8503b5e5127956bc8992012a59a9e30b))
</details>

## v0.27.0 (2023-04-17)

<csr-id-2883bb6ce4412d1672a1cc0d07a43b83adb37d92/>
<csr-id-c2ce36d0d83cf107430bd41cc6eaadbab96c535e/>

### Chore

 - <csr-id-2883bb6ce4412d1672a1cc0d07a43b83adb37d92/> update all deps

### New Features

 - <csr-id-5fa8c3c2e370387cc4476f542b4470e1b93d46e5/> Allow resume exist download package progress
 - <csr-id-ce8f8ad24ec83ec92444b324384c9f1ae4cde65e/> Fetch un-compress database file in mips64r6el arch
   ... In mips64r6el (preformance like 486) machine, decompressing the files is a huge ordeal, so to protect our boston, only the uncompressed database files are downloaded here

### Bug Fixes

 - <csr-id-fe808ba0545c0daa6140fadd97b6aed4ac68a5f0/> Download failed reset wrong progress bar status

### Style

 - <csr-id-c2ce36d0d83cf107430bd41cc6eaadbab96c535e/> use cargo-clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 3 calendar days.
 - 4 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.27.0 ([`6ca744e`](https://github.com/AOSC-Dev/oma/commit/6ca744e471b9b110802eb8b0445f0814fbfdb3c8))
    - Update all deps ([`2883bb6`](https://github.com/AOSC-Dev/oma/commit/2883bb6ce4412d1672a1cc0d07a43b83adb37d92))
    - Use cargo-clippy to lint code ([`c2ce36d`](https://github.com/AOSC-Dev/oma/commit/c2ce36d0d83cf107430bd41cc6eaadbab96c535e))
    - Download failed reset wrong progress bar status ([`fe808ba`](https://github.com/AOSC-Dev/oma/commit/fe808ba0545c0daa6140fadd97b6aed4ac68a5f0))
    - Allow resume exist download package progress ([`5fa8c3c`](https://github.com/AOSC-Dev/oma/commit/5fa8c3c2e370387cc4476f542b4470e1b93d46e5))
    - Fetch un-compress database file in mips64r6el arch ([`ce8f8ad`](https://github.com/AOSC-Dev/oma/commit/ce8f8ad24ec83ec92444b324384c9f1ae4cde65e))
</details>

## v0.26.0 (2023-04-13)

<csr-id-151f5495c66feaf24d223d46c2083c85ffd3cd65/>
<csr-id-5dbd68f501cb6a5b4ce1c588ef7bc915a17224b0/>
<csr-id-0491219d475d705035f8c7ed26fc51a92408a8fa/>

### Chore

 - <csr-id-151f5495c66feaf24d223d46c2083c85ffd3cd65/> update all deps

### New Features

 - <csr-id-b011846954e879660edb92cae9279d7fb7742ea3/> Add upgradable check unmet dependency

### Bug Fixes

 - <csr-id-9160c245c3707ec42317b2fc5ed1ff544c86fd02/> If get ARCH run dpkg to failed, error missing context (2)
 - <csr-id-c28196a4ec2c6b2163cecdfa4a4b16c05567228d/> If get ARCH run dpkg to failed, error missing context
 - <csr-id-14bdbd54600312b2a5e1427ab4180487a599b8a8/> If can not get ARCH, error missing context

### Refactor

 - <csr-id-5dbd68f501cb6a5b4ce1c588ef7bc915a17224b0/> use dpkg --print-architecture to get arch name
   ... This commit we can without --features mipsr6 to build oma and use

### Style

 - <csr-id-0491219d475d705035f8c7ed26fc51a92408a8fa/> use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.26.0 ([`1ac82ed`](https://github.com/AOSC-Dev/oma/commit/1ac82ede5fd89c57344059a9927b176d4cf41280))
    - Update all deps ([`151f549`](https://github.com/AOSC-Dev/oma/commit/151f5495c66feaf24d223d46c2083c85ffd3cd65))
    - Use cargo-fmt to format code ([`0491219`](https://github.com/AOSC-Dev/oma/commit/0491219d475d705035f8c7ed26fc51a92408a8fa))
    - Add upgradable check unmet dependency ([`b011846`](https://github.com/AOSC-Dev/oma/commit/b011846954e879660edb92cae9279d7fb7742ea3))
    - If get ARCH run dpkg to failed, error missing context (2) ([`9160c24`](https://github.com/AOSC-Dev/oma/commit/9160c245c3707ec42317b2fc5ed1ff544c86fd02))
    - If get ARCH run dpkg to failed, error missing context ([`c28196a`](https://github.com/AOSC-Dev/oma/commit/c28196a4ec2c6b2163cecdfa4a4b16c05567228d))
    - If can not get ARCH, error missing context ([`14bdbd5`](https://github.com/AOSC-Dev/oma/commit/14bdbd54600312b2a5e1427ab4180487a599b8a8))
    - Use dpkg --print-architecture to get arch name ([`5dbd68f`](https://github.com/AOSC-Dev/oma/commit/5dbd68f501cb6a5b4ce1c588ef7bc915a17224b0))
</details>

## v0.25.0 (2023-04-11)

<csr-id-59b43f41c1c3b0cf46a76abe5aa3fa371fed41dd/>
<csr-id-696cfff46b2836cc1581cb92298f0159e4801b33/>

### Chore

 - <csr-id-59b43f41c1c3b0cf46a76abe5aa3fa371fed41dd/> update all deps

### New Features

 - <csr-id-9e0fc75bcf605e68a40ef2217781080fd8a46682/> Support mips64r6el arch
   ... Need use --features mipsr6 to use
 - <csr-id-ce1a68bc427e0c17e1d88803527ecea62cf64e48/> Support oma -v to display oma version

### Bug Fixes

 - <csr-id-659140f65b9b49330ad34c0a5190d5f89ed6fd27/> Repeated version flag to run build.rs script to failed
 - <csr-id-fea7d1ca3fa38be845bcabfa68fccd4598440846/> Missing --version (-v, -V) help message

### Other

 - <csr-id-696cfff46b2836cc1581cb92298f0159e4801b33/> capitalise first letter of project description

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 1 day passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.25.0 ([`e94b869`](https://github.com/AOSC-Dev/oma/commit/e94b86904adec94380a81601e7ddce10b69a02b7))
    - Update all deps ([`59b43f4`](https://github.com/AOSC-Dev/oma/commit/59b43f41c1c3b0cf46a76abe5aa3fa371fed41dd))
    - Support mips64r6el arch ([`9e0fc75`](https://github.com/AOSC-Dev/oma/commit/9e0fc75bcf605e68a40ef2217781080fd8a46682))
    - Repeated version flag to run build.rs script to failed ([`659140f`](https://github.com/AOSC-Dev/oma/commit/659140f65b9b49330ad34c0a5190d5f89ed6fd27))
    - Capitalise first letter of project description ([`696cfff`](https://github.com/AOSC-Dev/oma/commit/696cfff46b2836cc1581cb92298f0159e4801b33))
    - Missing --version (-v, -V) help message ([`fea7d1c`](https://github.com/AOSC-Dev/oma/commit/fea7d1ca3fa38be845bcabfa68fccd4598440846))
    - Support oma -v to display oma version ([`ce1a68b`](https://github.com/AOSC-Dev/oma/commit/ce1a68bc427e0c17e1d88803527ecea62cf64e48))
</details>

## v0.24.3 (2023-04-09)

### Bug Fixes

 - <csr-id-f2329bab17a19055a959545d00b787fd5eb1fa19/> Can not set logger with --debug flag
   - Also set --debug as global flag

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.24.3 ([`97907fa`](https://github.com/AOSC-Dev/oma/commit/97907fa6dacf186e9b19c620f0b64ba2cc6712e4))
    - Can not set logger with --debug flag ([`f2329ba`](https://github.com/AOSC-Dev/oma/commit/f2329bab17a19055a959545d00b787fd5eb1fa19))
</details>

## v0.24.2 (2023-04-09)

<csr-id-271851401fd918dcdbec5ca46e9173e87cb5170a/>
<csr-id-693b3a93c6e1535b0c85c78e7bdc06ddf66a7c74/>

### New Features

 - <csr-id-b73ae60c92a8b10f758cd07d11a1da85593e84c8/> Improve command-not-found output

### Bug Fixes

 - <csr-id-fa13097a5f034a1c0f2477d245123b7116c35c26/> Pick can not get no_upgrade argument to panic
 - <csr-id-f52fcffd589a9d20d018cd243f3813eebae1dba0/> Provides search absolute path can't get any result

### Style

 - <csr-id-271851401fd918dcdbec5ca46e9173e87cb5170a/> use cargo-fmt to format code
 - <csr-id-693b3a93c6e1535b0c85c78e7bdc06ddf66a7c74/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.24.2 ([`aaa04d5`](https://github.com/AOSC-Dev/oma/commit/aaa04d506b093a243d0136c9957953c08319b49c))
    - Use cargo-fmt to format code ([`2718514`](https://github.com/AOSC-Dev/oma/commit/271851401fd918dcdbec5ca46e9173e87cb5170a))
    - Pick can not get no_upgrade argument to panic ([`fa13097`](https://github.com/AOSC-Dev/oma/commit/fa13097a5f034a1c0f2477d245123b7116c35c26))
    - Use cargo clippy to lint code ([`693b3a9`](https://github.com/AOSC-Dev/oma/commit/693b3a93c6e1535b0c85c78e7bdc06ddf66a7c74))
    - Improve command-not-found output ([`b73ae60`](https://github.com/AOSC-Dev/oma/commit/b73ae60c92a8b10f758cd07d11a1da85593e84c8))
    - Provides search absolute path can't get any result ([`f52fcff`](https://github.com/AOSC-Dev/oma/commit/f52fcffd589a9d20d018cd243f3813eebae1dba0))
</details>

## v0.24.1 (2023-04-09)

<csr-id-e70c3aef3d487593494fba25e33483ac7121477c/>
<csr-id-08dddaf3882414c4c9b24484b5d36f7d99d48965/>
<csr-id-85deb9fd8b562fffa5dec0e762eb29a559639470/>

### Chore

 - <csr-id-e70c3aef3d487593494fba25e33483ac7121477c/> update dep crossbeam-channel to 0.5.8

### Documentation

 - <csr-id-86b85ddc51ef22d00ed27ad5854ec4ca4fea7a0e/> Update README

### Bug Fixes

 - <csr-id-1781711818be211e532bfd8a1094559361e26d96/> No additional version info tips
 - <csr-id-33f6a8e8cc3fd4ffed16c9ebc5e48343bdacb67b/> Pick no_fix_broekn wrong argument name to panic
 - <csr-id-02de21592e80af57c1aea8013b4f600cc2370f88/> Reinstall does not in repo version to panic
 - <csr-id-8a61940b26a790427a54b5dee04c16dadf310e1c/> Oma dep output wrong grammar

### Refactor

 - <csr-id-08dddaf3882414c4c9b24484b5d36f7d99d48965/> improve list method code style

### Style

 - <csr-id-85deb9fd8b562fffa5dec0e762eb29a559639470/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.24.1 ([`4b8ce0d`](https://github.com/AOSC-Dev/oma/commit/4b8ce0d9b6d161898e7d11d2bab17247967ce029))
    - Use cargo clippy to lint code ([`85deb9f`](https://github.com/AOSC-Dev/oma/commit/85deb9fd8b562fffa5dec0e762eb29a559639470))
    - Update dep crossbeam-channel to 0.5.8 ([`e70c3ae`](https://github.com/AOSC-Dev/oma/commit/e70c3aef3d487593494fba25e33483ac7121477c))
    - No additional version info tips ([`1781711`](https://github.com/AOSC-Dev/oma/commit/1781711818be211e532bfd8a1094559361e26d96))
    - Pick no_fix_broekn wrong argument name to panic ([`33f6a8e`](https://github.com/AOSC-Dev/oma/commit/33f6a8e8cc3fd4ffed16c9ebc5e48343bdacb67b))
    - Improve list method code style ([`08dddaf`](https://github.com/AOSC-Dev/oma/commit/08dddaf3882414c4c9b24484b5d36f7d99d48965))
    - Reinstall does not in repo version to panic ([`02de215`](https://github.com/AOSC-Dev/oma/commit/02de21592e80af57c1aea8013b4f600cc2370f88))
    - Oma dep output wrong grammar ([`8a61940`](https://github.com/AOSC-Dev/oma/commit/8a61940b26a790427a54b5dee04c16dadf310e1c))
    - Update README ([`86b85dd`](https://github.com/AOSC-Dev/oma/commit/86b85ddc51ef22d00ed27ad5854ec4ca4fea7a0e))
</details>

## v0.24.0 (2023-04-08)

<csr-id-66fb496cb5ea21f8288e04987c371bd1d2cbee90/>
<csr-id-9a7e556094c415cbadee46ca3a9a0c26d5c947d5/>

### Chore

 - <csr-id-66fb496cb5ea21f8288e04987c371bd1d2cbee90/> update all deps

### Documentation

 - <csr-id-fbe79f756566cbb54d9f8f20522e8be2cfd1b846/> Improve help and man document
   - feat: --no-upgrade => --no-refresh and more argument name adjust

### Bug Fixes

 - <csr-id-0b937fee9e9740f1f98537a31652ab9504d98a3c/> Fix wrong oma list info display
   `oma list a b` will not display additional version info
 - <csr-id-c99a6e63a0094101bbc3302d3cd367262e42ed1b/> Set search arg name as pattern
 - <csr-id-06ea3cb5ba208625f9b4ac503dc231a6604a3341/> Fix oma show needs packages argument
 - <csr-id-6e5cd72e262c3cfc68776e1913d341e9e890e720/> Fix without dry-run argument subcommand run
 - <csr-id-e55554b3a134d2315c42d3d2fb52270b2d26bd2e/> Use PossibleValues to fix oma-mark man document

### Refactor

 - <csr-id-9a7e556094c415cbadee46ca3a9a0c26d5c947d5/> improve setup dry_tun flag logic

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 2 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.24.0 ([`14a3d0d`](https://github.com/AOSC-Dev/oma/commit/14a3d0d6a18d4dfe30a524bad2392c351de397bf))
    - Update all deps ([`66fb496`](https://github.com/AOSC-Dev/oma/commit/66fb496cb5ea21f8288e04987c371bd1d2cbee90))
    - Fix wrong oma list info display ([`0b937fe`](https://github.com/AOSC-Dev/oma/commit/0b937fee9e9740f1f98537a31652ab9504d98a3c))
    - Set search arg name as pattern ([`c99a6e6`](https://github.com/AOSC-Dev/oma/commit/c99a6e63a0094101bbc3302d3cd367262e42ed1b))
    - Fix oma show needs packages argument ([`06ea3cb`](https://github.com/AOSC-Dev/oma/commit/06ea3cb5ba208625f9b4ac503dc231a6604a3341))
    - Fix without dry-run argument subcommand run ([`6e5cd72`](https://github.com/AOSC-Dev/oma/commit/6e5cd72e262c3cfc68776e1913d341e9e890e720))
    - Improve setup dry_tun flag logic ([`9a7e556`](https://github.com/AOSC-Dev/oma/commit/9a7e556094c415cbadee46ca3a9a0c26d5c947d5))
    - Use PossibleValues to fix oma-mark man document ([`e55554b`](https://github.com/AOSC-Dev/oma/commit/e55554b3a134d2315c42d3d2fb52270b2d26bd2e))
    - Improve help and man document ([`fbe79f7`](https://github.com/AOSC-Dev/oma/commit/fbe79f756566cbb54d9f8f20522e8be2cfd1b846))
</details>

## v0.23.0 (2023-04-06)

<csr-id-d5154552b077bae8744e9b14eae5aff91437bdc0/>
<csr-id-fdf7f624eabb13f0493de60a90c3c393700e5c62/>
<csr-id-119f0594d44d938515ac82c20ae1a19d9a5499af/>
<csr-id-82e758268836be0751125b6c7bcf629eea89192f/>
<csr-id-1fef5a496acfa29f413bd4ecdcee4fd9ce7f44da/>

### Chore

 - <csr-id-d5154552b077bae8744e9b14eae5aff91437bdc0/> update serde-yaml to 0.9.20

### New Features

 - <csr-id-ddbe2d90c9ae78514e8482eff10e2ce7f50cf021/> Oma pick do not autoremove by default
 - <csr-id-11b5c8ce664108f6f35b64e26832629fe29aa9b7/> Oma install do not autoremove by default
 - <csr-id-fffcd13eacea342696a3dd50b53c5ee4ece7d11a/> Add query packages database spinner
 - <csr-id-abd0d7481ce081623e681cc4d9dcd4a5a8ba2ad3/> Add --no-autoremove argument for oma {install,upgrade,remove,pick}
 - <csr-id-b826d028295b12e3491aa0d8d59b6cdb9f047a32/> Add cache.get_archives spinner
 - <csr-id-4355b06227b132e627294b75346906bd8575c6bc/> --debug argument now can run without dry-run mode

### Bug Fixes

 - <csr-id-17c1d7f023eba71d500660963ec23940f9a148e2/> Fix query database zombie progress bar
 - <csr-id-e568b01446a4f36d53deafb546ecaf104784cb0a/> Fix oma pick no_autoremove arg requires
 - <csr-id-df3b53b32ef4081292c6fae0de46b1ccdb0ad0ec/> Fix refresh database file exist global bar progress
 - <csr-id-a38a6840cb0628b595ce9a380754fab0d5d71eef/> Fix global bar progress percent color

### Refactor

 - <csr-id-fdf7f624eabb13f0493de60a90c3c393700e5c62/> improve pending ui detail capitalize logic
 - <csr-id-119f0594d44d938515ac82c20ae1a19d9a5499af/> set Multiprogress Bar as lazy var

### Style

 - <csr-id-82e758268836be0751125b6c7bcf629eea89192f/> use cargo-clippy to lint code
 - <csr-id-1fef5a496acfa29f413bd4ecdcee4fd9ce7f44da/> run cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release.
 - 15 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.23.0 ([`0a5bdf8`](https://github.com/AOSC-Dev/oma/commit/0a5bdf8a301278b6389547a8c79d9c752918a3d4))
    - Update serde-yaml to 0.9.20 ([`d515455`](https://github.com/AOSC-Dev/oma/commit/d5154552b077bae8744e9b14eae5aff91437bdc0))
    - Improve pending ui detail capitalize logic ([`fdf7f62`](https://github.com/AOSC-Dev/oma/commit/fdf7f624eabb13f0493de60a90c3c393700e5c62))
    - Use cargo-clippy to lint code ([`82e7582`](https://github.com/AOSC-Dev/oma/commit/82e758268836be0751125b6c7bcf629eea89192f))
    - Oma pick do not autoremove by default ([`ddbe2d9`](https://github.com/AOSC-Dev/oma/commit/ddbe2d90c9ae78514e8482eff10e2ce7f50cf021))
    - Oma install do not autoremove by default ([`11b5c8c`](https://github.com/AOSC-Dev/oma/commit/11b5c8ce664108f6f35b64e26832629fe29aa9b7))
    - Fix query database zombie progress bar ([`17c1d7f`](https://github.com/AOSC-Dev/oma/commit/17c1d7f023eba71d500660963ec23940f9a148e2))
    - Set Multiprogress Bar as lazy var ([`119f059`](https://github.com/AOSC-Dev/oma/commit/119f0594d44d938515ac82c20ae1a19d9a5499af))
    - Add query packages database spinner ([`fffcd13`](https://github.com/AOSC-Dev/oma/commit/fffcd13eacea342696a3dd50b53c5ee4ece7d11a))
    - Run cargo clippy to lint code ([`1fef5a4`](https://github.com/AOSC-Dev/oma/commit/1fef5a496acfa29f413bd4ecdcee4fd9ce7f44da))
    - Fix oma pick no_autoremove arg requires ([`e568b01`](https://github.com/AOSC-Dev/oma/commit/e568b01446a4f36d53deafb546ecaf104784cb0a))
    - Fix refresh database file exist global bar progress ([`df3b53b`](https://github.com/AOSC-Dev/oma/commit/df3b53b32ef4081292c6fae0de46b1ccdb0ad0ec))
    - Add --no-autoremove argument for oma {install,upgrade,remove,pick} ([`abd0d74`](https://github.com/AOSC-Dev/oma/commit/abd0d7481ce081623e681cc4d9dcd4a5a8ba2ad3))
    - Add cache.get_archives spinner ([`b826d02`](https://github.com/AOSC-Dev/oma/commit/b826d028295b12e3491aa0d8d59b6cdb9f047a32))
    - Fix global bar progress percent color ([`a38a684`](https://github.com/AOSC-Dev/oma/commit/a38a6840cb0628b595ce9a380754fab0d5d71eef))
    - --debug argument now can run without dry-run mode ([`4355b06`](https://github.com/AOSC-Dev/oma/commit/4355b06227b132e627294b75346906bd8575c6bc))
</details>

## v0.22.0 (2023-04-05)

<csr-id-a87a40d79c10a7acbf2f76ba6e4fa413e206ad46/>
<csr-id-46b75d978dd0e8fe93847086f1040b3a22a1603c/>
<csr-id-05baa26ff7c75c4c103ad5f1b78f53f8b571c52c/>

### Chore

 - <csr-id-a87a40d79c10a7acbf2f76ba6e4fa413e206ad46/> update all deps

### New Features

 - <csr-id-b44035d76b13675b85d6a2c648f6b6ab76eea448/> Error output message adjust
 - <csr-id-3bb8b317af8eaacd7a03eb5a0939a15f3449b37e/> If needs run dpkg --configure -a, run it
 - <csr-id-7ae0faca3b79ecbbbf50f4f28e369bbeca4d13fb/> Build all subcommand man

### Bug Fixes

 - <csr-id-4df750e8587145d44bca5f112aace690e08c6107/> Fix autoremove/non-autoremove pkg pending ui wrong detail

### Refactor

 - <csr-id-46b75d978dd0e8fe93847086f1040b3a22a1603c/> improve capitalize output message logic in apt_handler mehod

### Style

 - <csr-id-05baa26ff7c75c4c103ad5f1b78f53f8b571c52c/> use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.22.0 ([`c8b5139`](https://github.com/AOSC-Dev/oma/commit/c8b513957c644802fa8b88433c7965760666608d))
    - Update all deps ([`a87a40d`](https://github.com/AOSC-Dev/oma/commit/a87a40d79c10a7acbf2f76ba6e4fa413e206ad46))
    - Use cargo-fmt to format code ([`05baa26`](https://github.com/AOSC-Dev/oma/commit/05baa26ff7c75c4c103ad5f1b78f53f8b571c52c))
    - Error output message adjust ([`b44035d`](https://github.com/AOSC-Dev/oma/commit/b44035d76b13675b85d6a2c648f6b6ab76eea448))
    - Fix autoremove/non-autoremove pkg pending ui wrong detail ([`4df750e`](https://github.com/AOSC-Dev/oma/commit/4df750e8587145d44bca5f112aace690e08c6107))
    - If needs run dpkg --configure -a, run it ([`3bb8b31`](https://github.com/AOSC-Dev/oma/commit/3bb8b317af8eaacd7a03eb5a0939a15f3449b37e))
    - Build all subcommand man ([`7ae0fac`](https://github.com/AOSC-Dev/oma/commit/7ae0faca3b79ecbbbf50f4f28e369bbeca4d13fb))
    - Improve capitalize output message logic in apt_handler mehod ([`46b75d9`](https://github.com/AOSC-Dev/oma/commit/46b75d978dd0e8fe93847086f1040b3a22a1603c))
</details>

## v0.21.0 (2023-04-03)

<csr-id-f80adf68ef978cdaee19eb6072ccb4449207c93c/>
<csr-id-b719e78fde46d4fd1b08bb3a87a8b8470e0cd827/>

### Chore

 - <csr-id-f80adf68ef978cdaee19eb6072ccb4449207c93c/> update all deps

### New Features

 - <csr-id-cc6546548e526d88360c0ed0750d4d5953a07c82/> If update dpkg-force-all mode after has broken count, return error
 - <csr-id-d7d0253dbfcd7dbea00d5da68be16e55e3f82577/> If retry 2 times apt has error, go to dpkg-force-all mode

### Bug Fixes

 - <csr-id-8cfc2cf9efd57e8a353a81f985695a24b63f901e/> Fix a typo

### Style

 - <csr-id-b719e78fde46d4fd1b08bb3a87a8b8470e0cd827/> use cargo fmt and cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.21.0 ([`5f50847`](https://github.com/AOSC-Dev/oma/commit/5f50847782c13f83c158e1412a2f2d41dbbcf522))
    - Update all deps ([`f80adf6`](https://github.com/AOSC-Dev/oma/commit/f80adf68ef978cdaee19eb6072ccb4449207c93c))
    - Fix a typo ([`8cfc2cf`](https://github.com/AOSC-Dev/oma/commit/8cfc2cf9efd57e8a353a81f985695a24b63f901e))
    - If update dpkg-force-all mode after has broken count, return error ([`cc65465`](https://github.com/AOSC-Dev/oma/commit/cc6546548e526d88360c0ed0750d4d5953a07c82))
    - If retry 2 times apt has error, go to dpkg-force-all mode ([`d7d0253`](https://github.com/AOSC-Dev/oma/commit/d7d0253dbfcd7dbea00d5da68be16e55e3f82577))
    - Use cargo fmt and cargo clippy to lint code ([`b719e78`](https://github.com/AOSC-Dev/oma/commit/b719e78fde46d4fd1b08bb3a87a8b8470e0cd827))
</details>

## v0.20.0 (2023-04-02)

### New Features

 - <csr-id-e7929f3296eb063e189cbb1604bbae35f072e263/> Improve progress bar style again
 - <csr-id-d931d46d1eb9397257eb052194723db49e102930/> Improve progress bar style
 - <csr-id-e7f9c50150305aa25da0893936a45fd6c80266ef/> Improve error message display

### Bug Fixes

 - <csr-id-20ad91331ba0e5f8e35f1a23b975e06f316d3549/> Fix /run/lock directory does not exist
 - <csr-id-5e699c2518112fa335cbf40681e9eea59a434456/> Fix oma subcommand history run

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 1 day passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.20.0 ([`e08795a`](https://github.com/AOSC-Dev/oma/commit/e08795ad39b00ec6051ce02e82e95003c176a0c2))
    - Improve progress bar style again ([`e7929f3`](https://github.com/AOSC-Dev/oma/commit/e7929f3296eb063e189cbb1604bbae35f072e263))
    - Correct typos in args.rs ([`acd0ad6`](https://github.com/AOSC-Dev/oma/commit/acd0ad6b0f7c6b91a8c220fd69215bc94f8dd5ea))
    - Improve progress bar style ([`d931d46`](https://github.com/AOSC-Dev/oma/commit/d931d46d1eb9397257eb052194723db49e102930))
    - Fix /run/lock directory does not exist ([`20ad913`](https://github.com/AOSC-Dev/oma/commit/20ad91331ba0e5f8e35f1a23b975e06f316d3549))
    - Fix oma subcommand history run ([`5e699c2`](https://github.com/AOSC-Dev/oma/commit/5e699c2518112fa335cbf40681e9eea59a434456))
    - Improve error message display ([`e7f9c50`](https://github.com/AOSC-Dev/oma/commit/e7f9c50150305aa25da0893936a45fd6c80266ef))
</details>

## v0.19.0 (2023-04-01)

<csr-id-4de171ac886e50ed1496337b513f0935717b5a8f/>
<csr-id-1e8d8ab5d2b8f7955c40945de32838f47606a010/>

### Chore

 - <csr-id-4de171ac886e50ed1496337b513f0935717b5a8f/> update rustix dep

### New Features

 - <csr-id-39e066e3adf8a5f20769e6c083b74238b8c8321b/> Add {upgrade,install,fix-broken} subcommand --dpkg-force-all argument

### Bug Fixes

 - <csr-id-13f4bf7fd1dccb1f4b7641f7c6b084dcd20a0d37/> Add missing progress bar logic

### Style

 - <csr-id-1e8d8ab5d2b8f7955c40945de32838f47606a010/> use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.19.0 ([`6a673d3`](https://github.com/AOSC-Dev/oma/commit/6a673d352453af8b8fd704ad9d9ed69d35aeced8))
    - Update rustix dep ([`4de171a`](https://github.com/AOSC-Dev/oma/commit/4de171ac886e50ed1496337b513f0935717b5a8f))
    - Use cargo-fmt to format code ([`1e8d8ab`](https://github.com/AOSC-Dev/oma/commit/1e8d8ab5d2b8f7955c40945de32838f47606a010))
    - Add {upgrade,install,fix-broken} subcommand --dpkg-force-all argument ([`39e066e`](https://github.com/AOSC-Dev/oma/commit/39e066e3adf8a5f20769e6c083b74238b8c8321b))
    - Add missing progress bar logic ([`13f4bf7`](https://github.com/AOSC-Dev/oma/commit/13f4bf7fd1dccb1f4b7641f7c6b084dcd20a0d37))
    - Revert "fix: do not display download progress in retry" ([`0848a19`](https://github.com/AOSC-Dev/oma/commit/0848a191a80e5f3aa5ca76124a233a16998b2643))
    - Revert "fix: fix yes argument download" ([`5736203`](https://github.com/AOSC-Dev/oma/commit/5736203f0fa697918f4d30cbf0605d02af48f971))
</details>

## v0.18.1 (2023-04-01)

<csr-id-3f8df6ab624f073948505739c9e8c7ee3731e242/>

### Bug Fixes

 - <csr-id-cc54069158c0a44c01ee7ffb5ac5c1256deee750/> Fix yes argument download
 - <csr-id-e346e06a30ddc829c73a8c981498ada4ce6844ab/> Do not display download progress in retry
 - <csr-id-de55a3218a0bf0bd79464f82787c548ad6a20749/> Pending ui message too loong to panic

### Refactor

 - <csr-id-3f8df6ab624f073948505739c9e8c7ee3731e242/> optmize download before check file is exist logic

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.18.1 ([`74c2dd8`](https://github.com/AOSC-Dev/oma/commit/74c2dd89137b903532c5be2aa2db8320f7d70b61))
    - Optmize download before check file is exist logic ([`3f8df6a`](https://github.com/AOSC-Dev/oma/commit/3f8df6ab624f073948505739c9e8c7ee3731e242))
    - Fix yes argument download ([`cc54069`](https://github.com/AOSC-Dev/oma/commit/cc54069158c0a44c01ee7ffb5ac5c1256deee750))
    - Do not display download progress in retry ([`e346e06`](https://github.com/AOSC-Dev/oma/commit/e346e06a30ddc829c73a8c981498ada4ce6844ab))
    - Pending ui message too loong to panic ([`de55a32`](https://github.com/AOSC-Dev/oma/commit/de55a3218a0bf0bd79464f82787c548ad6a20749))
</details>

## v0.18.0 (2023-03-31)

<csr-id-374b8e51c2b5169cb497e08cb5e1f9163083ead1/>
<csr-id-00ab06101ae2b211e2cbaea1fca7c76df2e6b1ac/>
<csr-id-eb9cae9c30805f5d80a6168b71a472bd53db9d2d/>
<csr-id-f57d97ffefb244e74890386ed720a3e026238907/>

### Chore

 - <csr-id-374b8e51c2b5169cb497e08cb5e1f9163083ead1/> update all deps
 - <csr-id-00ab06101ae2b211e2cbaea1fca7c76df2e6b1ac/> remove useless file
 - <csr-id-eb9cae9c30805f5d80a6168b71a472bd53db9d2d/> add man to .gitignore

### New Features

 - <csr-id-5960c70495227cb80ca9a991352857fde09ee60f/> Improve command short help

### Bug Fixes

 - <csr-id-6efcc4fb282384c1ec8f9db623cac57964c6a230/> Add missing oma mark help message
 - <csr-id-1d74e7083e36509cd54d636773c4b633c8c58973/> Add missing subcommand ...
   ... Also log subcommand rename history
 - <csr-id-67f099ac2d618a6a0aab7e99ea5ea708e8bee151/> Fix package name ends_with deb install

### Style

 - <csr-id-f57d97ffefb244e74890386ed720a3e026238907/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.18.0 ([`f944f91`](https://github.com/AOSC-Dev/oma/commit/f944f91f2d23b20b0c0a860dd1d84ca9651af743))
    - Update all deps ([`374b8e5`](https://github.com/AOSC-Dev/oma/commit/374b8e51c2b5169cb497e08cb5e1f9163083ead1))
    - Use cargo clippy to lint code ([`f57d97f`](https://github.com/AOSC-Dev/oma/commit/f57d97ffefb244e74890386ed720a3e026238907))
    - Add missing oma mark help message ([`6efcc4f`](https://github.com/AOSC-Dev/oma/commit/6efcc4fb282384c1ec8f9db623cac57964c6a230))
    - Improve command short help ([`5960c70`](https://github.com/AOSC-Dev/oma/commit/5960c70495227cb80ca9a991352857fde09ee60f))
    - Add missing subcommand ... ([`1d74e70`](https://github.com/AOSC-Dev/oma/commit/1d74e7083e36509cd54d636773c4b633c8c58973))
    - Remove useless file ([`00ab061`](https://github.com/AOSC-Dev/oma/commit/00ab06101ae2b211e2cbaea1fca7c76df2e6b1ac))
    - Fix package name ends_with deb install ([`67f099a`](https://github.com/AOSC-Dev/oma/commit/67f099ac2d618a6a0aab7e99ea5ea708e8bee151))
    - Add man to .gitignore ([`eb9cae9`](https://github.com/AOSC-Dev/oma/commit/eb9cae9c30805f5d80a6168b71a472bd53db9d2d))
</details>

## v0.17.1 (2023-03-31)

<csr-id-08eb9d4349fe67e3fdb49f299023a3b29d385689/>
<csr-id-ca593f5ffdb80db1e159ce7dc59dbcb7d4acd921/>
<csr-id-5e8d086e7809ac3cd039c83eb4f618a25aa69e16/>
<csr-id-595f7772d54b17b5f46d125140073597b67551f3/>
<csr-id-f4318cb27202a3e99d30e718560254d8ae4a1449/>
<csr-id-a8e1ca91d85eafa1acd91994640eb029b86669d5/>
<csr-id-5e2a8adb0e1b7dce39a74b0f3d7aaaca4d3756ab/>

### Chore

 - <csr-id-08eb9d4349fe67e3fdb49f299023a3b29d385689/> update all deps
   - Also remove useless dep
 - <csr-id-ca593f5ffdb80db1e159ce7dc59dbcb7d4acd921/> clap_cli.rs => args.rs
 - <csr-id-5e8d086e7809ac3cd039c83eb4f618a25aa69e16/> update README
 - <csr-id-595f7772d54b17b5f46d125140073597b67551f3/> remove useless tracing-subscriber envfilter dep

### New Features

 - <csr-id-438b89df8796ab1b74f2c2f69ac818e5f5ae764c/> Output man pages to /man
 - <csr-id-f3f2ebba5b4feb23ecb3817c2571884d8588312b/> Try use clap to gen man
 - <csr-id-3d572de0735df4fa193b6168e71657317b50995d/> Add extract and verify database progress bar

### Refactor

 - <csr-id-f4318cb27202a3e99d30e718560254d8ae4a1449/> use clap build api to build argument
   - Also fix if fix-broken count == 0 display less by eatradish

### Style

 - <csr-id-a8e1ca91d85eafa1acd91994640eb029b86669d5/> run cargo clippy
 - <csr-id-5e2a8adb0e1b7dce39a74b0f3d7aaaca4d3756ab/> move single_handler code line location

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.17.1 ([`7e80de5`](https://github.com/AOSC-Dev/oma/commit/7e80de5aad6cf344e5e78d770f163c7d7f184528))
    - Run cargo clippy ([`a8e1ca9`](https://github.com/AOSC-Dev/oma/commit/a8e1ca91d85eafa1acd91994640eb029b86669d5))
    - Update all deps ([`08eb9d4`](https://github.com/AOSC-Dev/oma/commit/08eb9d4349fe67e3fdb49f299023a3b29d385689))
    - Output man pages to /man ([`438b89d`](https://github.com/AOSC-Dev/oma/commit/438b89df8796ab1b74f2c2f69ac818e5f5ae764c))
    - Use clap build api to build argument ([`f4318cb`](https://github.com/AOSC-Dev/oma/commit/f4318cb27202a3e99d30e718560254d8ae4a1449))
    - Clap_cli.rs => args.rs ([`ca593f5`](https://github.com/AOSC-Dev/oma/commit/ca593f5ffdb80db1e159ce7dc59dbcb7d4acd921))
    - Try use clap to gen man ([`f3f2ebb`](https://github.com/AOSC-Dev/oma/commit/f3f2ebba5b4feb23ecb3817c2571884d8588312b))
    - Move single_handler code line location ([`5e2a8ad`](https://github.com/AOSC-Dev/oma/commit/5e2a8adb0e1b7dce39a74b0f3d7aaaca4d3756ab))
    - Add extract and verify database progress bar ([`3d572de`](https://github.com/AOSC-Dev/oma/commit/3d572de0735df4fa193b6168e71657317b50995d))
    - Update README ([`5e8d086`](https://github.com/AOSC-Dev/oma/commit/5e8d086e7809ac3cd039c83eb4f618a25aa69e16))
    - Remove useless tracing-subscriber envfilter dep ([`595f777`](https://github.com/AOSC-Dev/oma/commit/595f7772d54b17b5f46d125140073597b67551f3))
</details>

## v0.17.0 (2023-03-28)

<csr-id-0dca24982887115dd11b30f58a65d5014b2e1419/>
<csr-id-6def641dc68e68b54970090dea4d618f19b60787/>
<csr-id-58fd8c6a8eea2aa918ed9d271f995bfc1b598851/>
<csr-id-06793013e11364e1aa64f81e9183e930888ba8bc/>
<csr-id-50b9512f1e270012888c720a521a9354402fcf34/>
<csr-id-fddc86f2e61261f1e842c5a24eb94e6c49660bff/>
<csr-id-6bc254e135c6613852f3a221df30694988310a50/>
<csr-id-9bb717573b5ee6928f139613cd0145dd00f120d5/>
<csr-id-6b90abdd8e1cd2a7140dc2c0ea881d8a2e875bd0/>
<csr-id-ed199fddf44d379e9fa04af24ead28568814f9d6/>
<csr-id-4f8a5ffdc160319ca28d6251bc2f6e5ccef0ceb7/>

### Chore

 - <csr-id-0dca24982887115dd11b30f58a65d5014b2e1419/> update all deps
 - <csr-id-6def641dc68e68b54970090dea4d618f19b60787/> update rust-apt to newest git snapshot
 - <csr-id-58fd8c6a8eea2aa918ed9d271f995bfc1b598851/> add dependencies comment in Cargo.toml

### New Features

 - <csr-id-355746f2e38a2832e4ae645862eeef7f61387eea/> If fetch last url has error, output error prefix
 - <csr-id-33cb84a6eddfb5b2a507dff3361a09a1c874d56f/> Add .policy file to add policykit oma infomation
 - <csr-id-a6ae5aede440aa515bd05257cc6f9701f491cdaf/> Add policykit support

### Bug Fixes

 - <csr-id-980480288e2a2793ac2164994076c12ee814d865/> Fix warning message before global bar draw display
 - <csr-id-b6006ee97d648426e4b70dc32be45ebe0a50d0c5/> Try to fix download progress bar count
 - <csr-id-12f4e4cb784d2a7c93dd394326b493b74fa92005/> Fix download database global bar display in file:// prefix local mirror
 - <csr-id-388a00416cee98107d89d7d38f6196a1223053c4/> Fix exit code with policykit run

### Other

 - <csr-id-06793013e11364e1aa64f81e9183e930888ba8bc/> default to /bin/oma
 - <csr-id-50b9512f1e270012888c720a521a9354402fcf34/> improve UI strings

### Refactor

 - <csr-id-fddc86f2e61261f1e842c5a24eb94e6c49660bff/> decompress database do not block tokio runner
 - <csr-id-6bc254e135c6613852f3a221df30694988310a50/> refactor content::file_handle method; rename to remove_prefix
 - <csr-id-9bb717573b5ee6928f139613cd0145dd00f120d5/> refactor some code style
 - <csr-id-6b90abdd8e1cd2a7140dc2c0ea881d8a2e875bd0/> do not always in running in async runtime

### Style

 - <csr-id-ed199fddf44d379e9fa04af24ead28568814f9d6/> run cargo fmt and clippy
 - <csr-id-4f8a5ffdc160319ca28d6251bc2f6e5ccef0ceb7/> OmaAction => Oma

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.17.0 ([`7562a18`](https://github.com/AOSC-Dev/oma/commit/7562a18ed823cb51ce23007621b347d6e7761d16))
    - Update all deps ([`0dca249`](https://github.com/AOSC-Dev/oma/commit/0dca24982887115dd11b30f58a65d5014b2e1419))
    - Run cargo fmt and clippy ([`ed199fd`](https://github.com/AOSC-Dev/oma/commit/ed199fddf44d379e9fa04af24ead28568814f9d6))
    - If fetch last url has error, output error prefix ([`355746f`](https://github.com/AOSC-Dev/oma/commit/355746f2e38a2832e4ae645862eeef7f61387eea))
    - Fix warning message before global bar draw display ([`9804802`](https://github.com/AOSC-Dev/oma/commit/980480288e2a2793ac2164994076c12ee814d865))
    - Try to fix download progress bar count ([`b6006ee`](https://github.com/AOSC-Dev/oma/commit/b6006ee97d648426e4b70dc32be45ebe0a50d0c5))
    - Fix download database global bar display in file:// prefix local mirror ([`12f4e4c`](https://github.com/AOSC-Dev/oma/commit/12f4e4cb784d2a7c93dd394326b493b74fa92005))
    - Default to /bin/oma ([`0679301`](https://github.com/AOSC-Dev/oma/commit/06793013e11364e1aa64f81e9183e930888ba8bc))
    - Improve UI strings ([`50b9512`](https://github.com/AOSC-Dev/oma/commit/50b9512f1e270012888c720a521a9354402fcf34))
    - Add .policy file to add policykit oma infomation ([`33cb84a`](https://github.com/AOSC-Dev/oma/commit/33cb84a6eddfb5b2a507dff3361a09a1c874d56f))
    - Decompress database do not block tokio runner ([`fddc86f`](https://github.com/AOSC-Dev/oma/commit/fddc86f2e61261f1e842c5a24eb94e6c49660bff))
    - Refactor content::file_handle method; rename to remove_prefix ([`6bc254e`](https://github.com/AOSC-Dev/oma/commit/6bc254e135c6613852f3a221df30694988310a50))
    - Update rust-apt to newest git snapshot ([`6def641`](https://github.com/AOSC-Dev/oma/commit/6def641dc68e68b54970090dea4d618f19b60787))
    - Add dependencies comment in Cargo.toml ([`58fd8c6`](https://github.com/AOSC-Dev/oma/commit/58fd8c6a8eea2aa918ed9d271f995bfc1b598851))
    - OmaAction => Oma ([`4f8a5ff`](https://github.com/AOSC-Dev/oma/commit/4f8a5ffdc160319ca28d6251bc2f6e5ccef0ceb7))
    - Refactor some code style ([`9bb7175`](https://github.com/AOSC-Dev/oma/commit/9bb717573b5ee6928f139613cd0145dd00f120d5))
    - Do not always in running in async runtime ([`6b90abd`](https://github.com/AOSC-Dev/oma/commit/6b90abdd8e1cd2a7140dc2c0ea881d8a2e875bd0))
    - Fix exit code with policykit run ([`388a004`](https://github.com/AOSC-Dev/oma/commit/388a00416cee98107d89d7d38f6196a1223053c4))
    - Add policykit support ([`a6ae5ae`](https://github.com/AOSC-Dev/oma/commit/a6ae5aede440aa515bd05257cc6f9701f491cdaf))
</details>

## v0.16.0 (2023-03-27)

<csr-id-85ad4b758dea42854c920957b69ae544f6d91a17/>
<csr-id-c411087f4cf27b06f87b9db9ef8701f5c787ad81/>
<csr-id-bbfc384d3cb5289743014bf7a5e2805bb69dc4d0/>
<csr-id-cf804b98d6220921a6b585b8da7bd3512c3268d9/>
<csr-id-dfbb9e418273d2f20f08a5c4b9b6b09a82935110/>
<csr-id-948923de893b131e4fbd59bc5ed4be1ca383b69d/>
<csr-id-2dde83d82417506b9fa0f4a39237e8596dc6f530/>
<csr-id-7adb76c9fe0c33b02ada9e856306db6855459227/>

### Chore

 - <csr-id-85ad4b758dea42854c920957b69ae544f6d91a17/> update all deps

### New Features

 - <csr-id-3c1d8355ced332e3c0249597cd6067798d21d9f5/> Oma dep/rdep PreDepended by => Pre-Depended by
 - <csr-id-059e770325bf9c72b616324435b73185fa836859/> Oma dep/rdep improve grammar display
 - <csr-id-6dd009b0aed38352c24e5d36c9389a662f8ee8d2/> Do not display JSON-like args info in dry-run mode
 - <csr-id-99e0d9c46c82ebe8f7882ef14a1f1242ebddf626/> Add oma --debug argument to see dry-run debug message
 - <csr-id-20368da4ca497bb5bbbf93e68599bf94f6c755b8/> Log file remove 'Resolver' word
 - <csr-id-6f1c53a2d60bced6abc07128256866559874d227/> Improve oma log Resolver key style
 - <csr-id-02217a97c880bee13efcefba8de8754f5e79a965/> Log user actions in a human-readable fashion
 - <csr-id-88996e144efe83d63e20b4b71779cdde6d65fe5d/> Find all provides in search.rs method to improve search result
 - <csr-id-afd404efad65a2089f0e12bdf3f22081dcd3da43/> Fix reset is_provide status in search_pkgs method
 - <csr-id-3b4edeb9138f6b00547e14f967754193186490f2/> Improve oma search result sort
 - <csr-id-d44c3469404f075a831332f3018f2b91a81a793a/> Read Dir::Cache::Archives value to get apt set download dir config
 - <csr-id-6b2a70a025e91f3e7b9c3f528ec487cc08e8c719/> Oma download success will display download packages path
 - <csr-id-45d2dd15e3693adcbe0a6a9e032184a3e2d3e228/> Oma download add argument --path (-p)
 - <csr-id-53e1d9e88c83113980483074016f552d1612e452/> Support provides package install

### Bug Fixes

 - <csr-id-93835ef2382f6dfde75e59c35061f7ef5ed12f0b/> Fix oma start-date/end-date localtime offset
 - <csr-id-883dc2bb19810972b49c7b9129287b29c2b0f6d4/> Fix local package install
   But not using a better approach, wait https://gitlab.com/volian/rust-apt/-/issues/23
 - <csr-id-36cd76ffa6646d302fa2e5ad416b61a9c7c2fac3/> Add Dir::Cache::Archives fallback logic
 - <csr-id-3c62b065befd8e2cf9819ea41e3d6d2cee4e63e4/> Fix archive dir read logic
 - <csr-id-4e71663770db750e45ed749a645689ae9f3c4b1d/> Only virtual pkg get provides to get real pkg
 - <csr-id-ded754f888c1587fcfab76353ba8a92008fd6019/> Oma download do not download non candidate pkg
 - <csr-id-932b17beff2a4e3878833a7a45ee5f9e90dd9f1c/> Local mirror progress display

### Refactor

 - <csr-id-c411087f4cf27b06f87b9db9ef8701f5c787ad81/> improve query pkg method
 - <csr-id-bbfc384d3cb5289743014bf7a5e2805bb69dc4d0/> improve get local pkgs
   Now, no need depend dep-archive

### Style

 - <csr-id-cf804b98d6220921a6b585b8da7bd3512c3268d9/> run cargo clippy and cargo fmt to lint code
 - <csr-id-dfbb9e418273d2f20f08a5c4b9b6b09a82935110/> lint code use cargo clippy
 - <csr-id-948923de893b131e4fbd59bc5ed4be1ca383b69d/> use cargo clippy and cargo fmt
 - <csr-id-2dde83d82417506b9fa0f4a39237e8596dc6f530/> drop useless line
 - <csr-id-7adb76c9fe0c33b02ada9e856306db6855459227/> run cargo clippy'
   - Also cargo fmt

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 30 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 29 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.16.0 ([`24c6dad`](https://github.com/AOSC-Dev/oma/commit/24c6dadb4e50c634490f77ddabd5be05f82068c5))
    - Update all deps ([`85ad4b7`](https://github.com/AOSC-Dev/oma/commit/85ad4b758dea42854c920957b69ae544f6d91a17))
    - Run cargo clippy and cargo fmt to lint code ([`cf804b9`](https://github.com/AOSC-Dev/oma/commit/cf804b98d6220921a6b585b8da7bd3512c3268d9))
    - Oma dep/rdep PreDepended by => Pre-Depended by ([`3c1d835`](https://github.com/AOSC-Dev/oma/commit/3c1d8355ced332e3c0249597cd6067798d21d9f5))
    - Oma dep/rdep improve grammar display ([`059e770`](https://github.com/AOSC-Dev/oma/commit/059e770325bf9c72b616324435b73185fa836859))
    - Do not display JSON-like args info in dry-run mode ([`6dd009b`](https://github.com/AOSC-Dev/oma/commit/6dd009b0aed38352c24e5d36c9389a662f8ee8d2))
    - Add oma --debug argument to see dry-run debug message ([`99e0d9c`](https://github.com/AOSC-Dev/oma/commit/99e0d9c46c82ebe8f7882ef14a1f1242ebddf626))
    - Log file remove 'Resolver' word ([`20368da`](https://github.com/AOSC-Dev/oma/commit/20368da4ca497bb5bbbf93e68599bf94f6c755b8))
    - Improve oma log Resolver key style ([`6f1c53a`](https://github.com/AOSC-Dev/oma/commit/6f1c53a2d60bced6abc07128256866559874d227))
    - Log user actions in a human-readable fashion ([`02217a9`](https://github.com/AOSC-Dev/oma/commit/02217a97c880bee13efcefba8de8754f5e79a965))
    - Lint code use cargo clippy ([`dfbb9e4`](https://github.com/AOSC-Dev/oma/commit/dfbb9e418273d2f20f08a5c4b9b6b09a82935110))
    - Fix oma start-date/end-date localtime offset ([`93835ef`](https://github.com/AOSC-Dev/oma/commit/93835ef2382f6dfde75e59c35061f7ef5ed12f0b))
    - Find all provides in search.rs method to improve search result ([`88996e1`](https://github.com/AOSC-Dev/oma/commit/88996e144efe83d63e20b4b71779cdde6d65fe5d))
    - Fix reset is_provide status in search_pkgs method ([`afd404e`](https://github.com/AOSC-Dev/oma/commit/afd404efad65a2089f0e12bdf3f22081dcd3da43))
    - Improve oma search result sort ([`3b4edeb`](https://github.com/AOSC-Dev/oma/commit/3b4edeb9138f6b00547e14f967754193186490f2))
    - Use cargo clippy and cargo fmt ([`948923d`](https://github.com/AOSC-Dev/oma/commit/948923de893b131e4fbd59bc5ed4be1ca383b69d))
    - Fix local package install ([`883dc2b`](https://github.com/AOSC-Dev/oma/commit/883dc2bb19810972b49c7b9129287b29c2b0f6d4))
    - Add Dir::Cache::Archives fallback logic ([`36cd76f`](https://github.com/AOSC-Dev/oma/commit/36cd76ffa6646d302fa2e5ad416b61a9c7c2fac3))
    - Drop useless line ([`2dde83d`](https://github.com/AOSC-Dev/oma/commit/2dde83d82417506b9fa0f4a39237e8596dc6f530))
    - Fix archive dir read logic ([`3c62b06`](https://github.com/AOSC-Dev/oma/commit/3c62b065befd8e2cf9819ea41e3d6d2cee4e63e4))
    - Read Dir::Cache::Archives value to get apt set download dir config ([`d44c346`](https://github.com/AOSC-Dev/oma/commit/d44c3469404f075a831332f3018f2b91a81a793a))
    - Run cargo clippy' ([`7adb76c`](https://github.com/AOSC-Dev/oma/commit/7adb76c9fe0c33b02ada9e856306db6855459227))
    - Oma download success will display download packages path ([`6b2a70a`](https://github.com/AOSC-Dev/oma/commit/6b2a70a025e91f3e7b9c3f528ec487cc08e8c719))
    - Oma download add argument --path (-p) ([`45d2dd1`](https://github.com/AOSC-Dev/oma/commit/45d2dd15e3693adcbe0a6a9e032184a3e2d3e228))
    - Only virtual pkg get provides to get real pkg ([`4e71663`](https://github.com/AOSC-Dev/oma/commit/4e71663770db750e45ed749a645689ae9f3c4b1d))
    - Improve query pkg method ([`c411087`](https://github.com/AOSC-Dev/oma/commit/c411087f4cf27b06f87b9db9ef8701f5c787ad81))
    - Improve get local pkgs ([`bbfc384`](https://github.com/AOSC-Dev/oma/commit/bbfc384d3cb5289743014bf7a5e2805bb69dc4d0))
    - Support provides package install ([`53e1d9e`](https://github.com/AOSC-Dev/oma/commit/53e1d9e88c83113980483074016f552d1612e452))
    - Oma download do not download non candidate pkg ([`ded754f`](https://github.com/AOSC-Dev/oma/commit/ded754f888c1587fcfab76353ba8a92008fd6019))
    - Local mirror progress display ([`932b17b`](https://github.com/AOSC-Dev/oma/commit/932b17beff2a4e3878833a7a45ee5f9e90dd9f1c))
</details>

## v0.15.0 (2023-03-24)

<csr-id-44235f94ccb41303e33e308a6b75215d2d2f2f48/>
<csr-id-698396ab0be30c4d815138fa6ebc5cda4a41df43/>
<csr-id-4f9782f2112188ce689133654d4122835b57742e/>

### Chore

 - <csr-id-44235f94ccb41303e33e308a6b75215d2d2f2f48/> update all deps

### New Features

 - <csr-id-3be76aab9e4380467e050acd0b6dd00613c99b10/> Dry-run read RUST_LOG env to see debug message (default RUST_LOG is info)
 - <csr-id-8aafec43ec24de10bcf0544ec078ceb8f8d4ad02/> Set oma and os dry-run info as debug
 - <csr-id-ce234a3d9ffc2622235d5ca6d1e762d1629a6d23/> Improve log user-family output
 - <csr-id-900a15564cf0a88ade4efab875fb080fe334a9d1/> Dry-run mode display oma and OS info
   - Also fix dry-run downgrade typo

### Bug Fixes

 - <csr-id-6e100d38ecd6a2787bb2b94a9d531a706bfa27db/> Fix dry-run default display log
 - <csr-id-1276493d91b129fa63307f872522071c4d239f8a/> Fix dry-run in fix-broken subcommand argument
 - <csr-id-526bc122ea477eb36c99415892e11df14ae3e452/> Do not real run {mark,clean,download} in dry-run mode

### Style

 - <csr-id-698396ab0be30c4d815138fa6ebc5cda4a41df43/> use cargo fmt to lint code style
 - <csr-id-4f9782f2112188ce689133654d4122835b57742e/> improve pick method code style
   - Also run cargo fmt

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.15.0 ([`679d8e7`](https://github.com/AOSC-Dev/oma/commit/679d8e7f6ff23d1c848c09a46891e02f2f492681))
    - Update all deps ([`44235f9`](https://github.com/AOSC-Dev/oma/commit/44235f94ccb41303e33e308a6b75215d2d2f2f48))
    - Fix dry-run default display log ([`6e100d3`](https://github.com/AOSC-Dev/oma/commit/6e100d38ecd6a2787bb2b94a9d531a706bfa27db))
    - Use cargo fmt to lint code style ([`698396a`](https://github.com/AOSC-Dev/oma/commit/698396ab0be30c4d815138fa6ebc5cda4a41df43))
    - Dry-run read RUST_LOG env to see debug message (default RUST_LOG is info) ([`3be76aa`](https://github.com/AOSC-Dev/oma/commit/3be76aab9e4380467e050acd0b6dd00613c99b10))
    - Fix dry-run in fix-broken subcommand argument ([`1276493`](https://github.com/AOSC-Dev/oma/commit/1276493d91b129fa63307f872522071c4d239f8a))
    - Do not real run {mark,clean,download} in dry-run mode ([`526bc12`](https://github.com/AOSC-Dev/oma/commit/526bc122ea477eb36c99415892e11df14ae3e452))
    - Set oma and os dry-run info as debug ([`8aafec4`](https://github.com/AOSC-Dev/oma/commit/8aafec43ec24de10bcf0544ec078ceb8f8d4ad02))
    - Improve log user-family output ([`ce234a3`](https://github.com/AOSC-Dev/oma/commit/ce234a3d9ffc2622235d5ca6d1e762d1629a6d23))
    - Dry-run mode display oma and OS info ([`900a155`](https://github.com/AOSC-Dev/oma/commit/900a15564cf0a88ade4efab875fb080fe334a9d1))
    - Improve pick method code style ([`4f9782f`](https://github.com/AOSC-Dev/oma/commit/4f9782f2112188ce689133654d4122835b57742e))
</details>

## v0.14.0 (2023-03-23)

<csr-id-ff6749dce59ae4fa50cac786cfaecc17e828c040/>
<csr-id-89ea59750fccbc487ea7826193e85bbfb39ce14b/>

### New Features

<csr-id-4e6bb0a59edd36b02f7fd0e656f37d5acc5bb0db/>

 - <csr-id-903748b5ce82e5bb0c7b78a75b9f1dbe3d32d1ea/> Dry-run mode add args tracing
 - <csr-id-123da2c985075fe3ae4a04be55e007fdede83460/> Add oma --dry-run argument
   - Also fix oma pick dialoguer default select position

### Refactor

 - <csr-id-ff6749dce59ae4fa50cac786cfaecc17e828c040/> improve DOWNLOAD_DIR var use
 - <csr-id-89ea59750fccbc487ea7826193e85bbfb39ce14b/> use fs::read to replace fs::File::open and read_buf

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.14.0 ([`241a797`](https://github.com/AOSC-Dev/oma/commit/241a797fa428db235522848a2c9ffb28b3dfedbf))
    - Dry-run mode add args tracing ([`903748b`](https://github.com/AOSC-Dev/oma/commit/903748b5ce82e5bb0c7b78a75b9f1dbe3d32d1ea))
    - Add oma --dry-run argument ([`123da2c`](https://github.com/AOSC-Dev/oma/commit/123da2c985075fe3ae4a04be55e007fdede83460))
    - Improve DOWNLOAD_DIR var use ([`ff6749d`](https://github.com/AOSC-Dev/oma/commit/ff6749dce59ae4fa50cac786cfaecc17e828c040))
    - Use fs::read to replace fs::File::open and read_buf ([`89ea597`](https://github.com/AOSC-Dev/oma/commit/89ea59750fccbc487ea7826193e85bbfb39ce14b))
    - If pkg is essential, oma will reject it mark to delete ([`4e6bb0a`](https://github.com/AOSC-Dev/oma/commit/4e6bb0a59edd36b02f7fd0e656f37d5acc5bb0db))
</details>

## v0.13.2 (2023-03-22)

<csr-id-600f7d65ca44dc2e91813db55a47ee3e63c7628a/>
<csr-id-eb77d991a3c2bf21a784af41b5cc92fc0792af42/>
<csr-id-1c119f3e53f2e0bb5a5ca55c1d53c9431fd60caf/>
<csr-id-4431915c4fa66237e93705b19771147d1d660ad8/>
<csr-id-3e9764b82afaf587fb207edc00bb94202117d181/>
<csr-id-ced63ab66de0027cd072cf28457bfe9af7091835/>
<csr-id-942a98490c370741f08b34d0fc4f0ee49c3cb904/>
<csr-id-c147f6f66b54f51dcbbc95a84af04764602913ab/>
<csr-id-2b19d1d1d43503b696d1f68e825e8db62e940851/>
<csr-id-35e27cf6eb267da34f6d07e7f0df8ac6564befa0/>
<csr-id-acf7e43838811177f4838cf2a97a217540803e86/>
<csr-id-1341b47750c8541fb6cdfabe4b0191443c407a10/>

### Other

 - <csr-id-600f7d65ca44dc2e91813db55a47ee3e63c7628a/> bump version to 0.13.2 for adapt cargo-smart-release
 - <csr-id-eb77d991a3c2bf21a784af41b5cc92fc0792af42/> cargo fmt
   - Also update all deps
 - <csr-id-1c119f3e53f2e0bb5a5ca55c1d53c9431fd60caf/> find_unmet_deps_with_markinstall if apt cache could not find pkg, add to UnmetTable list
 - <csr-id-4431915c4fa66237e93705b19771147d1d660ad8/> find_unmet_deps_with_markinstall method do not display 'User aborted the operation' info
 - <csr-id-3e9764b82afaf587fb207edc00bb94202117d181/> add find_unmet_deps_with_markinstall method to handle if mark_install can't success
 - <csr-id-ced63ab66de0027cd072cf28457bfe9af7091835/> if find_unmet_deps can't find any dependencies problem, return apt's error
 - <csr-id-942a98490c370741f08b34d0fc4f0ee49c3cb904/> update
 - <csr-id-c147f6f66b54f51dcbbc95a84af04764602913ab/> fake clap more like real clap
 - <csr-id-2b19d1d1d43503b696d1f68e825e8db62e940851/> fake clap more like real clap
 - <csr-id-35e27cf6eb267da34f6d07e7f0df8ac6564befa0/> use cargo fmt
 - <csr-id-acf7e43838811177f4838cf2a97a217540803e86/> add fake clap output for wrong --ailurus argument count

### Other

 - <csr-id-1341b47750c8541fb6cdfabe4b0191443c407a10/> new

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 1 day passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma v0.13.2 ([`490b22f`](https://github.com/AOSC-Dev/oma/commit/490b22fe4aa5a3fc5dc6988c42c0755a8abf9b60))
    - New ([`1341b47`](https://github.com/AOSC-Dev/oma/commit/1341b47750c8541fb6cdfabe4b0191443c407a10))
    - Bump version to 0.13.2 for adapt cargo-smart-release ([`600f7d6`](https://github.com/AOSC-Dev/oma/commit/600f7d65ca44dc2e91813db55a47ee3e63c7628a))
    - Cargo fmt ([`eb77d99`](https://github.com/AOSC-Dev/oma/commit/eb77d991a3c2bf21a784af41b5cc92fc0792af42))
    - Find_unmet_deps_with_markinstall if apt cache could not find pkg, add to UnmetTable list ([`1c119f3`](https://github.com/AOSC-Dev/oma/commit/1c119f3e53f2e0bb5a5ca55c1d53c9431fd60caf))
    - Find_unmet_deps_with_markinstall method do not display 'User aborted the operation' info ([`4431915`](https://github.com/AOSC-Dev/oma/commit/4431915c4fa66237e93705b19771147d1d660ad8))
    - Add find_unmet_deps_with_markinstall method to handle if mark_install can't success ([`3e9764b`](https://github.com/AOSC-Dev/oma/commit/3e9764b82afaf587fb207edc00bb94202117d181))
    - If find_unmet_deps can't find any dependencies problem, return apt's error ([`ced63ab`](https://github.com/AOSC-Dev/oma/commit/ced63ab66de0027cd072cf28457bfe9af7091835))
    - Update ([`942a984`](https://github.com/AOSC-Dev/oma/commit/942a98490c370741f08b34d0fc4f0ee49c3cb904))
    - Fake clap more like real clap ([`c147f6f`](https://github.com/AOSC-Dev/oma/commit/c147f6f66b54f51dcbbc95a84af04764602913ab))
    - Revert "main: fake clap more like real clap" ([`9f19391`](https://github.com/AOSC-Dev/oma/commit/9f19391cf0a06bb08c2bebdbe075360e12fff62b))
    - Fake clap more like real clap ([`2b19d1d`](https://github.com/AOSC-Dev/oma/commit/2b19d1d1d43503b696d1f68e825e8db62e940851))
    - Use cargo fmt ([`35e27cf`](https://github.com/AOSC-Dev/oma/commit/35e27cf6eb267da34f6d07e7f0df8ac6564befa0))
    - Add fake clap output for wrong --ailurus argument count ([`acf7e43`](https://github.com/AOSC-Dev/oma/commit/acf7e43838811177f4838cf2a97a217540803e86))
</details>

## v0.13.1 (2023-03-21)

<csr-id-3203ddf3619c2d5680d9c30e872675e98e752a56/>
<csr-id-94c2694ff1dfd8eccccd01e1e280b9418e83ae1b/>
<csr-id-0ae546044d7a3c5b5dedf971d44f485e5b8dd270/>
<csr-id-cd0eeeb92dcd0f037e4ea4ff55430129b29bc551/>
<csr-id-70e34c78bc001ed51eec97a0ae340ba78a8d75b6/>
<csr-id-0facb38b8edee4c8e1dcf9448c0fe5da7ae87600/>
<csr-id-d693d2be4c2690182faf4121af71ff93b513159f/>
<csr-id-38d8386882bd863da094a8b5ca6dadc0f53a41b7/>
<csr-id-bc35416bf773ec00df6e7de4efdbc1fffe54d83c/>
<csr-id-02ab2e5aeed834cb26a660b8e88a3081f838ae92/>
<csr-id-feb5116fda77e1e67e68853adfae7c2189fa77c9/>
<csr-id-5e24eaccd5828d51439ef7d18debf5f192559e46/>
<csr-id-64fedc84eb38d0640239ea559b67110547aa63be/>
<csr-id-c7fe71c710e8284549913abbf16eb81be2c38a43/>
<csr-id-c0380232311a6ee7871ddb17d8b6f396831a34e3/>
<csr-id-15b02280f87894e3c0367cda82b03e1f629f22ee/>

### Other

 - <csr-id-3203ddf3619c2d5680d9c30e872675e98e752a56/> 0.13.1
 - <csr-id-94c2694ff1dfd8eccccd01e1e280b9418e83ae1b/> improve pending ui style
 - <csr-id-0ae546044d7a3c5b5dedf971d44f485e5b8dd270/> 0.13.0
   - Also use cargo clippy, cargo fmt and update all deps
 - <csr-id-cd0eeeb92dcd0f037e4ea4ff55430129b29bc551/> progress spinner use oma style
 - <csr-id-70e34c78bc001ed51eec97a0ae340ba78a8d75b6/> unmet ui do not right align
 - <csr-id-0facb38b8edee4c8e1dcf9448c0fe5da7ae87600/> improve unmet pending ui style
 - <csr-id-d693d2be4c2690182faf4121af71ff93b513159f/> add PreDepends to unmet dependencies table
 - <csr-id-38d8386882bd863da094a8b5ca6dadc0f53a41b7/> add Break and Conflict to unmet dependencies table
 - <csr-id-bc35416bf773ec00df6e7de4efdbc1fffe54d83c/> use cargo clippy
 - <csr-id-02ab2e5aeed834cb26a660b8e88a3081f838ae92/> use OnceCell::Lazy<PathBuf> to replace Path String
 - <csr-id-feb5116fda77e1e67e68853adfae7c2189fa77c9/> move mark_install method to pkg.rs
 - <csr-id-5e24eaccd5828d51439ef7d18debf5f192559e46/> adjust code stract
 - <csr-id-64fedc84eb38d0640239ea559b67110547aa63be/> improve find unmet dep logic
 - <csr-id-c7fe71c710e8284549913abbf16eb81be2c38a43/> do not display user abort op in find_unmet dep method
 - <csr-id-c0380232311a6ee7871ddb17d8b6f396831a34e3/> add unmet dependency error output
 - <csr-id-15b02280f87894e3c0367cda82b03e1f629f22ee/> use Lazy<Writer> replaced OnceCell<Writer>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.13.1 ([`3203ddf`](https://github.com/AOSC-Dev/oma/commit/3203ddf3619c2d5680d9c30e872675e98e752a56))
    - Improve pending ui style ([`94c2694`](https://github.com/AOSC-Dev/oma/commit/94c2694ff1dfd8eccccd01e1e280b9418e83ae1b))
    - 0.13.0 ([`0ae5460`](https://github.com/AOSC-Dev/oma/commit/0ae546044d7a3c5b5dedf971d44f485e5b8dd270))
    - Progress spinner use oma style ([`cd0eeeb`](https://github.com/AOSC-Dev/oma/commit/cd0eeeb92dcd0f037e4ea4ff55430129b29bc551))
    - Unmet ui do not right align ([`70e34c7`](https://github.com/AOSC-Dev/oma/commit/70e34c78bc001ed51eec97a0ae340ba78a8d75b6))
    - Improve unmet pending ui style ([`0facb38`](https://github.com/AOSC-Dev/oma/commit/0facb38b8edee4c8e1dcf9448c0fe5da7ae87600))
    - Add PreDepends to unmet dependencies table ([`d693d2b`](https://github.com/AOSC-Dev/oma/commit/d693d2be4c2690182faf4121af71ff93b513159f))
    - Add Break and Conflict to unmet dependencies table ([`38d8386`](https://github.com/AOSC-Dev/oma/commit/38d8386882bd863da094a8b5ca6dadc0f53a41b7))
    - Use cargo clippy ([`bc35416`](https://github.com/AOSC-Dev/oma/commit/bc35416bf773ec00df6e7de4efdbc1fffe54d83c))
    - Use OnceCell::Lazy<PathBuf> to replace Path String ([`02ab2e5`](https://github.com/AOSC-Dev/oma/commit/02ab2e5aeed834cb26a660b8e88a3081f838ae92))
    - Move mark_install method to pkg.rs ([`feb5116`](https://github.com/AOSC-Dev/oma/commit/feb5116fda77e1e67e68853adfae7c2189fa77c9))
    - Adjust code stract ([`5e24eac`](https://github.com/AOSC-Dev/oma/commit/5e24eaccd5828d51439ef7d18debf5f192559e46))
    - Improve find unmet dep logic ([`64fedc8`](https://github.com/AOSC-Dev/oma/commit/64fedc84eb38d0640239ea559b67110547aa63be))
    - Do not display user abort op in find_unmet dep method ([`c7fe71c`](https://github.com/AOSC-Dev/oma/commit/c7fe71c710e8284549913abbf16eb81be2c38a43))
    - Add unmet dependency error output ([`c038023`](https://github.com/AOSC-Dev/oma/commit/c0380232311a6ee7871ddb17d8b6f396831a34e3))
    - Use Lazy<Writer> replaced OnceCell<Writer> ([`15b0228`](https://github.com/AOSC-Dev/oma/commit/15b02280f87894e3c0367cda82b03e1f629f22ee))
</details>

## v0.1.0-alpha.12 (2023-03-19)

<csr-id-3864bf049d5dd1871d2eb3ed2d437529249f5532/>
<csr-id-717685f8ca5301d01a5ed493b64d75cfc4dd6edf/>
<csr-id-7361f3f1bb04c027b46dfcbdbf1ea20ef2304e90/>
<csr-id-bd1b4542b32a0261e20220d8e013eb3baca13ec5/>
<csr-id-f828910e466a282b194d82f833954d46a5736a06/>
<csr-id-63355e40544b1ae8fd6741dda9ecd1f412bf0c03/>
<csr-id-97f8c985bed1c5615d16009ae4deb45339d5ba9e/>
<csr-id-ac1b745bdf1e3c7573e8fc7b2ac8356a92ad9c82/>
<csr-id-9431bfc5dead6109a5432c89cb49afe014f68f60/>
<csr-id-fd75161f7445829ef7757342eb290328b00bef26/>
<csr-id-69055f2ab5a43d8691d675203343a6eb41b0fd9b/>
<csr-id-72f274938a65398e3fb40fe8be3cfc37d4eb6303/>
<csr-id-538bd24f670ea4b8d89480691b3757a34efc8ad5/>
<csr-id-a3838cb3522da32f7c8eb6fa26d792609765f3cc/>
<csr-id-471d21899858cdd5e07cbe6ab2231a8fe36ae4e1/>
<csr-id-44668bf051abdde163e2a7661e61cb0520b121a8/>
<csr-id-06ee6f61df9ac195cea1c63b7d78f647e5361c87/>
<csr-id-fc240c97d13ecfcfe619ca0cb964b0bdb2b12f65/>
<csr-id-453f08050dd3cce2bfb81fbc9e663b02895d12b7/>
<csr-id-eceba98e0caa91323b3e9b613ee2917975f56e35/>

### Other

 - <csr-id-3864bf049d5dd1871d2eb3ed2d437529249f5532/> 0.1.0-alpha.12
 - <csr-id-717685f8ca5301d01a5ed493b64d75cfc4dd6edf/> adjust log format
   - Also update deps
 - <csr-id-7361f3f1bb04c027b46dfcbdbf1ea20ef2304e90/> use cargo clippy
 - <csr-id-bd1b4542b32a0261e20220d8e013eb3baca13ec5/> use once_cell replaced lazy_static
 - <csr-id-f828910e466a282b194d82f833954d46a5736a06/> log format adjust
 - <csr-id-63355e40544b1ae8fd6741dda9ecd1f412bf0c03/> fix install loop
 - <csr-id-97f8c985bed1c5615d16009ae4deb45339d5ba9e/> rewrite log write
 - <csr-id-ac1b745bdf1e3c7573e8fc7b2ac8356a92ad9c82/> set log filename as history
 - <csr-id-9431bfc5dead6109a5432c89cb49afe014f68f60/> add oma log feature ...
   Write oma log to /var/log/oma/oma-history.log
 - <csr-id-fd75161f7445829ef7757342eb290328b00bef26/> improve remove table ui statement
 - <csr-id-69055f2ab5a43d8691d675203343a6eb41b0fd9b/> adjust upgrade table color again
 - <csr-id-72f274938a65398e3fb40fe8be3cfc37d4eb6303/> fix pending ui upgrade/install style
 - <csr-id-538bd24f670ea4b8d89480691b3757a34efc8ad5/> improve pending ui style ...
   - Fix new line on install/reinstall
   - Fix upgrade color
 - <csr-id-a3838cb3522da32f7c8eb6fa26d792609765f3cc/> update TODO
 - <csr-id-471d21899858cdd5e07cbe6ab2231a8fe36ae4e1/> remove redundant reqwest error handle
 - <csr-id-44668bf051abdde163e2a7661e61cb0520b121a8/> improve 'download' method logic ...
   ... Remove redundant get requests
 - <csr-id-06ee6f61df9ac195cea1c63b7d78f647e5361c87/> add some ailurus
 - <csr-id-fc240c97d13ecfcfe619ca0cb964b0bdb2b12f65/> code clean up
 - <csr-id-453f08050dd3cce2bfb81fbc9e663b02895d12b7/> use bouncingBall spinner style
 - <csr-id-eceba98e0caa91323b3e9b613ee2917975f56e35/> improve download InRelease ProgressBar

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 23 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 20 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.12 ([`3864bf0`](https://github.com/AOSC-Dev/oma/commit/3864bf049d5dd1871d2eb3ed2d437529249f5532))
    - Adjust log format ([`717685f`](https://github.com/AOSC-Dev/oma/commit/717685f8ca5301d01a5ed493b64d75cfc4dd6edf))
    - Use cargo clippy ([`7361f3f`](https://github.com/AOSC-Dev/oma/commit/7361f3f1bb04c027b46dfcbdbf1ea20ef2304e90))
    - Use once_cell replaced lazy_static ([`bd1b454`](https://github.com/AOSC-Dev/oma/commit/bd1b4542b32a0261e20220d8e013eb3baca13ec5))
    - Main, action: add oma 'log' subcommand ([`dce41a7`](https://github.com/AOSC-Dev/oma/commit/dce41a7597180244f25187f55ef2324daa1124d3))
    - Log format adjust ([`f828910`](https://github.com/AOSC-Dev/oma/commit/f828910e466a282b194d82f833954d46a5736a06))
    - Fix install loop ([`63355e4`](https://github.com/AOSC-Dev/oma/commit/63355e40544b1ae8fd6741dda9ecd1f412bf0c03))
    - Rewrite log write ([`97f8c98`](https://github.com/AOSC-Dev/oma/commit/97f8c985bed1c5615d16009ae4deb45339d5ba9e))
    - Set log filename as history ([`ac1b745`](https://github.com/AOSC-Dev/oma/commit/ac1b745bdf1e3c7573e8fc7b2ac8356a92ad9c82))
    - Add oma log feature ... ([`9431bfc`](https://github.com/AOSC-Dev/oma/commit/9431bfc5dead6109a5432c89cb49afe014f68f60))
    - Action, main: add oma remove --keep-config argument ([`2ad9b61`](https://github.com/AOSC-Dev/oma/commit/2ad9b616c8aaffbea5067001ad58866fc07ac502))
    - Improve remove table ui statement ([`fd75161`](https://github.com/AOSC-Dev/oma/commit/fd75161f7445829ef7757342eb290328b00bef26))
    - Adjust upgrade table color again ([`69055f2`](https://github.com/AOSC-Dev/oma/commit/69055f2ab5a43d8691d675203343a6eb41b0fd9b))
    - Fix pending ui upgrade/install style ([`72f2749`](https://github.com/AOSC-Dev/oma/commit/72f274938a65398e3fb40fe8be3cfc37d4eb6303))
    - Improve pending ui style ... ([`538bd24`](https://github.com/AOSC-Dev/oma/commit/538bd24f670ea4b8d89480691b3757a34efc8ad5))
    - Update TODO ([`a3838cb`](https://github.com/AOSC-Dev/oma/commit/a3838cb3522da32f7c8eb6fa26d792609765f3cc))
    - Action, main: add 'rdepends' subcommand ([`b338c34`](https://github.com/AOSC-Dev/oma/commit/b338c3414021a99affaaae39dab10b4a333a80c9))
    - Remove redundant reqwest error handle ([`471d218`](https://github.com/AOSC-Dev/oma/commit/471d21899858cdd5e07cbe6ab2231a8fe36ae4e1))
    - Improve 'download' method logic ... ([`44668bf`](https://github.com/AOSC-Dev/oma/commit/44668bf051abdde163e2a7661e61cb0520b121a8))
    - Add some ailurus ([`06ee6f6`](https://github.com/AOSC-Dev/oma/commit/06ee6f61df9ac195cea1c63b7d78f647e5361c87))
    - Code clean up ([`fc240c9`](https://github.com/AOSC-Dev/oma/commit/fc240c97d13ecfcfe619ca0cb964b0bdb2b12f65))
    - Use bouncingBall spinner style ([`453f080`](https://github.com/AOSC-Dev/oma/commit/453f08050dd3cce2bfb81fbc9e663b02895d12b7))
    - Improve download InRelease ProgressBar ([`eceba98`](https://github.com/AOSC-Dev/oma/commit/eceba98e0caa91323b3e9b613ee2917975f56e35))
</details>

## v0.1.0-alpha.11 (2023-03-16)

<csr-id-ee758444c7467afd607bbef7f29ccb7efe412284/>
<csr-id-7c137d68e90b2f83d03305a8ae703d860a2ab1a1/>

### Other

 - <csr-id-ee758444c7467afd607bbef7f29ccb7efe412284/> fix multi key in one cert file error handle (2)
 - <csr-id-7c137d68e90b2f83d03305a8ae703d860a2ab1a1/> fix multi key in one cert file  error handle

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release 0.1.0-alpha.11 ([`5f80ef1`](https://github.com/AOSC-Dev/oma/commit/5f80ef1b06077f63919d8b26e57c1838d87147ed))
    - Fix multi key in one cert file error handle (2) ([`ee75844`](https://github.com/AOSC-Dev/oma/commit/ee758444c7467afd607bbef7f29ccb7efe412284))
    - Fix multi key in one cert file  error handle ([`7c137d6`](https://github.com/AOSC-Dev/oma/commit/7c137d68e90b2f83d03305a8ae703d860a2ab1a1))
</details>

## v0.1.0-alpha.10 (2023-03-16)

<csr-id-0535b85f85e53257c1464b4f87c2736ad54f680c/>
<csr-id-8884e41d50f596f825f9a765b29fdc9ba2dd2008/>
<csr-id-62c0a2f36b05ccafea3f1e96937ec75bc90326e7/>
<csr-id-d2797a0cf8a9942ec84c947e92ffdc98de791e46/>
<csr-id-3a825ee033fa29705cc4d0d09dd6b910085c3007/>
<csr-id-013281f587ac0481e8a61598b544fe50b9b83753/>
<csr-id-ae7fb4c3c1ea19dfc9978236933a354b540e97d7/>
<csr-id-7ea894d28bebe7bbfa99a4a30e1e470aab0a60e8/>
<csr-id-5065a424240d8bd8b964b9753fac7efb0582c9ae/>
<csr-id-c9de9df27acd61a1a91fca765dfadf63ed77e28f/>
<csr-id-a81c4be8b023a1b2146c12d3ae86897f0219a157/>
<csr-id-5d3583346a4e85e7e89cda63e6ec889fbf01777c/>
<csr-id-2fa253c1fadfe55e8637cc0c9f5828021d351e79/>
<csr-id-7ee782ef5f0a45bf82d2f38039e25707fe566b8e/>
<csr-id-5b44b139fd2ba2b5bd8cf85cb9627a9841a3ef13/>
<csr-id-c008b8c921af22aa31645fd23273cb9900730be6/>
<csr-id-b2cff14f4093678e286739237ad2b5e0932812bb/>
<csr-id-7aed08d72501cb8de3d133968d0ba1bb38344a5d/>
<csr-id-a8b65ee6bb60020ad3be1a56019382547dc62be6/>
<csr-id-6e94e6b2b9fab1377f2a21af79c075777dc34b6d/>
<csr-id-5823364d2177b07890fe21a0d3664f2cac2f5658/>
<csr-id-4d600702e04d948f7758a533dfc8323feca8a54e/>
<csr-id-8103ebbe8200983671e26f1599e6439515c6c1ab/>
<csr-id-532a184b8f958d211b671051eda44fc898888d50/>
<csr-id-d1be5839fc6e6fe61df690d21bc929818363dd36/>
<csr-id-824ccaf4a43e6cc2df863fad944a899aa86e319a/>
<csr-id-d9011297c7dd215992e47eea629536aefc2b2c8a/>
<csr-id-9bf4c2fc3d19bf83a0632ca20ac2b75433ca5cb0/>
<csr-id-bc484e40dad02e3965f4d8ffe719a6558b0a5a91/>
<csr-id-714529da1fa6face96e866fc86793b8c21e62abd/>
<csr-id-0f2b95d9f842840f6fb5870157ab265d30622852/>
<csr-id-656444875d5d74864ddda55faff5df7b40cac05d/>
<csr-id-a4fcfac05d6719b43124b4cac5022a115014df19/>
<csr-id-3e21fe2cbc820b911ab915a8eff098047d6a2601/>
<csr-id-7acad788ab7be3c890d325aaf55a88a50ba3f8e1/>
<csr-id-3dc170c3d89f2ea2c73e6fbbaf4b0091a0a7be6f/>
<csr-id-bfbd490691e7458e51d4a47e9167ea1afc501a6c/>
<csr-id-ee544105388ac5d2cad755a640f772df62ee5d2e/>
<csr-id-9b315b9713f49b2d0f615128785e6e6564f5ed25/>
<csr-id-7a3caedeab4634d2dc6e18a102f91fd9ba2eaa8a/>
<csr-id-cfd533dac3f5372f4dbb67262f8f82f821044eec/>
<csr-id-6962e1c589f42b7c01f92f2b414abb7539991e8d/>
<csr-id-5ec77326f4c7531d6e0fead9766327d584c5675a/>
<csr-id-83d7b60ce48111e2c6984f45124749e2507add39/>
<csr-id-7bdf0190216f03590af9f73a0e95159537947bb3/>
<csr-id-177213b5b35a2ac7a35c59272fcc82cdc2516e1c/>

### Other

 - <csr-id-0535b85f85e53257c1464b4f87c2736ad54f680c/> fix multi key in one cert file parser
 - <csr-id-8884e41d50f596f825f9a765b29fdc9ba2dd2008/> use cargo clippy
 - <csr-id-62c0a2f36b05ccafea3f1e96937ec75bc90326e7/> use own debcontrol-rs fork to fix rustc warning
 - <csr-id-d2797a0cf8a9942ec84c947e92ffdc98de791e46/> fix global pb style
 - <csr-id-3a825ee033fa29705cc4d0d09dd6b910085c3007/> add args comment
 - <csr-id-013281f587ac0481e8a61598b544fe50b9b83753/> if no --yes do not set --force-confold
 - <csr-id-ae7fb4c3c1ea19dfc9978236933a354b540e97d7/> add missing key dueto
 - <csr-id-7ea894d28bebe7bbfa99a4a30e1e470aab0a60e8/> support .asc file verify
 - <csr-id-5065a424240d8bd8b964b9753fac7efb0582c9ae/> add apt sources.list signed-by support
 - <csr-id-c9de9df27acd61a1a91fca765dfadf63ed77e28f/> {Yes,Apt}InstallProgress -> OmaAptInstallProgress ...
   ... And default --yes use dpkg --force-confold, and add oma install --force-confnew argument
 - <csr-id-a81c4be8b023a1b2146c12d3ae86897f0219a157/> set global pb steady_tick as 100ms
 - <csr-id-5d3583346a4e85e7e89cda63e6ec889fbf01777c/> optimize update_db logic
 - <csr-id-2fa253c1fadfe55e8637cc0c9f5828021d351e79/> optimize down_package method logic
 - <csr-id-7ee782ef5f0a45bf82d2f38039e25707fe566b8e/> add 'force_yes' argument to apt_handler method
 - <csr-id-5b44b139fd2ba2b5bd8cf85cb9627a9841a3ef13/> make cargo clippy happy
 - <csr-id-c008b8c921af22aa31645fd23273cb9900730be6/> fix install need root tips
 - <csr-id-b2cff14f4093678e286739237ad2b5e0932812bb/> add yes warn
 - <csr-id-7aed08d72501cb8de3d133968d0ba1bb38344a5d/> oma-yes =? oma --yes/-y
 - <csr-id-a8b65ee6bb60020ad3be1a56019382547dc62be6/> try to fix apt automatic install (2)
 - <csr-id-6e94e6b2b9fab1377f2a21af79c075777dc34b6d/> try to fix apt automatic install
 - <csr-id-5823364d2177b07890fe21a0d3664f2cac2f5658/> add yes warn
 - <csr-id-4d600702e04d948f7758a533dfc8323feca8a54e/> fix dead forloop
 - <csr-id-8103ebbe8200983671e26f1599e6439515c6c1ab/> use cargo clippy
 - <csr-id-532a184b8f958d211b671051eda44fc898888d50/> allow yes option
   If binary name (or symlink) is oma-yes, At this point all operations are performed automatically
 - <csr-id-d1be5839fc6e6fe61df690d21bc929818363dd36/> add 'yes' option
 - <csr-id-824ccaf4a43e6cc2df863fad944a899aa86e319a/> move main.rs to bin/oma.rs
 - <csr-id-d9011297c7dd215992e47eea629536aefc2b2c8a/> add oma list argument --upgradable (-u)
   - Do not panic if list is empty
 - <csr-id-9bf4c2fc3d19bf83a0632ca20ac2b75433ca5cb0/> use cargo clippy
 - <csr-id-bc484e40dad02e3965f4d8ffe719a6558b0a5a91/> try to fix ci
 - <csr-id-714529da1fa6face96e866fc86793b8c21e62abd/> try to fix random segfault
 - <csr-id-0f2b95d9f842840f6fb5870157ab265d30622852/> fix oma pick select pos
 - <csr-id-656444875d5d74864ddda55faff5df7b40cac05d/> improve pick version display
 - <csr-id-a4fcfac05d6719b43124b4cac5022a115014df19/> fix pick display wrong branch
 - <csr-id-3e21fe2cbc820b911ab915a8eff098047d6a2601/> fix pick panic
 - <csr-id-7acad788ab7be3c890d325aaf55a88a50ba3f8e1/> pick display branch if version is equal
 - <csr-id-3dc170c3d89f2ea2c73e6fbbaf4b0091a0a7be6f/> improve pick select version
 - <csr-id-bfbd490691e7458e51d4a47e9167ea1afc501a6c/> improve install version select
 - <csr-id-ee544105388ac5d2cad755a640f772df62ee5d2e/> use cargo clippy
 - <csr-id-9b315b9713f49b2d0f615128785e6e6564f5ed25/> fix the version selection problem of the same version but different sources
 - <csr-id-7a3caedeab4634d2dc6e18a102f91fd9ba2eaa8a/> fix local source twice fetch
   - Also fix packages_download fetch local source
 - <csr-id-cfd533dac3f5372f4dbb67262f8f82f821044eec/> fix a typo
 - <csr-id-6962e1c589f42b7c01f92f2b414abb7539991e8d/> update TODO and usage
 - <csr-id-5ec77326f4c7531d6e0fead9766327d584c5675a/> update TODO
 - <csr-id-83d7b60ce48111e2c6984f45124749e2507add39/> add clean subcommand description
 - <csr-id-7bdf0190216f03590af9f73a0e95159537947bb3/> update
 - <csr-id-177213b5b35a2ac7a35c59272fcc82cdc2516e1c/> add 'oma clean' subcommand

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 50 commits contributed to the release over the course of 4 calendar days.
 - 4 days passed between releases.
 - 46 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release 0.1.0-alpha.10 ([`f2eb643`](https://github.com/AOSC-Dev/oma/commit/f2eb6433884ac7b989283297bcd4c805db5c5b7c))
    - Fix multi key in one cert file parser ([`0535b85`](https://github.com/AOSC-Dev/oma/commit/0535b85f85e53257c1464b4f87c2736ad54f680c))
    - Use cargo clippy ([`8884e41`](https://github.com/AOSC-Dev/oma/commit/8884e41d50f596f825f9a765b29fdc9ba2dd2008))
    - Use own debcontrol-rs fork to fix rustc warning ([`62c0a2f`](https://github.com/AOSC-Dev/oma/commit/62c0a2f36b05ccafea3f1e96937ec75bc90326e7))
    - Fix global pb style ([`d2797a0`](https://github.com/AOSC-Dev/oma/commit/d2797a0cf8a9942ec84c947e92ffdc98de791e46))
    - Add args comment ([`3a825ee`](https://github.com/AOSC-Dev/oma/commit/3a825ee033fa29705cc4d0d09dd6b910085c3007))
    - If no --yes do not set --force-confold ([`013281f`](https://github.com/AOSC-Dev/oma/commit/013281f587ac0481e8a61598b544fe50b9b83753))
    - Add missing key dueto ([`ae7fb4c`](https://github.com/AOSC-Dev/oma/commit/ae7fb4c3c1ea19dfc9978236933a354b540e97d7))
    - Support .asc file verify ([`7ea894d`](https://github.com/AOSC-Dev/oma/commit/7ea894d28bebe7bbfa99a4a30e1e470aab0a60e8))
    - Add apt sources.list signed-by support ([`5065a42`](https://github.com/AOSC-Dev/oma/commit/5065a424240d8bd8b964b9753fac7efb0582c9ae))
    - {Yes,Apt}InstallProgress -> OmaAptInstallProgress ... ([`c9de9df`](https://github.com/AOSC-Dev/oma/commit/c9de9df27acd61a1a91fca765dfadf63ed77e28f))
    - Set global pb steady_tick as 100ms ([`a81c4be`](https://github.com/AOSC-Dev/oma/commit/a81c4be8b023a1b2146c12d3ae86897f0219a157))
    - Optimize update_db logic ([`5d35833`](https://github.com/AOSC-Dev/oma/commit/5d3583346a4e85e7e89cda63e6ec889fbf01777c))
    - Optimize down_package method logic ([`2fa253c`](https://github.com/AOSC-Dev/oma/commit/2fa253c1fadfe55e8637cc0c9f5828021d351e79))
    - Download global progress bar use bytes/total_bytes, not speed ([`a3734d0`](https://github.com/AOSC-Dev/oma/commit/a3734d0a3dd954a730f7506e4bf654b1cb3e2e81))
    - Add 'force_yes' argument to apt_handler method ([`7ee782e`](https://github.com/AOSC-Dev/oma/commit/7ee782ef5f0a45bf82d2f38039e25707fe566b8e))
    - Make cargo clippy happy ([`5b44b13`](https://github.com/AOSC-Dev/oma/commit/5b44b139fd2ba2b5bd8cf85cb9627a9841a3ef13))
    - Action, main: add --force-yes option ([`fbcb1d1`](https://github.com/AOSC-Dev/oma/commit/fbcb1d1eb0c130980bb1062a2ea6299a99f7fb89))
    - Fix install need root tips ([`c008b8c`](https://github.com/AOSC-Dev/oma/commit/c008b8c921af22aa31645fd23273cb9900730be6))
    - Add yes warn ([`b2cff14`](https://github.com/AOSC-Dev/oma/commit/b2cff14f4093678e286739237ad2b5e0932812bb))
    - Oma-yes =? oma --yes/-y ([`7aed08d`](https://github.com/AOSC-Dev/oma/commit/7aed08d72501cb8de3d133968d0ba1bb38344a5d))
    - Try to fix apt automatic install (2) ([`a8b65ee`](https://github.com/AOSC-Dev/oma/commit/a8b65ee6bb60020ad3be1a56019382547dc62be6))
    - Try to fix apt automatic install ([`6e94e6b`](https://github.com/AOSC-Dev/oma/commit/6e94e6b2b9fab1377f2a21af79c075777dc34b6d))
    - Add yes warn ([`5823364`](https://github.com/AOSC-Dev/oma/commit/5823364d2177b07890fe21a0d3664f2cac2f5658))
    - Fix dead forloop ([`4d60070`](https://github.com/AOSC-Dev/oma/commit/4d600702e04d948f7758a533dfc8323feca8a54e))
    - Use cargo clippy ([`8103ebb`](https://github.com/AOSC-Dev/oma/commit/8103ebbe8200983671e26f1599e6439515c6c1ab))
    - Allow yes option ([`532a184`](https://github.com/AOSC-Dev/oma/commit/532a184b8f958d211b671051eda44fc898888d50))
    - Add 'yes' option ([`d1be583`](https://github.com/AOSC-Dev/oma/commit/d1be5839fc6e6fe61df690d21bc929818363dd36))
    - Move main.rs to bin/oma.rs ([`824ccaf`](https://github.com/AOSC-Dev/oma/commit/824ccaf4a43e6cc2df863fad944a899aa86e319a))
    - Add oma list argument --upgradable (-u) ([`d901129`](https://github.com/AOSC-Dev/oma/commit/d9011297c7dd215992e47eea629536aefc2b2c8a))
    - Use cargo clippy ([`9bf4c2f`](https://github.com/AOSC-Dev/oma/commit/9bf4c2fc3d19bf83a0632ca20ac2b75433ca5cb0))
    - Try to fix ci ([`bc484e4`](https://github.com/AOSC-Dev/oma/commit/bc484e40dad02e3965f4d8ffe719a6558b0a5a91))
    - Revert "main: try to fix random segfault" ([`8558ff6`](https://github.com/AOSC-Dev/oma/commit/8558ff628377e4c53bfc2d1f3e0e5792fbe1accc))
    - Try to fix random segfault ([`714529d`](https://github.com/AOSC-Dev/oma/commit/714529da1fa6face96e866fc86793b8c21e62abd))
    - Fix oma pick select pos ([`0f2b95d`](https://github.com/AOSC-Dev/oma/commit/0f2b95d9f842840f6fb5870157ab265d30622852))
    - Improve pick version display ([`6564448`](https://github.com/AOSC-Dev/oma/commit/656444875d5d74864ddda55faff5df7b40cac05d))
    - Fix pick display wrong branch ([`a4fcfac`](https://github.com/AOSC-Dev/oma/commit/a4fcfac05d6719b43124b4cac5022a115014df19))
    - Fix pick panic ([`3e21fe2`](https://github.com/AOSC-Dev/oma/commit/3e21fe2cbc820b911ab915a8eff098047d6a2601))
    - Pick display branch if version is equal ([`7acad78`](https://github.com/AOSC-Dev/oma/commit/7acad788ab7be3c890d325aaf55a88a50ba3f8e1))
    - Improve pick select version ([`3dc170c`](https://github.com/AOSC-Dev/oma/commit/3dc170c3d89f2ea2c73e6fbbaf4b0091a0a7be6f))
    - Improve install version select ([`bfbd490`](https://github.com/AOSC-Dev/oma/commit/bfbd490691e7458e51d4a47e9167ea1afc501a6c))
    - Use cargo clippy ([`ee54410`](https://github.com/AOSC-Dev/oma/commit/ee544105388ac5d2cad755a640f772df62ee5d2e))
    - Fix the version selection problem of the same version but different sources ([`9b315b9`](https://github.com/AOSC-Dev/oma/commit/9b315b9713f49b2d0f615128785e6e6564f5ed25))
    - Fix local source twice fetch ([`7a3caed`](https://github.com/AOSC-Dev/oma/commit/7a3caedeab4634d2dc6e18a102f91fd9ba2eaa8a))
    - Fix a typo ([`cfd533d`](https://github.com/AOSC-Dev/oma/commit/cfd533dac3f5372f4dbb67262f8f82f821044eec))
    - Update TODO and usage ([`6962e1c`](https://github.com/AOSC-Dev/oma/commit/6962e1c589f42b7c01f92f2b414abb7539991e8d))
    - Update TODO ([`5ec7732`](https://github.com/AOSC-Dev/oma/commit/5ec77326f4c7531d6e0fead9766327d584c5675a))
    - Add clean subcommand description ([`83d7b60`](https://github.com/AOSC-Dev/oma/commit/83d7b60ce48111e2c6984f45124749e2507add39))
    - Update ([`7bdf019`](https://github.com/AOSC-Dev/oma/commit/7bdf0190216f03590af9f73a0e95159537947bb3))
    - Add 'oma clean' subcommand ([`177213b`](https://github.com/AOSC-Dev/oma/commit/177213b5b35a2ac7a35c59272fcc82cdc2516e1c))
</details>

## v0.1.0-alpha.9 (2023-03-11)

<csr-id-8cae498183782659c2a4dec3f70cc28406d03d88/>
<csr-id-dc18621dbf7ee163c77f608cfb11c63dd8f15950/>
<csr-id-3942a495a6839b7e86c00c454d21d73eed578c47/>
<csr-id-14a0cd28bbbf3e2fcfe4a4e644f6fabe711b594b/>
<csr-id-2f2e883024b18b9329dbebb60dd0f67193c09b2d/>
<csr-id-87726c6f0c8a7bbf6e8aa94b790ffea0bed6f4b1/>
<csr-id-431e1b926e9fb7ff5e44a17b346ac14347a84a56/>
<csr-id-bccae00191a00804a3dedc42313c4baa1d4885b7/>
<csr-id-fb78a8ed0cb5fe33eab90739c55f2a98bbb9d9ed/>
<csr-id-baf917eac0481fe84f8717368d101d93f4c3824b/>
<csr-id-df275e38f5b9eed2eda25adc1f689ebdbff7af6f/>
<csr-id-17c6d8ef7b116e141c578b3cab1ff82c0f623c62/>
<csr-id-681f8382e51aede9f8ca63c16e82df1741435989/>
<csr-id-83da175abc0882f1a4b79eee90df640448caa480/>
<csr-id-8e503f2a000b38597c2c28d2641800bb575bcd51/>
<csr-id-38e5c4d5600d9016fc44964e9cbadcd927b78bf9/>

### Other

 - <csr-id-8cae498183782659c2a4dec3f70cc28406d03d88/> 0.1.0-alpha.9
   - Also update deps and cargo clippy
 - <csr-id-dc18621dbf7ee163c77f608cfb11c63dd8f15950/> comment unuse code
 - <csr-id-3942a495a6839b7e86c00c454d21d73eed578c47/> use fs4 replaced fs2 crate
 - <csr-id-14a0cd28bbbf3e2fcfe4a4e644f6fabe711b594b/> size_checker display human bytes
 - <csr-id-2f2e883024b18b9329dbebb60dd0f67193c09b2d/> try to fix install count == 0 but configure count != 0, oma exit
 - <csr-id-87726c6f0c8a7bbf6e8aa94b790ffea0bed6f4b1/> fix mark hold/unhold pkgs can't unlock dpkg
 - <csr-id-431e1b926e9fb7ff5e44a17b346ac14347a84a56/> move packages_download function from db.rs
 - <csr-id-bccae00191a00804a3dedc42313c4baa1d4885b7/> fix a typo
 - <csr-id-fb78a8ed0cb5fe33eab90739c55f2a98bbb9d9ed/> oma list/show/search if results is empty, return error
 - <csr-id-baf917eac0481fe84f8717368d101d93f4c3824b/> add tips to oma install doesn't not exist pkg
 - <csr-id-df275e38f5b9eed2eda25adc1f689ebdbff7af6f/> add some comment; improve display_result logic
 - <csr-id-17c6d8ef7b116e141c578b3cab1ff82c0f623c62/> add oma install --no-fix-broken and --no-upgrade argument
 - <csr-id-681f8382e51aede9f8ca63c16e82df1741435989/> oma install default fix_broken pkg
 - <csr-id-83da175abc0882f1a4b79eee90df640448caa480/> improve fix-broken feature
 - <csr-id-8e503f2a000b38597c2c28d2641800bb575bcd51/> fix list installed display
 - <csr-id-38e5c4d5600d9016fc44964e9cbadcd927b78bf9/> update usage and fix  typo

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 3 calendar days.
 - 3 days passed between releases.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.9 ([`8cae498`](https://github.com/AOSC-Dev/oma/commit/8cae498183782659c2a4dec3f70cc28406d03d88))
    - Comment unuse code ([`dc18621`](https://github.com/AOSC-Dev/oma/commit/dc18621dbf7ee163c77f608cfb11c63dd8f15950))
    - Use fs4 replaced fs2 crate ([`3942a49`](https://github.com/AOSC-Dev/oma/commit/3942a495a6839b7e86c00c454d21d73eed578c47))
    - Size_checker display human bytes ([`14a0cd2`](https://github.com/AOSC-Dev/oma/commit/14a0cd28bbbf3e2fcfe4a4e644f6fabe711b594b))
    - Try to fix install count == 0 but configure count != 0, oma exit ([`2f2e883`](https://github.com/AOSC-Dev/oma/commit/2f2e883024b18b9329dbebb60dd0f67193c09b2d))
    - Main, action, pkg: add 'oma depends' subcommand ([`988ff6c`](https://github.com/AOSC-Dev/oma/commit/988ff6c65db6b8a52a0d5403195c353872b07e8d))
    - Fix mark hold/unhold pkgs can't unlock dpkg ([`87726c6`](https://github.com/AOSC-Dev/oma/commit/87726c6f0c8a7bbf6e8aa94b790ffea0bed6f4b1))
    - Search, main: support oma search multi keyword ([`86befbd`](https://github.com/AOSC-Dev/oma/commit/86befbde0daab0deb12b18a497a9e593fff0b673))
    - Move packages_download function from db.rs ([`431e1b9`](https://github.com/AOSC-Dev/oma/commit/431e1b926e9fb7ff5e44a17b346ac14347a84a56))
    - Fix a typo ([`bccae00`](https://github.com/AOSC-Dev/oma/commit/bccae00191a00804a3dedc42313c4baa1d4885b7))
    - Oma list/show/search if results is empty, return error ([`fb78a8e`](https://github.com/AOSC-Dev/oma/commit/fb78a8ed0cb5fe33eab90739c55f2a98bbb9d9ed))
    - Add tips to oma install doesn't not exist pkg ([`baf917e`](https://github.com/AOSC-Dev/oma/commit/baf917eac0481fe84f8717368d101d93f4c3824b))
    - Add some comment; improve display_result logic ([`df275e3`](https://github.com/AOSC-Dev/oma/commit/df275e38f5b9eed2eda25adc1f689ebdbff7af6f))
    - Add oma install --no-fix-broken and --no-upgrade argument ([`17c6d8e`](https://github.com/AOSC-Dev/oma/commit/17c6d8ef7b116e141c578b3cab1ff82c0f623c62))
    - Oma install default fix_broken pkg ([`681f838`](https://github.com/AOSC-Dev/oma/commit/681f8382e51aede9f8ca63c16e82df1741435989))
    - Improve fix-broken feature ([`83da175`](https://github.com/AOSC-Dev/oma/commit/83da175abc0882f1a4b79eee90df640448caa480))
    - Fix list installed display ([`8e503f2`](https://github.com/AOSC-Dev/oma/commit/8e503f2a000b38597c2c28d2641800bb575bcd51))
    - Update usage and fix  typo ([`38e5c4d`](https://github.com/AOSC-Dev/oma/commit/38e5c4d5600d9016fc44964e9cbadcd927b78bf9))
</details>

## v0.1.0-alpha.8 (2023-03-08)

<csr-id-5fcc486b75fb3004545197666012ae89d5f2ef79/>
<csr-id-f3f7367e650e05fe12923252374a3b66e5e4a587/>
<csr-id-bb028296f2434d21b033a70433538c7336e3bc0e/>
<csr-id-2672aa86b3c88bbd03b7b41bea48843f08b16062/>
<csr-id-3877d8221fc0d95f8e10961b140d42b2209c70c1/>
<csr-id-0df060b6cb008a7f0d908b74ffb73d3390a683fb/>
<csr-id-0df66e56c5ed025f45e1dbfe147ddacf9f3d1d3d/>

### Other

 - <csr-id-5fcc486b75fb3004545197666012ae89d5f2ef79/> 0.1.0-alpha.8
 - <csr-id-f3f7367e650e05fe12923252374a3b66e5e4a587/> use cargo fmt and clippy
   - Also update deps
 - <csr-id-bb028296f2434d21b033a70433538c7336e3bc0e/> improve cmp logic
 - <csr-id-2672aa86b3c88bbd03b7b41bea48843f08b16062/> if input equal provide name, sort to top
 - <csr-id-3877d8221fc0d95f8e10961b140d42b2209c70c1/> fix install wrong pkg version
 - <csr-id-0df060b6cb008a7f0d908b74ffb73d3390a683fb/> different pages display different tips
 - <csr-id-0df66e56c5ed025f45e1dbfe147ddacf9f3d1d3d/> if height > 1 page, use less to display

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.8 ([`5fcc486`](https://github.com/AOSC-Dev/oma/commit/5fcc486b75fb3004545197666012ae89d5f2ef79))
    - Use cargo fmt and clippy ([`f3f7367`](https://github.com/AOSC-Dev/oma/commit/f3f7367e650e05fe12923252374a3b66e5e4a587))
    - Improve cmp logic ([`bb02829`](https://github.com/AOSC-Dev/oma/commit/bb028296f2434d21b033a70433538c7336e3bc0e))
    - If input equal provide name, sort to top ([`2672aa8`](https://github.com/AOSC-Dev/oma/commit/2672aa86b3c88bbd03b7b41bea48843f08b16062))
    - Fix install wrong pkg version ([`3877d82`](https://github.com/AOSC-Dev/oma/commit/3877d8221fc0d95f8e10961b140d42b2209c70c1))
    - Different pages display different tips ([`0df060b`](https://github.com/AOSC-Dev/oma/commit/0df060b6cb008a7f0d908b74ffb73d3390a683fb))
    - Action, search: fix writeln broken pipe ([`63ff207`](https://github.com/AOSC-Dev/oma/commit/63ff207a4691dfbd7abdbf1bcec72457f282a2bc))
    - If height > 1 page, use less to display ([`0df66e5`](https://github.com/AOSC-Dev/oma/commit/0df66e56c5ed025f45e1dbfe147ddacf9f3d1d3d))
    - Action, cli, main: support oma provides/list-files use pages ([`105e120`](https://github.com/AOSC-Dev/oma/commit/105e12085f1acf6c6a01af7bc17860a8ffcefce9))
</details>

## v0.1.0-alpha.7 (2023-03-05)

<csr-id-4a2af1ceca9ef77f0df2fe4fdd77824b8a836ca0/>
<csr-id-aee0f984b2b15f3f92d64bf54327a6dc42bd773c/>
<csr-id-6d96ef8b3e161cb87fc662e54a6a221c6ce151ec/>
<csr-id-9c161b55025bf8cbf4322390ad94e15f99fa48af/>
<csr-id-ee829d300062c465416d9130ba96d664feb74032/>
<csr-id-8653cad6188a014c5edc7c331e344049ad52bbfc/>
<csr-id-e6eaa163a6bc9588406f5ecdb80d6fe01c6bfd4c/>
<csr-id-ca5b43a45943d8681f81b6d026de58cf5ef94158/>
<csr-id-c18e3c52059ce45bb6957e78c20e104d5865bc4c/>
<csr-id-2e5e068e6bac2ad4cb180402cd5b496d6b45be39/>
<csr-id-18aa896bb7d19c913d1901d77973d29afc6b3731/>
<csr-id-344147f8a054d19ad3365a413d23b1dbaab1329d/>
<csr-id-e8781aae38372f65a116e19da7e289f5282b4d97/>
<csr-id-fcb986c886ebb0b3c32862c92f189992bb226e8c/>
<csr-id-a1713ea7f1a4dd17f93a16a64e0232cd8a36fa0d/>
<csr-id-904452659fa2a81aa27ace515fbde92c261ddcf3/>
<csr-id-3b090cb746d392293f1b8c010b4d4aa7af56f612/>
<csr-id-047a967352020806bf71408f3f8b5dd7063aae0f/>
<csr-id-76b4fda0c7c692360054661519dd6225a7a953b0/>
<csr-id-27d07d4a2e09159ad09686ce1488c44d1d014f3b/>
<csr-id-b1c73e61277fa309fce1cd090a1233895ae9b600/>
<csr-id-cec450e059935b21f5cd259c8b018bcff6fa6a8e/>

### Other

 - <csr-id-4a2af1ceca9ef77f0df2fe4fdd77824b8a836ca0/> 0.1.0-alpha.7
 - <csr-id-aee0f984b2b15f3f92d64bf54327a6dc42bd773c/> use cargo clippy, fmt
   - Also update deps
 - <csr-id-6d96ef8b3e161cb87fc662e54a6a221c6ce151ec/> improve logic
 - <csr-id-9c161b55025bf8cbf4322390ad94e15f99fa48af/> improve local deb install logic
 - <csr-id-ee829d300062c465416d9130ba96d664feb74032/> adjust pb steady_tick and if rg return non-zero code return error
 - <csr-id-8653cad6188a014c5edc7c331e344049ad52bbfc/> add progress spinner output
 - <csr-id-e6eaa163a6bc9588406f5ecdb80d6fe01c6bfd4c/> subcommand 'mark' adjust
   - Add oma mark --help tips
   - Allow multi package mark
 - <csr-id-ca5b43a45943d8681f81b6d026de58cf5ef94158/> if oma remove package does not exist display info
 - <csr-id-c18e3c52059ce45bb6957e78c20e104d5865bc4c/> fetch done display info
 - <csr-id-2e5e068e6bac2ad4cb180402cd5b496d6b45be39/> check root after lock oma to fix need root tips
 - <csr-id-18aa896bb7d19c913d1901d77973d29afc6b3731/> show add display additional version info output
 - <csr-id-344147f8a054d19ad3365a413d23b1dbaab1329d/> add oma show -a argument
 - <csr-id-e8781aae38372f65a116e19da7e289f5282b4d97/> fix another_version info display again
 - <csr-id-fcb986c886ebb0b3c32862c92f189992bb226e8c/> fix another_version info display
 - <csr-id-a1713ea7f1a4dd17f93a16a64e0232cd8a36fa0d/> list add display additional version info output
 - <csr-id-904452659fa2a81aa27ace515fbde92c261ddcf3/> list add automatic status display
 - <csr-id-3b090cb746d392293f1b8c010b4d4aa7af56f612/> oma remove add 'purge' alias
 - <csr-id-047a967352020806bf71408f3f8b5dd7063aae0f/> fix local source metadata fetch
   - Also handle fetch local source output
 - <csr-id-76b4fda0c7c692360054661519dd6225a7a953b0/> cargo clippy
 - <csr-id-27d07d4a2e09159ad09686ce1488c44d1d014f3b/> fix install local pkg version select
 - <csr-id-b1c73e61277fa309fce1cd090a1233895ae9b600/> output package file path when local installation returns an error
 - <csr-id-cec450e059935b21f5cd259c8b018bcff6fa6a8e/> abstract some step to mark_install function

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 22 commits contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 22 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.7 ([`4a2af1c`](https://github.com/AOSC-Dev/oma/commit/4a2af1ceca9ef77f0df2fe4fdd77824b8a836ca0))
    - Use cargo clippy, fmt ([`aee0f98`](https://github.com/AOSC-Dev/oma/commit/aee0f984b2b15f3f92d64bf54327a6dc42bd773c))
    - Improve logic ([`6d96ef8`](https://github.com/AOSC-Dev/oma/commit/6d96ef8b3e161cb87fc662e54a6a221c6ce151ec))
    - Improve local deb install logic ([`9c161b5`](https://github.com/AOSC-Dev/oma/commit/9c161b55025bf8cbf4322390ad94e15f99fa48af))
    - Adjust pb steady_tick and if rg return non-zero code return error ([`ee829d3`](https://github.com/AOSC-Dev/oma/commit/ee829d300062c465416d9130ba96d664feb74032))
    - Add progress spinner output ([`8653cad`](https://github.com/AOSC-Dev/oma/commit/8653cad6188a014c5edc7c331e344049ad52bbfc))
    - Subcommand 'mark' adjust ([`e6eaa16`](https://github.com/AOSC-Dev/oma/commit/e6eaa163a6bc9588406f5ecdb80d6fe01c6bfd4c))
    - If oma remove package does not exist display info ([`ca5b43a`](https://github.com/AOSC-Dev/oma/commit/ca5b43a45943d8681f81b6d026de58cf5ef94158))
    - Fetch done display info ([`c18e3c5`](https://github.com/AOSC-Dev/oma/commit/c18e3c52059ce45bb6957e78c20e104d5865bc4c))
    - Check root after lock oma to fix need root tips ([`2e5e068`](https://github.com/AOSC-Dev/oma/commit/2e5e068e6bac2ad4cb180402cd5b496d6b45be39))
    - Show add display additional version info output ([`18aa896`](https://github.com/AOSC-Dev/oma/commit/18aa896bb7d19c913d1901d77973d29afc6b3731))
    - Add oma show -a argument ([`344147f`](https://github.com/AOSC-Dev/oma/commit/344147f8a054d19ad3365a413d23b1dbaab1329d))
    - Fix another_version info display again ([`e8781aa`](https://github.com/AOSC-Dev/oma/commit/e8781aae38372f65a116e19da7e289f5282b4d97))
    - Fix another_version info display ([`fcb986c`](https://github.com/AOSC-Dev/oma/commit/fcb986c886ebb0b3c32862c92f189992bb226e8c))
    - List add display additional version info output ([`a1713ea`](https://github.com/AOSC-Dev/oma/commit/a1713ea7f1a4dd17f93a16a64e0232cd8a36fa0d))
    - List add automatic status display ([`9044526`](https://github.com/AOSC-Dev/oma/commit/904452659fa2a81aa27ace515fbde92c261ddcf3))
    - Oma remove add 'purge' alias ([`3b090cb`](https://github.com/AOSC-Dev/oma/commit/3b090cb746d392293f1b8c010b4d4aa7af56f612))
    - Fix local source metadata fetch ([`047a967`](https://github.com/AOSC-Dev/oma/commit/047a967352020806bf71408f3f8b5dd7063aae0f))
    - Cargo clippy ([`76b4fda`](https://github.com/AOSC-Dev/oma/commit/76b4fda0c7c692360054661519dd6225a7a953b0))
    - Fix install local pkg version select ([`27d07d4`](https://github.com/AOSC-Dev/oma/commit/27d07d4a2e09159ad09686ce1488c44d1d014f3b))
    - Output package file path when local installation returns an error ([`b1c73e6`](https://github.com/AOSC-Dev/oma/commit/b1c73e61277fa309fce1cd090a1233895ae9b600))
    - Abstract some step to mark_install function ([`cec450e`](https://github.com/AOSC-Dev/oma/commit/cec450e059935b21f5cd259c8b018bcff6fa6a8e))
</details>

## v0.1.0-alpha.6 (2023-03-03)

<csr-id-3d73b8332f3c2406d419c248eb38890b7e3a8930/>
<csr-id-f71e8ca35774cc90f696d3029b2585fc1797ea23/>
<csr-id-345e4efdfc80becb54446f1203b576ad8fc2d85a/>
<csr-id-3231d16356dcc484afaa8629b5585f698da2360d/>

### Other

 - <csr-id-3d73b8332f3c2406d419c248eb38890b7e3a8930/> 0.1.0-alpha.6
 - <csr-id-f71e8ca35774cc90f696d3029b2585fc1797ea23/> remove debug output
 - <csr-id-345e4efdfc80becb54446f1203b576ad8fc2d85a/> fix download need sudo
 - <csr-id-3231d16356dcc484afaa8629b5585f698da2360d/> fix marked upgrade/downgrade check

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 day passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.6 ([`3d73b83`](https://github.com/AOSC-Dev/oma/commit/3d73b8332f3c2406d419c248eb38890b7e3a8930))
    - Remove debug output ([`f71e8ca`](https://github.com/AOSC-Dev/oma/commit/f71e8ca35774cc90f696d3029b2585fc1797ea23))
    - Fix download need sudo ([`345e4ef`](https://github.com/AOSC-Dev/oma/commit/345e4efdfc80becb54446f1203b576ad8fc2d85a))
    - Fix marked upgrade/downgrade check ([`3231d16`](https://github.com/AOSC-Dev/oma/commit/3231d16356dcc484afaa8629b5585f698da2360d))
</details>

## v0.1.0-alpha.5 (2023-03-02)

<csr-id-8922d97c428e4e8d0a630aa275157d2f31b90d23/>
<csr-id-4cfdcf017625363780eb8e0b2be66afd8a3f6ad9/>
<csr-id-3a414ac90d07668987b15812cfacecea681df984/>
<csr-id-f8e82e297f1d5244c03cedc2f137be347abf5604/>
<csr-id-a7862b065f6817c343bbbde40ee7af273ed118ba/>
<csr-id-0f1d7b78cb1e0f1f90e43f054e526385c479b975/>
<csr-id-f441a12f606fe24aa104bc9f677f2bb50965c9bd/>
<csr-id-fcb85010376e8deb0179d1d447cabe8f9f4524ee/>
<csr-id-e1912a322a7fc6f3531b0b087ce7beb572610d48/>
<csr-id-3d64ab6700898fd2f4b82cc80db5e115e54b9da1/>
<csr-id-a2cd0d74e67c86ea937f348a38640c4668309dc2/>
<csr-id-103d9af49716a724f0c5d7d5d17c53f12668219b/>
<csr-id-7495109719c1b261c341709c5a504ca6107db35c/>

### Other

 - <csr-id-8922d97c428e4e8d0a630aa275157d2f31b90d23/> 0.1.0-alpha.5
 - <csr-id-4cfdcf017625363780eb8e0b2be66afd8a3f6ad9/> use cargo clippy
 - <csr-id-3a414ac90d07668987b15812cfacecea681df984/> fix a typo
 - <csr-id-f8e82e297f1d5244c03cedc2f137be347abf5604/> add TODO
 - <csr-id-a7862b065f6817c343bbbde40ee7af273ed118ba/> improve local package reinstall logic
 - <csr-id-0f1d7b78cb1e0f1f90e43f054e526385c479b975/> support reinstall local package
 - <csr-id-f441a12f606fe24aa104bc9f677f2bb50965c9bd/> fix handle if package depends does not exist
 - <csr-id-fcb85010376e8deb0179d1d447cabe8f9f4524ee/> update dep
 - <csr-id-e1912a322a7fc6f3531b0b087ce7beb572610d48/> try fix ci
   Only this try, ok?
 - <csr-id-3d64ab6700898fd2f4b82cc80db5e115e54b9da1/> add rust templete
 - <csr-id-a2cd0d74e67c86ea937f348a38640c4668309dc2/> add 'oma refresh' tips to tell user can upgradable and auto removable package
 - <csr-id-103d9af49716a724f0c5d7d5d17c53f12668219b/> bump version to 0.1.0-alpha.4
 - <csr-id-7495109719c1b261c341709c5a504ca6107db35c/> use cargo clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release over the course of 3 calendar days.
 - 5 days passed between releases.
 - 13 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - 0.1.0-alpha.5 ([`8922d97`](https://github.com/AOSC-Dev/oma/commit/8922d97c428e4e8d0a630aa275157d2f31b90d23))
    - Use cargo clippy ([`4cfdcf0`](https://github.com/AOSC-Dev/oma/commit/4cfdcf017625363780eb8e0b2be66afd8a3f6ad9))
    - Fix a typo ([`3a414ac`](https://github.com/AOSC-Dev/oma/commit/3a414ac90d07668987b15812cfacecea681df984))
    - Add TODO ([`f8e82e2`](https://github.com/AOSC-Dev/oma/commit/f8e82e297f1d5244c03cedc2f137be347abf5604))
    - Improve local package reinstall logic ([`a7862b0`](https://github.com/AOSC-Dev/oma/commit/a7862b065f6817c343bbbde40ee7af273ed118ba))
    - Support reinstall local package ([`0f1d7b7`](https://github.com/AOSC-Dev/oma/commit/0f1d7b78cb1e0f1f90e43f054e526385c479b975))
    - Fix handle if package depends does not exist ([`f441a12`](https://github.com/AOSC-Dev/oma/commit/f441a12f606fe24aa104bc9f677f2bb50965c9bd))
    - Update dep ([`fcb8501`](https://github.com/AOSC-Dev/oma/commit/fcb85010376e8deb0179d1d447cabe8f9f4524ee))
    - Update README.md ([`7ad4f95`](https://github.com/AOSC-Dev/oma/commit/7ad4f95785fd1463ec7fa764eab62b7937e7795d))
    - Update rust.yml ([`faea0c5`](https://github.com/AOSC-Dev/oma/commit/faea0c56300474c1235143c828284bacf1069653))
    - Try fix ci ([`e1912a3`](https://github.com/AOSC-Dev/oma/commit/e1912a322a7fc6f3531b0b087ce7beb572610d48))
    - Add rust templete ([`3d64ab6`](https://github.com/AOSC-Dev/oma/commit/3d64ab6700898fd2f4b82cc80db5e115e54b9da1))
    - Add 'oma refresh' tips to tell user can upgradable and auto removable package ([`a2cd0d7`](https://github.com/AOSC-Dev/oma/commit/a2cd0d74e67c86ea937f348a38640c4668309dc2))
    - Bump version to 0.1.0-alpha.4 ([`103d9af`](https://github.com/AOSC-Dev/oma/commit/103d9af49716a724f0c5d7d5d17c53f12668219b))
    - Use cargo clippy ([`7495109`](https://github.com/AOSC-Dev/oma/commit/7495109719c1b261c341709c5a504ca6107db35c))
    - Update README.md ([`10b7031`](https://github.com/AOSC-Dev/oma/commit/10b7031d97b57421afe473bcfaf34a6cec5599e6))
    - Update README.md ([`11581bb`](https://github.com/AOSC-Dev/oma/commit/11581bb576a0410bf84d8ba34ceb7f023353fb5c))
    - Action add --reinstall argument for install subcommand ([`c833308`](https://github.com/AOSC-Dev/oma/commit/c8333086d5cb5c48a4ebf607213058b9e5628d3b))
</details>

## v0.1.0-alpha.3 (2023-02-25)

<csr-id-5269b3a778e94b7b94461883ee572783138f26a0/>
<csr-id-fe93a4893ab5acc53238cdc70f0597e5d88e107d/>
<csr-id-34a1c9d71b9aafd6a825aaaf1723adc5fc755ed1/>
<csr-id-3e7b99e38168891bf1ec32ce11d0124df601ce1b/>
<csr-id-79a1387b98365135cf04b14dc703db4dc832acd1/>
<csr-id-0b84a911028dccfc600fff405366e35000102883/>
<csr-id-5c05d54e624ccd306a557b407fc31bb31acec60f/>
<csr-id-06d773f4c90978c1c4e135c476470ca839ece053/>
<csr-id-2e28a7c902c3a2fb79d837625e6ea7d351c51271/>
<csr-id-0f3596776e61d61dd7c89a3c25d248cb57b925da/>
<csr-id-d741253d4f722c5db724712761599d1568de39e8/>
<csr-id-131e723592ae1771694d538320bd5d749b351e4b/>
<csr-id-224baa9afb74d4631645f1382711f1a0ab422d92/>
<csr-id-cebf0830c2c64c861a40b9467aab7f1cc23ac3c7/>
<csr-id-f2ea925ff3d71feabe8cc5b93c617c95250c7db7/>
<csr-id-e9c5ed49884cd04eddb5ceaf1ef51dca765c31fc/>
<csr-id-ce3c905cecec29c8ab78001c1b758d681e94333f/>
<csr-id-05d5fcddd1c7ac5456660b8248b8662f5d3852b6/>
<csr-id-e122fbc9c0148386683bacfb0718589a6a5a768c/>
<csr-id-eba69a38f1f57b790894cab31ec10e8d94b76266/>
<csr-id-11680fd5ab84541e0f996348bdb49109734db451/>
<csr-id-8a7ecd884b319d54297b22ee14e81d693edc4e49/>
<csr-id-c73af01567cb41424d516fd6b042b25751b1a845/>
<csr-id-af00ee77ef5e8d5ddb7cfc6d40948a6f38aaa4d9/>
<csr-id-b93ea53fa7955c821e070dc5f5603b832e318772/>
<csr-id-fe229e1156810e1ef00b5930a5f328002940f729/>
<csr-id-813aa511fda8f0232c4f562ec7fc2b8ec9930092/>
<csr-id-32657b9a326a9d5b93458479266ab40c5f9bf123/>
<csr-id-350835baf02b8fa04d1a71cd7d3fe5531437c342/>
<csr-id-1f67a9d68987a2f99543b88989215a723a8f2c81/>
<csr-id-49082ee51f38e6cf56a68cb24d0e8faed7c6f7c2/>
<csr-id-f97070523b19d16523870c57ff26f5202fdd5474/>
<csr-id-a0aa5ba604fc2928b1d6e17cfecbf8c746c13e70/>
<csr-id-f714c963bbc453c89828bd4513a871a0bdf8aa23/>
<csr-id-7850ce3df097280dcd5d1ddc5527ec8ee00acdb6/>
<csr-id-f7ef7e65608bf17b3aca9fa45279a896314ac437/>
<csr-id-e3e2bd739d9a37256811b1e02196184ba4fc466c/>
<csr-id-7307840cdd446b9daecfbb6475ec138a1efb39cd/>
<csr-id-38e8eaf0a94057d8b149dea7415f3465c94ab638/>
<csr-id-31bdfea72798adffbcc6c893ea6c54138e0b250f/>
<csr-id-c624f5c5d424a47d6770e7f12b83244acecf6ca6/>
<csr-id-82ff44f216985f5b15f43925a2d40d13aa912b3d/>
<csr-id-9cd02f60d81e87a61f1e9ca343196d5ebdd4e30c/>
<csr-id-2c4c9e3a36c0bd17e7e76b9ffc5b4c5e0e7263f3/>
<csr-id-d7a51c71cc16ccbb97399d3ddb4f710d38298166/>
<csr-id-c03d4cc04ea8f7ca4d969445e5a8e89b35c8cb20/>
<csr-id-7d66d7775f1c56ccc4bcd95407df8a232ff5212f/>
<csr-id-b562788198b26ab5ba7cda10bd5a4c6a21819a9e/>
<csr-id-8e63ac6269959f0f97502d06a476e1b7797a391d/>
<csr-id-37f4ff109457765216c7c59c042b108976f149fe/>
<csr-id-a136e82a8d0b386a39ad8d396c36f6afb0122303/>
<csr-id-80ac7ac85368afbd259be7539f11543b47045d41/>
<csr-id-73cde8941210379abf7f2f1b572cfd064ddf8bb0/>
<csr-id-82ecc5f8ca5e8de1719cb6a78583a2803524c5c2/>
<csr-id-17e7c5b1c635c3708284e8f2d5027cf2350d8dd4/>
<csr-id-1dbc93277c75f65ccf19cb40b939c4669ca40bfd/>
<csr-id-ff390988defe99aefdd68c753512ee4b9e206905/>
<csr-id-5bde010f0cbe0e3bec4c9ebcd95b62a48c096a3f/>
<csr-id-37a5b651de67bceda00950201000895f7951c96e/>
<csr-id-d02d073945910501d8d4e7dd4b48e43a39d0db1d/>
<csr-id-be1d647e135f9c754ecc98f84a86a0652b95a7d6/>
<csr-id-5f62f4b921c4a1e649420eece1c4a1e4d10fe5a3/>
<csr-id-006b0602ca42b51edafc48c7b0e2451b3950782e/>
<csr-id-a1d75e183c26717ab98fbadf2430050fa89dff32/>
<csr-id-0a93298be9d5ec3440d614784bd294ee3bf96faf/>
<csr-id-abbb1fc8cc537c1b65e64ec05d09904d754e5f96/>
<csr-id-e5c38a7efcf8573126f745b590ce94c0011932ba/>
<csr-id-6951391f9bfda969f32931c400edf870d4563fa4/>
<csr-id-de2dc71d12498195cf3b557c061cb241f0233238/>
<csr-id-465afa7355bbe4c665796df5ae39f906b5997954/>
<csr-id-e9838282ae03398f1c14aaf880b11dfb5a421cf7/>
<csr-id-c6d4f2dfed897c077a3c0ae2aea93507f44b928e/>
<csr-id-9d30ed53fccaff7fba050179e319ef9f180d7ad6/>
<csr-id-e00ea67ad6320937bc97914fca561c8d2af4f0b4/>
<csr-id-a047d802569d054c1b65af655fdd6734c0d8b266/>
<csr-id-bc79fe397873c860876dfd28b185eb9ed879e756/>
<csr-id-80a3127af00e1d240855969e5a1653555d6dbab8/>
<csr-id-85b5aff00be3705bf658a09a45c31d24d4a3837b/>
<csr-id-5a4eefb38f61b51ef1dd026f0c129b7cd0066020/>
<csr-id-dc4cf61e51354b2f91a8f0b30f297a7e6b199f92/>
<csr-id-892ce4c69e7778fdff6c6905e2eb42b5d0b10177/>
<csr-id-7861f8b4f2465d928f072527141893e6cafd59b2/>
<csr-id-4a5009138cbe7805b2797f85c305a8b87ef236ac/>
<csr-id-84c4591a454b6e11c503c2cf473df0b0e9f3edf5/>
<csr-id-6270f2758dda2c8e1325ec1798bc8147089bf35f/>
<csr-id-935a2f19a9129a7ee5f499e7f62c3895c158e1ef/>
<csr-id-d4d989b374705cda90987ce11f2ea80649b5bafa/>
<csr-id-f66528dbf97dda74598984d4ae3532865b55223e/>
<csr-id-1d14a40f141bd0d6c37c9edfeb024e2c4cc307b0/>
<csr-id-897768954112b2ef36a9e841550a4cd3598b6b0b/>
<csr-id-9ed8f895bb23713bc637d39fa5946666c7ee1614/>
<csr-id-a606e361c9e2f6fefe977eef5ce51b45dccc9efc/>
<csr-id-300f311b293a7dddb481673c3d112af4a4f84b78/>
<csr-id-b0142fb66b327fe1f631d5c216d978eca0f0bb96/>
<csr-id-00467685a532424d82cb4c0de6262ba3e98c3f2d/>
<csr-id-71b48c2015f481f452f0953e0896402b3895ad78/>
<csr-id-81476054159b3b9cc935393b642b02671eb8b192/>
<csr-id-c2bd3eb3ccc2fc43bba42a77602f4200bc3005d6/>
<csr-id-a92aab480c1c05328acae9fe55016f0aff7c34dc/>
<csr-id-3cb03aae0e36238b157b12bb088abae37e66c016/>
<csr-id-1b683a9982aaa0bfb1120905fcdfa155879cd981/>
<csr-id-92cb689dc9159b74324c38fdad99aae72f38e9ad/>
<csr-id-2a114455a39402a5a77149eba747a07aff0c09de/>
<csr-id-59d47a77f29e8cbd3854333993124ce23839dca4/>
<csr-id-11296f38e6b30470f7339e7eb5e0c36e0d6b19c6/>
<csr-id-a5eb1f11810f18f05d339d54165dadfdab422eba/>
<csr-id-54e6e7f373f9c3e966be16dbc94563ad260d948c/>
<csr-id-0a8231e6c65fdf3f5dbabe1f47b45af307ab3f94/>
<csr-id-db0a78a4d99961e80bc73ae7f0b9afe1d644f842/>
<csr-id-57899403ca491166c9a695ad50a74fca24fa0358/>
<csr-id-4c9930b49dbcc0c88868bf9c33f959d29b4cb6a5/>
<csr-id-023080f6ba870a461a07ef70101d969efe342d2b/>
<csr-id-4f4b0af7d0fc22aa64d54cc4eff26f96470ef3ea/>
<csr-id-0eb592c20f4e29e96db3559ce1bb9e42256951e0/>
<csr-id-e68970b2b3002988b462d61ef60b98402da1f47a/>
<csr-id-63477bf49058f7455bd54e0d840bf915a36531c8/>
<csr-id-14a2d050f193ba0533969eabd282259dfe2b111a/>
<csr-id-02b919bb33d375a51b27f360e7681860382d1de9/>
<csr-id-45d3de844455d2a09adff33f7dc7b35fde0fa919/>
<csr-id-6d0168a65d187f4d39ca7b95fd7991107a74fa63/>
<csr-id-86066fe7938332c1170d7786364e42b38827772c/>
<csr-id-2cdb7c8ec64128caef2854f6f5c8734605514539/>
<csr-id-154ccb7ca80060e68702beeb302e1b6a0cb02ffc/>
<csr-id-c65b9c58ea3f92cc11f0b4bce10d6e2da8cfc635/>
<csr-id-320ec8904db17c343071b3eb6e3a09eec99cf5ed/>
<csr-id-4b2b92dfe5251161d5fc09f4222ca4df61f501de/>
<csr-id-661818a80e93370d07033ee80536db1b4062c2bd/>
<csr-id-5717149aa40319896414a0dd24057274fe959c4c/>
<csr-id-9967e879e2183efc04bba46a6665c97c601ee6b3/>
<csr-id-b0f5ac9cd0f5e692efb019f07142308b95120201/>
<csr-id-86d6364ff64947a1b0d63c2e035c3c0d93225858/>
<csr-id-e353223aaf2dbb93fad9c1d5cbe175aef832b0f5/>
<csr-id-ea26ae54b9f37eb55aa78bf68a977e8db538d74a/>
<csr-id-b4deefd38d75393ce0d8c2c1cdc3688c19555347/>
<csr-id-bbe0986c66bee67665241983b6df83ea722cf512/>
<csr-id-6a654b01b89bc41e1e6b248a22101a6b9dc0fa47/>
<csr-id-65345aad43e65bd64a4d2fd9c28d2e6d325bd323/>
<csr-id-bd4cc8c567a3dec4510d10d64c2f30a5c7ff9feb/>
<csr-id-e551c8c4df264f6b03bb1a1344c6e84abb2d8bf3/>
<csr-id-acc9b36ada76d564baa40747299076cf4d15fcb6/>
<csr-id-03a7f6256ab6c5efd1baa90096c662c06581b8a8/>
<csr-id-ef3468ec9690712125a61f4a7cf4151e350bc408/>
<csr-id-620abec6dfe41d854a2fbf83eb7ed549a3ef72b6/>
<csr-id-d7a47f22e374472277a67ab85801b010e6073de9/>
<csr-id-3eaffa1a2921586bb1f21cd47c140d12747a6972/>
<csr-id-26c7262b81ce8f9275e02c1bf95d628f3429fa0c/>
<csr-id-56ba0efa8ca337de4a1ac31a6dc7153524109857/>
<csr-id-9d43baed1cd41b1fdf1e1362cf5963017952af5d/>
<csr-id-c48a9f970768364dc4c2ec890dde1056bb23efdc/>
<csr-id-c804d58436cd4f495b6d75dcee8419183d931b83/>
<csr-id-ab1ff4ff44a5aedebbfff8f3cecfe014399a49e1/>
<csr-id-f35200ff50a6af7365296187c1cf6ee2b03f2f95/>
<csr-id-896a41658025f2f9d939091ede7418b6830288fb/>
<csr-id-abda7634e77560f2fc9f761013e16303b4dcc725/>
<csr-id-9d18cd4d6321dd787f28d7a9e72d197f1a5156fa/>
<csr-id-904685f69c9ced1f31c5cb258f355f5c11f5034d/>
<csr-id-ed996daa4a0ad1d3a72ac7b72494bf4997e6e8b6/>
<csr-id-8de611fbc1e242c74a53de2229e9adb2b6689ce6/>
<csr-id-4d75c7b4c7ebbc19f1bb0fe2cfe395ead71aa5b0/>
<csr-id-0e295e8b834603a8bf0b43fa94c8470e8f2a8248/>
<csr-id-53b3010439a7b55051f1b30c27700d247a9ffc2f/>
<csr-id-8757a027ec7fb2a6d0fdc3a1a55ed8ab588ebb23/>
<csr-id-90cb592cc07cd1e1b2f0fcc76952d8735255ae22/>
<csr-id-49fc96d6cfa29423f14945ad21f744d188705b46/>
<csr-id-05993741172c41d9a78e52b9a6beb1df7ef6bc31/>
<csr-id-b23e90dd92db4c481c89bec5a836985b3eb75ea9/>
<csr-id-4896e4a92b9c05c48524849595bae22cfcba4cf4/>
<csr-id-0c9ecc678d9819f23cfd6209b4d33ad0c83ea38e/>
<csr-id-e46d53e87b45b40584e2f4523a7408b3d10758f1/>
<csr-id-34330ecc236e2a85b735bbef2794ee9bdd4a126b/>
<csr-id-44264992a753feba5ab80e95aee6a5a89c861ca6/>
<csr-id-aadf76bf87501f678751f9fbcff38403800e8592/>
<csr-id-19ee335b14a5f4c0cef728ad7a704b4bfcbc4f1f/>
<csr-id-4537a17c3b3cc85d826ff602a044873715afbf5f/>
<csr-id-8d586168561a8b21e0bc7c5771c8f89212a66269/>
<csr-id-cc15fae459ad8cff3505abe20bb66dd7029d3444/>
<csr-id-2be8ef9beec651c6d733961c7ae3fcffbe653f45/>
<csr-id-79db780e333d718244b202f9f1e4e53479d89d80/>
<csr-id-be660cf3eb89fe2339fc753d846539a3df168604/>
<csr-id-749eebf42a2f0727f9e9ed765fa2048df7b35313/>
<csr-id-75ad9a6911e3e8b2115b566cc20224053fae9e3d/>
<csr-id-d7e0541c1eefda41fca43f1178eb1ab345cb2203/>
<csr-id-b6e69ed25af683d624abe88268a5dc7157578d4c/>
<csr-id-a2b921e76ff97d598cc47f9daa3f2c3ce7d15df8/>
<csr-id-94358c81cdefd194473ec751dd321bd164e9c140/>
<csr-id-fa7331b6b86ac3287693c615eaf599ef5130d0de/>
<csr-id-663239ecbe758bbb68beea0d8ff4b9aa6a04763c/>
<csr-id-a7e0b0cb4acc3fca8aa188f97d364d5b2f4d17e0/>
<csr-id-152e8fc9b5c3dfc45c77e97beeca7c760694009b/>
<csr-id-3b40d166bbc1d8cb3ebbfeaa0a0b0853c5f6df66/>
<csr-id-dcdd4357bb88d51d234b76dbeefd87de52f00f7d/>
<csr-id-786d02b6569651433d99be7a84a9d5c5b1869d80/>
<csr-id-1a5c5688b2c746141d848c73d3612f544468f620/>
<csr-id-467a501d425d404259a7d2a8c9023b0d61beeeae/>

### Other

 - <csr-id-5269b3a778e94b7b94461883ee572783138f26a0/> bump version to 0.1.0.alpha.3
 - <csr-id-fe93a4893ab5acc53238cdc70f0597e5d88e107d/> list function improve code style
 - <csr-id-34a1c9d71b9aafd6a825aaaf1723adc5fc755ed1/> fix list installed display logic
 - <csr-id-3e7b99e38168891bf1ec32ce11d0124df601ce1b/> list display package arch
 - <csr-id-79a1387b98365135cf04b14dc703db4dc832acd1/> fix next line output logic
 - <csr-id-0b84a911028dccfc600fff405366e35000102883/> sort output order and add --installed argument
 - <csr-id-5c05d54e624ccd306a557b407fc31bb31acec60f/> fix list preformance and style
 - <csr-id-06d773f4c90978c1c4e135c476470ca839ece053/> use cargo clippy
 - <csr-id-2e28a7c902c3a2fb79d837625e6ea7d351c51271/> handle ctrlc exit status
 - <csr-id-0f3596776e61d61dd7c89a3c25d248cb57b925da/> buml ver to 0.1.0-alpha.2
 - <csr-id-d741253d4f722c5db724712761599d1568de39e8/> fix list display
 - <csr-id-131e723592ae1771694d538320bd5d749b351e4b/> improve flat/non-flat mirror refresh logic again
 - <csr-id-224baa9afb74d4631645f1382711f1a0ab422d92/> improve flat/non-flat mirror refresh logic
 - <csr-id-cebf0830c2c64c861a40b9467aab7f1cc23ac3c7/> fix non-flat local mirror refresh logic
 - <csr-id-f2ea925ff3d71feabe8cc5b93c617c95250c7db7/> remove useless dbg
 - <csr-id-e9c5ed49884cd04eddb5ceaf1ef51dca765c31fc/> fix flat repo refresh logic
 - <csr-id-ce3c905cecec29c8ab78001c1b758d681e94333f/> fix update_db checksum logic
 - <csr-id-05d5fcddd1c7ac5456660b8248b8662f5d3852b6/> use cargo clippy
 - <csr-id-e122fbc9c0148386683bacfb0718589a6a5a768c/> support flat and local mirror
 - <csr-id-eba69a38f1f57b790894cab31ec10e8d94b76266/> fix APT-Source field display ...
   ... TODO: oma list PACKAGE, fight with rustc so looooong QAQ
 - <csr-id-11680fd5ab84541e0f996348bdb49109734db451/> fix area/section/package line
 - <csr-id-8a7ecd884b319d54297b22ee14e81d693edc4e49/> adapt command-not-found subcommand
 - <csr-id-c73af01567cb41424d516fd6b042b25751b1a845/> add 'command-not-found' subcommand
 - <csr-id-af00ee77ef5e8d5ddb7cfc6d40948a6f38aaa4d9/> remove useless char
 - <csr-id-b93ea53fa7955c821e070dc5f5603b832e318772/> fix contents line is pkg_group logic
 - <csr-id-fe229e1156810e1ef00b5930a5f328002940f729/> use anyhow to handle non-apt errors in cache.upgrade
   After this change, oma will not retry 3 times on non-apt errors.
 - <csr-id-813aa511fda8f0232c4f562ec7fc2b8ec9930092/> use thiserror to control retry
 - <csr-id-32657b9a326a9d5b93458479266ab40c5f9bf123/> fix a typo
 - <csr-id-350835baf02b8fa04d1a71cd7d3fe5531437c342/> fix size_checker in chroot
 - <csr-id-1f67a9d68987a2f99543b88989215a723a8f2c81/> check available space before download and installation
 - <csr-id-49082ee51f38e6cf56a68cb24d0e8faed7c6f7c2/> size byte display B -> iB
 - <csr-id-f97070523b19d16523870c57ff26f5202fdd5474/> fix install size calculate display
 - <csr-id-a0aa5ba604fc2928b1d6e17cfecbf8c746c13e70/> rm useless line
 - <csr-id-f714c963bbc453c89828bd4513a871a0bdf8aa23/> add 'mark' command
 - <csr-id-7850ce3df097280dcd5d1ddc5527ec8ee00acdb6/> use cargo clippy
 - <csr-id-f7ef7e65608bf17b3aca9fa45279a896314ac437/> set section bases package as blue color
 - <csr-id-e3e2bd739d9a37256811b1e02196184ba4fc466c/> move unlock step from try_main to main
 - <csr-id-7307840cdd446b9daecfbb6475ec138a1efb39cd/> unlock_oma with has error
 - <csr-id-38e8eaf0a94057d8b149dea7415f3465c94ab638/> fix autoremove step
 - <csr-id-31bdfea72798adffbcc6c893ea6c54138e0b250f/> remove useless line
 - <csr-id-c624f5c5d424a47d6770e7f12b83244acecf6ca6/> move cache.resolve(true) to apt_handle function inner
 - <csr-id-82ff44f216985f5b15f43925a2d40d13aa912b3d/> move cache.resolve(true) to apt_install function inner
 - <csr-id-9cd02f60d81e87a61f1e9ca343196d5ebdd4e30c/> add 'pick' subcommand
 - <csr-id-2c4c9e3a36c0bd17e7e76b9ffc5b4c5e0e7263f3/> add oma install --dbg(--install-dbg) argument
 - <csr-id-d7a51c71cc16ccbb97399d3ddb4f710d38298166/> use search::show_pkgs to get pkg info, improve code style
 - <csr-id-c03d4cc04ea8f7ca4d969445e5a8e89b35c8cb20/> improve upgradable ui style
 - <csr-id-7d66d7775f1c56ccc4bcd95407df8a232ff5212f/> add error output if contents not exist
 - <csr-id-b562788198b26ab5ba7cda10bd5a4c6a21819a9e/> improve output result
 - <csr-id-8e63ac6269959f0f97502d06a476e1b7797a391d/> fix list-files package display
 - <csr-id-37f4ff109457765216c7c59c042b108976f149fe/> use stderr to output info/warn/debug/dueto ...
 - <csr-id-a136e82a8d0b386a39ad8d396c36f6afb0122303/> use rust-apt https://gitlab.com/volian/rust-apt/ newest git
 - <csr-id-80ac7ac85368afbd259be7539f11543b47045d41/> abstract is_root function
 - <csr-id-73cde8941210379abf7f2f1b572cfd064ddf8bb0/> revert update_db feature ...
   After discussion, this feature should remind the user to refresh the source data after a period of time without updates, instead of refreshing it immediately
 - <csr-id-82ecc5f8ca5e8de1719cb6a78583a2803524c5c2/> remove dpkg ctrlc handler ...
   In fact, apt + dpkg itself has a ctrlc handler, and the ctrlc handler of apt + dpkg is more intelligent, and with the powerful recovery capability of apt + dpkg, it is more appropriate to use the apt + dpkg handler
 - <csr-id-17e7c5b1c635c3708284e8f2d5027cf2350d8dd4/> improve lock/unkock logic from szclsya/sasm
 - <csr-id-1dbc93277c75f65ccf19cb40b939c4669ca40bfd/> lock ctrlc in dpkg install
 - <csr-id-ff390988defe99aefdd68c753512ee4b9e206905/> if user run oma provides/list-files, force run update_db
 - <csr-id-5bde010f0cbe0e3bec4c9ebcd95b62a48c096a3f/> if local contents file is empty, run update db
 - <csr-id-37a5b651de67bceda00950201000895f7951c96e/> use regex::escape to replace my escape step
 - <csr-id-d02d073945910501d8d4e7dd4b48e43a39d0db1d/> add dependencies
 - <csr-id-be1d647e135f9c754ecc98f84a86a0652b95a7d6/> add Japanese spelling for Omakase
 - <csr-id-5f62f4b921c4a1e649420eece1c4a1e4d10fe5a3/> add a definition for Omakase
 - <csr-id-006b0602ca42b51edafc48c7b0e2451b3950782e/> update
 - <csr-id-a1d75e183c26717ab98fbadf2430050fa89dff32/> rename to oma, fix grammar
   More fixes to come.
 - <csr-id-0a93298be9d5ec3440d614784bd294ee3bf96faf/> bump version to 0.1.0-alpha.1
 - <csr-id-abbb1fc8cc537c1b65e64ec05d09904d754e5f96/> use cargo clippy
 - <csr-id-e5c38a7efcf8573126f745b590ce94c0011932ba/> fix regex security issue
 - <csr-id-6951391f9bfda969f32931c400edf870d4563fa4/> fix-broken add packages_download step
 - <csr-id-de2dc71d12498195cf3b557c061cb241f0233238/> rename list-file to list-files
 - <csr-id-465afa7355bbe4c665796df5ae39f906b5997954/> fix-broken command add pending operations page
 - <csr-id-e9838282ae03398f1c14aaf880b11dfb5a421cf7/> add fix-broken command
 - <csr-id-c6d4f2dfed897c077a3c0ae2aea93507f44b928e/> improve error return again
 - <csr-id-9d30ed53fccaff7fba050179e319ef9f180d7ad6/> improve error return
 - <csr-id-e00ea67ad6320937bc97914fca561c8d2af4f0b4/> fix rg error return
 - <csr-id-a047d802569d054c1b65af655fdd6734c0d8b266/> rename search-file command to provides
 - <csr-id-bc79fe397873c860876dfd28b185eb9ed879e756/> move root check to need root function
 - <csr-id-80a3127af00e1d240855969e5a1653555d6dbab8/> download command only download package ifself
 - <csr-id-85b5aff00be3705bf658a09a45c31d24d4a3837b/> add 'download' command
 - <csr-id-5a4eefb38f61b51ef1dd026f0c129b7cd0066020/> move update alias to refresh command
 - <csr-id-dc4cf61e51354b2f91a8f0b30f297a7e6b199f92/> fix oma_style_pb in terminal length < 100 run
 - <csr-id-892ce4c69e7778fdff6c6905e2eb42b5d0b10177/> use ripgrep cli to read contents ...
   ... It so fast!
 - <csr-id-7861f8b4f2465d928f072527141893e6cafd59b2/> improve code style again
   ... Frustratingly, it doesn't get any faster
 - <csr-id-4a5009138cbe7805b2797f85c305a8b87ef236ac/> improve code style
 - <csr-id-84c4591a454b6e11c503c2cf473df0b0e9f3edf5/> improve output result
   Remove duplicate lines in file search results; use cargo clippy
 - <csr-id-6270f2758dda2c8e1325ec1798bc8147089bf35f/> improve contents logic
   Use ripgrep to search contents file
 - <csr-id-935a2f19a9129a7ee5f499e7f62c3895c158e1ef/> done, but so slow
 - <csr-id-d4d989b374705cda90987ce11f2ea80649b5bafa/> fix upgradable output
 - <csr-id-f66528dbf97dda74598984d4ae3532865b55223e/> fix a typo
 - <csr-id-1d14a40f141bd0d6c37c9edfeb024e2c4cc307b0/> rust-apt use my fork to fix search/show panic ...
   ... This change has been sent back upstream (https://gitlab.com/volian/rust-apt/-/merge_requests/29), pending merging
 - <csr-id-897768954112b2ef36a9e841550a4cd3598b6b0b/> no need to use indexmap
 - <csr-id-9ed8f895bb23713bc637d39fa5946666c7ee1614/> fix local install again ...
   ... Since rust-apt doesn't have a good way to get the package names from local sources, use debarchive to parse the package names first
 - <csr-id-a606e361c9e2f6fefe977eef5ce51b45dccc9efc/> fix install with branch and version
 - <csr-id-300f311b293a7dddb481673c3d112af4a4f84b78/> improve search ui style
   Thanks szclsya/sasm !
 - <csr-id-b0142fb66b327fe1f631d5c216d978eca0f0bb96/> add search feature
   Thanks old omakase style!
 - <csr-id-00467685a532424d82cb4c0de6262ba3e98c3f2d/> remove useless display version info
 - <csr-id-71b48c2015f481f452f0953e0896402b3895ad78/> add oma show command
 - <csr-id-81476054159b3b9cc935393b642b02671eb8b192/> cargo fmt
 - <csr-id-c2bd3eb3ccc2fc43bba42a77602f4200bc3005d6/> fix libapt get url
   If the package file name contains a + sign, libapt willescape the + in the
   download address to %2b, but apt will end up with a + file name, for
   compatibility oma does the same thing
 - <csr-id-a92aab480c1c05328acae9fe55016f0aff7c34dc/> multi thread download InRelease files
 - <csr-id-3cb03aae0e36238b157b12bb088abae37e66c016/> if local install error display file path
 - <csr-id-1b683a9982aaa0bfb1120905fcdfa155879cd981/> install_handle add comment
 - <csr-id-92cb689dc9159b74324c38fdad99aae72f38e9ad/> fix local install .deb package
 - <csr-id-2a114455a39402a5a77149eba747a07aff0c09de/> add oma info and root check ...
   ... and remove some useless code
 - <csr-id-59d47a77f29e8cbd3854333993124ce23839dca4/> update and set rust-apt to crate version
 - <csr-id-11296f38e6b30470f7339e7eb5e0c36e0d6b19c6/> use info to tell user what package is installed
 - <csr-id-a5eb1f11810f18f05d339d54165dadfdab422eba/> improve install tips ...
   ... if select package version is installed, output xxx VER is installed tips
 - <csr-id-54e6e7f373f9c3e966be16dbc94563ad260d948c/> use cargo clippy
 - <csr-id-0a8231e6c65fdf3f5dbabe1f47b45af307ab3f94/> abstract some step to try_download
 - <csr-id-db0a78a4d99961e80bc73ae7f0b9afe1d644f842/> code clean up again
 - <csr-id-57899403ca491166c9a695ad50a74fca24fa0358/> clean again
 - <csr-id-4c9930b49dbcc0c88868bf9c33f959d29b4cb6a5/> use cargo clippy
 - <csr-id-023080f6ba870a461a07ef70101d969efe342d2b/> code all clean up
 - <csr-id-4f4b0af7d0fc22aa64d54cc4eff26f96470ef3ea/> improve download logic ...
   ... use rust-apt to get download urls
 - <csr-id-0eb592c20f4e29e96db3559ce1bb9e42256951e0/> add install .deb from local
 - <csr-id-e68970b2b3002988b462d61ef60b98402da1f47a/> improve display package version logic
 - <csr-id-63477bf49058f7455bd54e0d840bf915a36531c8/> improve download message
 - <csr-id-14a2d050f193ba0533969eabd282259dfe2b111a/> fix color in non-global bar
 - <csr-id-02b919bb33d375a51b27f360e7681860382d1de9/> fix global bar number color
 - <csr-id-45d3de844455d2a09adff33f7dc7b35fde0fa919/> if error exit code 1
 - <csr-id-6d0168a65d187f4d39ca7b95fd7991107a74fa63/> improve error handle
 - <csr-id-86066fe7938332c1170d7786364e42b38827772c/> add a comment
 - <csr-id-2cdb7c8ec64128caef2854f6f5c8734605514539/> handle file:// or cdrom:// mirror
 - <csr-id-154ccb7ca80060e68702beeb302e1b6a0cb02ffc/> update rust-apt to new git commit
 - <csr-id-c65b9c58ea3f92cc11f0b4bce10d6e2da8cfc635/> improve global progress bar
 - <csr-id-320ec8904db17c343071b3eb6e3a09eec99cf5ed/> use more rust-apt library
 - <csr-id-4b2b92dfe5251161d5fc09f4222ca4df61f501de/> use rust-apt to read all new and old pkg database
 - <csr-id-661818a80e93370d07033ee80536db1b4062c2bd/> improve display result
 - <csr-id-5717149aa40319896414a0dd24057274fe959c4c/> fix remove result wrong issue ...
   ... use protect to protect resolv dep result and use resolver to resolv dep
 - <csr-id-9967e879e2183efc04bba46a6665c97c601ee6b3/> remove useless function
 - <csr-id-b0f5ac9cd0f5e692efb019f07142308b95120201/> fix cargo clippy
 - <csr-id-86d6364ff64947a1b0d63c2e035c3c0d93225858/> use cargo clippy
 - <csr-id-e353223aaf2dbb93fad9c1d5cbe175aef832b0f5/> oma install support glob ...
   ... like oma install kde*
 - <csr-id-ea26ae54b9f37eb55aa78bf68a977e8db538d74a/> fix like oma upgrade fish=3.5.1-1
 - <csr-id-b4deefd38d75393ce0d8c2c1cdc3688c19555347/> fix downgrade color
 - <csr-id-bbe0986c66bee67665241983b6df83ea722cf512/> protect mark_install with oma install PACKAGE/BRANCH
 - <csr-id-6a654b01b89bc41e1e6b248a22101a6b9dc0fa47/> support like oma upgrade meowdict ...
   ... It will do two things, the first thing is `oma upgrade` and the second thing is `oma install meowdict`
 - <csr-id-65345aad43e65bd64a4d2fd9c28d2e6d325bd323/> fix display select package version ...
   ... I didn't know how to get the modified version when traversing apt get_changes(), so I used the some method to store the user-selected version
 - <csr-id-bd4cc8c567a3dec4510d10d64c2f30a5c7ff9feb/> fix package download with version
 - <csr-id-e551c8c4df264f6b03bb1a1344c6e84abb2d8bf3/> use libapt-pkg to check download version
 - <csr-id-acc9b36ada76d564baa40747299076cf4d15fcb6/> pager add download size and disk_size display
 - <csr-id-03a7f6256ab6c5efd1baa90096c662c06581b8a8/> fix packages_download
 - <csr-id-ef3468ec9690712125a61f4a7cf4151e350bc408/> add result display ...
   ...Thanks omakase, output so beautiful
 - <csr-id-620abec6dfe41d854a2fbf83eb7ed549a3ef72b6/> fix checksum eat memory issue ...
   ... Thanks omakase v1 code
 - <csr-id-d7a47f22e374472277a67ab85801b010e6073de9/> add more ouput
 - <csr-id-3eaffa1a2921586bb1f21cd47c140d12747a6972/> add more info output
 - <csr-id-26c7262b81ce8f9275e02c1bf95d628f3429fa0c/> pb display branch
 - <csr-id-56ba0efa8ca337de4a1ac31a6dc7153524109857/> improve pb style
 - <csr-id-9d43baed1cd41b1fdf1e1362cf5963017952af5d/> add fetch database multiprogress
 - <csr-id-c48a9f970768364dc4c2ec890dde1056bb23efdc/> set update feature subcommand name as upgrade ...
   ... and set update, full-upgrade, dist-upgrade as alias
 - <csr-id-c804d58436cd4f495b6d75dcee8419183d931b83/> add global progress bar to global download packages progress
 - <csr-id-ab1ff4ff44a5aedebbfff8f3cecfe014399a49e1/> progressbar add count and len
 - <csr-id-f35200ff50a6af7365296187c1cf6ee2b03f2f95/> set name as oma (Oh My Ailurus)
 - <csr-id-896a41658025f2f9d939091ede7418b6830288fb/> handle pb message if text width > 48
 - <csr-id-abda7634e77560f2fc9f761013e16303b4dcc725/> fix download filename
 - <csr-id-9d18cd4d6321dd787f28d7a9e72d197f1a5156fa/> add download thread limit
 - <csr-id-904685f69c9ced1f31c5cb258f355f5c11f5034d/> use MultiProgress to display download progress
 - <csr-id-ed996daa4a0ad1d3a72ac7b72494bf4997e6e8b6/> add error check
 - <csr-id-8de611fbc1e242c74a53de2229e9adb2b6689ce6/> fix progress bar file size
 - <csr-id-4d75c7b4c7ebbc19f1bb0fe2cfe395ead71aa5b0/> use async to partial download
 - <csr-id-0e295e8b834603a8bf0b43fa94c8470e8f2a8248/> add refresh to only update package database
 - <csr-id-53b3010439a7b55051f1b30c27700d247a9ffc2f/> improve download code logic
 - <csr-id-8757a027ec7fb2a6d0fdc3a1a55ed8ab588ebb23/> fix if /var/lib/apt doesn't not exist
 - <csr-id-90cb592cc07cd1e1b2f0fcc76952d8735255ae22/> learn omakase
 - <csr-id-49fc96d6cfa29423f14945ad21f744d188705b46/> fix a bug, if arch = xxxnoarch
 - <csr-id-05993741172c41d9a78e52b9a6beb1df7ef6bc31/> fix filename to compatible apt download
 - <csr-id-b23e90dd92db4c481c89bec5a836985b3eb75ea9/> add exit code
 - <csr-id-4896e4a92b9c05c48524849595bae22cfcba4cf4/> use cargo clippy
 - <csr-id-0c9ecc678d9819f23cfd6209b4d33ad0c83ea38e/> improve retry mechanism
 - <csr-id-e46d53e87b45b40584e2f4523a7408b3d10758f1/> run cargo clippy
 - <csr-id-34330ecc236e2a85b735bbef2794ee9bdd4a126b/> fix autoremove
 - <csr-id-44264992a753feba5ab80e95aee6a5a89c861ca6/> use clap to handle subcommand
 - <csr-id-aadf76bf87501f678751f9fbcff38403800e8592/> improve install/update feature
   - Add autoremove step to install/update function
 - <csr-id-19ee335b14a5f4c0cef728ad7a704b4bfcbc4f1f/> add remove feature
   - Remove useless file
 - <csr-id-4537a17c3b3cc85d826ff602a044873715afbf5f/> abstract apt install step to apt_install function
 - <csr-id-8d586168561a8b21e0bc7c5771c8f89212a66269/> support apt install fish/stable
 - <csr-id-cc15fae459ad8cff3505abe20bb66dd7029d3444/> fix comment typo
 - <csr-id-2be8ef9beec651c6d733961c7ae3fcffbe653f45/> update and install done
 - <csr-id-79db780e333d718244b202f9f1e4e53479d89d80/> use rust-apt to calculate dep
 - <csr-id-be660cf3eb89fe2339fc753d846539a3df168604/> use debcontrol to replace 8dparser
 - <csr-id-749eebf42a2f0727f9e9ed765fa2048df7b35313/> new, this is User Action Control
 - <csr-id-75ad9a6911e3e8b2115b566cc20224053fae9e3d/> fill of remove and purge feature
 - <csr-id-d7e0541c1eefda41fca43f1178eb1ab345cb2203/> improve abstraction
 - <csr-id-b6e69ed25af683d624abe88268a5dc7157578d4c/> dpkg_executer: retry 3 times to workround dpkg trigger cycle
 - <csr-id-a2b921e76ff97d598cc47f9daa3f2c3ce7d15df8/> all done
 - <csr-id-94358c81cdefd194473ec751dd321bd164e9c140/> add debug info
 - <csr-id-fa7331b6b86ac3287693c615eaf599ef5130d0de/> add apt -s info parse
 - <csr-id-663239ecbe758bbb68beea0d8ff4b9aa6a04763c/> add AptAction::Purge
 - <csr-id-a7e0b0cb4acc3fca8aa188f97d364d5b2f4d17e0/> add apt_calc function
 - <csr-id-152e8fc9b5c3dfc45c77e97beeca7c760694009b/> UpdatePackage add some size field
 - <csr-id-3b40d166bbc1d8cb3ebbfeaa0a0b0853c5f6df66/> UpdatePackage add filename and from field; fix var name mistake
 - <csr-id-dcdd4357bb88d51d234b76dbeefd87de52f00f7d/> handle if apt Installed-Size and dpkg mismatch
 - <csr-id-786d02b6569651433d99be7a84a9d5c5b1869d80/> all done
 - <csr-id-1a5c5688b2c746141d848c73d3612f544468f620/> fill of download package list and contents
 - <csr-id-467a501d425d404259a7d2a8c9023b0d61beeeae/> use vector to put ChecksumItem

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 205 commits contributed to the release over the course of 26 calendar days.
 - 194 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump version to 0.1.0.alpha.3 ([`5269b3a`](https://github.com/AOSC-Dev/oma/commit/5269b3a778e94b7b94461883ee572783138f26a0))
    - List function improve code style ([`fe93a48`](https://github.com/AOSC-Dev/oma/commit/fe93a4893ab5acc53238cdc70f0597e5d88e107d))
    - Fix list installed display logic ([`34a1c9d`](https://github.com/AOSC-Dev/oma/commit/34a1c9d71b9aafd6a825aaaf1723adc5fc755ed1))
    - List display package arch ([`3e7b99e`](https://github.com/AOSC-Dev/oma/commit/3e7b99e38168891bf1ec32ce11d0124df601ce1b))
    - Fix next line output logic ([`79a1387`](https://github.com/AOSC-Dev/oma/commit/79a1387b98365135cf04b14dc703db4dc832acd1))
    - Sort output order and add --installed argument ([`0b84a91`](https://github.com/AOSC-Dev/oma/commit/0b84a911028dccfc600fff405366e35000102883))
    - Fix list preformance and style ([`5c05d54`](https://github.com/AOSC-Dev/oma/commit/5c05d54e624ccd306a557b407fc31bb31acec60f))
    - Revert "Revert "pager: handle ctrlc exit status"" ([`1806e3a`](https://github.com/AOSC-Dev/oma/commit/1806e3a4686489185e269165f4d15ebfac2add00))
    - Revert "pager: handle ctrlc exit status" ([`0852f93`](https://github.com/AOSC-Dev/oma/commit/0852f93afd3167eb383758394c7f0e5f3e17a18c))
    - Use cargo clippy ([`06d773f`](https://github.com/AOSC-Dev/oma/commit/06d773f4c90978c1c4e135c476470ca839ece053))
    - Handle ctrlc exit status ([`2e28a7c`](https://github.com/AOSC-Dev/oma/commit/2e28a7c902c3a2fb79d837625e6ea7d351c51271))
    - Buml ver to 0.1.0-alpha.2 ([`0f35967`](https://github.com/AOSC-Dev/oma/commit/0f3596776e61d61dd7c89a3c25d248cb57b925da))
    - Fix list display ([`d741253`](https://github.com/AOSC-Dev/oma/commit/d741253d4f722c5db724712761599d1568de39e8))
    - Improve flat/non-flat mirror refresh logic again ([`131e723`](https://github.com/AOSC-Dev/oma/commit/131e723592ae1771694d538320bd5d749b351e4b))
    - Improve flat/non-flat mirror refresh logic ([`224baa9`](https://github.com/AOSC-Dev/oma/commit/224baa9afb74d4631645f1382711f1a0ab422d92))
    - Fix non-flat local mirror refresh logic ([`cebf083`](https://github.com/AOSC-Dev/oma/commit/cebf0830c2c64c861a40b9467aab7f1cc23ac3c7))
    - Remove useless dbg ([`f2ea925`](https://github.com/AOSC-Dev/oma/commit/f2ea925ff3d71feabe8cc5b93c617c95250c7db7))
    - Fix flat repo refresh logic ([`e9c5ed4`](https://github.com/AOSC-Dev/oma/commit/e9c5ed49884cd04eddb5ceaf1ef51dca765c31fc))
    - Fix update_db checksum logic ([`ce3c905`](https://github.com/AOSC-Dev/oma/commit/ce3c905cecec29c8ab78001c1b758d681e94333f))
    - Use cargo clippy ([`05d5fcd`](https://github.com/AOSC-Dev/oma/commit/05d5fcddd1c7ac5456660b8248b8662f5d3852b6))
    - Support flat and local mirror ([`e122fbc`](https://github.com/AOSC-Dev/oma/commit/e122fbc9c0148386683bacfb0718589a6a5a768c))
    - Fix APT-Source field display ... ([`eba69a3`](https://github.com/AOSC-Dev/oma/commit/eba69a38f1f57b790894cab31ec10e8d94b76266))
    - Fix area/section/package line ([`11680fd`](https://github.com/AOSC-Dev/oma/commit/11680fd5ab84541e0f996348bdb49109734db451))
    - Correct a typo in search.rs ([`30f720e`](https://github.com/AOSC-Dev/oma/commit/30f720e3cd02b236f3ac2c2a76261dba9f91b694))
    - Adapt command-not-found subcommand ([`8a7ecd8`](https://github.com/AOSC-Dev/oma/commit/8a7ecd884b319d54297b22ee14e81d693edc4e49))
    - Add 'command-not-found' subcommand ([`c73af01`](https://github.com/AOSC-Dev/oma/commit/c73af01567cb41424d516fd6b042b25751b1a845))
    - Remove useless char ([`af00ee7`](https://github.com/AOSC-Dev/oma/commit/af00ee77ef5e8d5ddb7cfc6d40948a6f38aaa4d9))
    - Fix contents line is pkg_group logic ([`b93ea53`](https://github.com/AOSC-Dev/oma/commit/b93ea53fa7955c821e070dc5f5603b832e318772))
    - Use anyhow to handle non-apt errors in cache.upgrade ([`fe229e1`](https://github.com/AOSC-Dev/oma/commit/fe229e1156810e1ef00b5930a5f328002940f729))
    - Use thiserror to control retry ([`813aa51`](https://github.com/AOSC-Dev/oma/commit/813aa511fda8f0232c4f562ec7fc2b8ec9930092))
    - Fix a typo ([`32657b9`](https://github.com/AOSC-Dev/oma/commit/32657b9a326a9d5b93458479266ab40c5f9bf123))
    - Fix size_checker in chroot ([`350835b`](https://github.com/AOSC-Dev/oma/commit/350835baf02b8fa04d1a71cd7d3fe5531437c342))
    - Check available space before download and installation ([`1f67a9d`](https://github.com/AOSC-Dev/oma/commit/1f67a9d68987a2f99543b88989215a723a8f2c81))
    - Size byte display B -> iB ([`49082ee`](https://github.com/AOSC-Dev/oma/commit/49082ee51f38e6cf56a68cb24d0e8faed7c6f7c2))
    - Fix install size calculate display ([`f970705`](https://github.com/AOSC-Dev/oma/commit/f97070523b19d16523870c57ff26f5202fdd5474))
    - Rm useless line ([`a0aa5ba`](https://github.com/AOSC-Dev/oma/commit/a0aa5ba604fc2928b1d6e17cfecbf8c746c13e70))
    - Add 'mark' command ([`f714c96`](https://github.com/AOSC-Dev/oma/commit/f714c963bbc453c89828bd4513a871a0bdf8aa23))
    - Use cargo clippy ([`7850ce3`](https://github.com/AOSC-Dev/oma/commit/7850ce3df097280dcd5d1ddc5527ec8ee00acdb6))
    - Set section bases package as blue color ([`f7ef7e6`](https://github.com/AOSC-Dev/oma/commit/f7ef7e65608bf17b3aca9fa45279a896314ac437))
    - Move unlock step from try_main to main ([`e3e2bd7`](https://github.com/AOSC-Dev/oma/commit/e3e2bd739d9a37256811b1e02196184ba4fc466c))
    - Unlock_oma with has error ([`7307840`](https://github.com/AOSC-Dev/oma/commit/7307840cdd446b9daecfbb6475ec138a1efb39cd))
    - Fix autoremove step ([`38e8eaf`](https://github.com/AOSC-Dev/oma/commit/38e8eaf0a94057d8b149dea7415f3465c94ab638))
    - Remove useless line ([`31bdfea`](https://github.com/AOSC-Dev/oma/commit/31bdfea72798adffbcc6c893ea6c54138e0b250f))
    - Move cache.resolve(true) to apt_handle function inner ([`c624f5c`](https://github.com/AOSC-Dev/oma/commit/c624f5c5d424a47d6770e7f12b83244acecf6ca6))
    - Move cache.resolve(true) to apt_install function inner ([`82ff44f`](https://github.com/AOSC-Dev/oma/commit/82ff44f216985f5b15f43925a2d40d13aa912b3d))
    - Add 'pick' subcommand ([`9cd02f6`](https://github.com/AOSC-Dev/oma/commit/9cd02f60d81e87a61f1e9ca343196d5ebdd4e30c))
    - Add oma install --dbg(--install-dbg) argument ([`2c4c9e3`](https://github.com/AOSC-Dev/oma/commit/2c4c9e3a36c0bd17e7e76b9ffc5b4c5e0e7263f3))
    - Use search::show_pkgs to get pkg info, improve code style ([`d7a51c7`](https://github.com/AOSC-Dev/oma/commit/d7a51c71cc16ccbb97399d3ddb4f710d38298166))
    - Improve upgradable ui style ([`c03d4cc`](https://github.com/AOSC-Dev/oma/commit/c03d4cc04ea8f7ca4d969445e5a8e89b35c8cb20))
    - Add error output if contents not exist ([`7d66d77`](https://github.com/AOSC-Dev/oma/commit/7d66d7775f1c56ccc4bcd95407df8a232ff5212f))
    - Improve output result ([`b562788`](https://github.com/AOSC-Dev/oma/commit/b562788198b26ab5ba7cda10bd5a4c6a21819a9e))
    - Fix list-files package display ([`8e63ac6`](https://github.com/AOSC-Dev/oma/commit/8e63ac6269959f0f97502d06a476e1b7797a391d))
    - Use stderr to output info/warn/debug/dueto ... ([`37f4ff1`](https://github.com/AOSC-Dev/oma/commit/37f4ff109457765216c7c59c042b108976f149fe))
    - Use rust-apt https://gitlab.com/volian/rust-apt/ newest git ([`a136e82`](https://github.com/AOSC-Dev/oma/commit/a136e82a8d0b386a39ad8d396c36f6afb0122303))
    - Abstract is_root function ([`80ac7ac`](https://github.com/AOSC-Dev/oma/commit/80ac7ac85368afbd259be7539f11543b47045d41))
    - Revert update_db feature ... ([`73cde89`](https://github.com/AOSC-Dev/oma/commit/73cde8941210379abf7f2f1b572cfd064ddf8bb0))
    - Remove dpkg ctrlc handler ... ([`82ecc5f`](https://github.com/AOSC-Dev/oma/commit/82ecc5f8ca5e8de1719cb6a78583a2803524c5c2))
    - Improve lock/unkock logic from szclsya/sasm ([`17e7c5b`](https://github.com/AOSC-Dev/oma/commit/17e7c5b1c635c3708284e8f2d5027cf2350d8dd4))
    - Lock ctrlc in dpkg install ([`1dbc932`](https://github.com/AOSC-Dev/oma/commit/1dbc93277c75f65ccf19cb40b939c4669ca40bfd))
    - Action, main: add lock_oma and unlock_oma function to lock/unlock oma in install/remove/upgrade ([`840ac52`](https://github.com/AOSC-Dev/oma/commit/840ac5234a1fa686e1f3f12bbfaa6033ff37e693))
    - If user run oma provides/list-files, force run update_db ([`ff39098`](https://github.com/AOSC-Dev/oma/commit/ff390988defe99aefdd68c753512ee4b9e206905))
    - If local contents file is empty, run update db ([`5bde010`](https://github.com/AOSC-Dev/oma/commit/5bde010f0cbe0e3bec4c9ebcd95b62a48c096a3f))
    - Use regex::escape to replace my escape step ([`37a5b65`](https://github.com/AOSC-Dev/oma/commit/37a5b651de67bceda00950201000895f7951c96e))
    - Add dependencies ([`d02d073`](https://github.com/AOSC-Dev/oma/commit/d02d073945910501d8d4e7dd4b48e43a39d0db1d))
    - Add Japanese spelling for Omakase ([`be1d647`](https://github.com/AOSC-Dev/oma/commit/be1d647e135f9c754ecc98f84a86a0652b95a7d6))
    - Add a definition for Omakase ([`5f62f4b`](https://github.com/AOSC-Dev/oma/commit/5f62f4b921c4a1e649420eece1c4a1e4d10fe5a3))
    - Update ([`006b060`](https://github.com/AOSC-Dev/oma/commit/006b0602ca42b51edafc48c7b0e2451b3950782e))
    - Rename to oma, fix grammar ([`a1d75e1`](https://github.com/AOSC-Dev/oma/commit/a1d75e183c26717ab98fbadf2430050fa89dff32))
    - Bump version to 0.1.0-alpha.1 ([`0a93298`](https://github.com/AOSC-Dev/oma/commit/0a93298be9d5ec3440d614784bd294ee3bf96faf))
    - Use cargo clippy ([`abbb1fc`](https://github.com/AOSC-Dev/oma/commit/abbb1fc8cc537c1b65e64ec05d09904d754e5f96))
    - Fix regex security issue ([`e5c38a7`](https://github.com/AOSC-Dev/oma/commit/e5c38a7efcf8573126f745b590ce94c0011932ba))
    - Fix-broken add packages_download step ([`6951391`](https://github.com/AOSC-Dev/oma/commit/6951391f9bfda969f32931c400edf870d4563fa4))
    - Rename list-file to list-files ([`de2dc71`](https://github.com/AOSC-Dev/oma/commit/de2dc71d12498195cf3b557c061cb241f0233238))
    - Fix-broken command add pending operations page ([`465afa7`](https://github.com/AOSC-Dev/oma/commit/465afa7355bbe4c665796df5ae39f906b5997954))
    - Add fix-broken command ([`e983828`](https://github.com/AOSC-Dev/oma/commit/e9838282ae03398f1c14aaf880b11dfb5a421cf7))
    - Improve error return again ([`c6d4f2d`](https://github.com/AOSC-Dev/oma/commit/c6d4f2dfed897c077a3c0ae2aea93507f44b928e))
    - Improve error return ([`9d30ed5`](https://github.com/AOSC-Dev/oma/commit/9d30ed53fccaff7fba050179e319ef9f180d7ad6))
    - Fix rg error return ([`e00ea67`](https://github.com/AOSC-Dev/oma/commit/e00ea67ad6320937bc97914fca561c8d2af4f0b4))
    - Rename search-file command to provides ([`a047d80`](https://github.com/AOSC-Dev/oma/commit/a047d802569d054c1b65af655fdd6734c0d8b266))
    - Move root check to need root function ([`bc79fe3`](https://github.com/AOSC-Dev/oma/commit/bc79fe397873c860876dfd28b185eb9ed879e756))
    - Download command only download package ifself ([`80a3127`](https://github.com/AOSC-Dev/oma/commit/80a3127af00e1d240855969e5a1653555d6dbab8))
    - Add 'download' command ([`85b5aff`](https://github.com/AOSC-Dev/oma/commit/85b5aff00be3705bf658a09a45c31d24d4a3837b))
    - Move update alias to refresh command ([`5a4eefb`](https://github.com/AOSC-Dev/oma/commit/5a4eefb38f61b51ef1dd026f0c129b7cd0066020))
    - Fix oma_style_pb in terminal length < 100 run ([`dc4cf61`](https://github.com/AOSC-Dev/oma/commit/dc4cf61e51354b2f91a8f0b30f297a7e6b199f92))
    - Use ripgrep cli to read contents ... ([`892ce4c`](https://github.com/AOSC-Dev/oma/commit/892ce4c69e7778fdff6c6905e2eb42b5d0b10177))
    - Improve code style again ([`7861f8b`](https://github.com/AOSC-Dev/oma/commit/7861f8b4f2465d928f072527141893e6cafd59b2))
    - Improve code style ([`4a50091`](https://github.com/AOSC-Dev/oma/commit/4a5009138cbe7805b2797f85c305a8b87ef236ac))
    - Improve output result ([`84c4591`](https://github.com/AOSC-Dev/oma/commit/84c4591a454b6e11c503c2cf473df0b0e9f3edf5))
    - Improve contents logic ([`6270f27`](https://github.com/AOSC-Dev/oma/commit/6270f2758dda2c8e1325ec1798bc8147089bf35f))
    - Done, but so slow ([`935a2f1`](https://github.com/AOSC-Dev/oma/commit/935a2f19a9129a7ee5f499e7f62c3895c158e1ef))
    - Fix upgradable output ([`d4d989b`](https://github.com/AOSC-Dev/oma/commit/d4d989b374705cda90987ce11f2ea80649b5bafa))
    - Fix a typo ([`f66528d`](https://github.com/AOSC-Dev/oma/commit/f66528dbf97dda74598984d4ae3532865b55223e))
    - Rust-apt use my fork to fix search/show panic ... ([`1d14a40`](https://github.com/AOSC-Dev/oma/commit/1d14a40f141bd0d6c37c9edfeb024e2c4cc307b0))
    - No need to use indexmap ([`8977689`](https://github.com/AOSC-Dev/oma/commit/897768954112b2ef36a9e841550a4cd3598b6b0b))
    - Fix local install again ... ([`9ed8f89`](https://github.com/AOSC-Dev/oma/commit/9ed8f895bb23713bc637d39fa5946666c7ee1614))
    - Fix install with branch and version ([`a606e36`](https://github.com/AOSC-Dev/oma/commit/a606e361c9e2f6fefe977eef5ce51b45dccc9efc))
    - Improve search ui style ([`300f311`](https://github.com/AOSC-Dev/oma/commit/300f311b293a7dddb481673c3d112af4a4f84b78))
    - Add search feature ([`b0142fb`](https://github.com/AOSC-Dev/oma/commit/b0142fb66b327fe1f631d5c216d978eca0f0bb96))
    - Remove useless display version info ([`0046768`](https://github.com/AOSC-Dev/oma/commit/00467685a532424d82cb4c0de6262ba3e98c3f2d))
    - Add oma show command ([`71b48c2`](https://github.com/AOSC-Dev/oma/commit/71b48c2015f481f452f0953e0896402b3895ad78))
    - Cargo fmt ([`8147605`](https://github.com/AOSC-Dev/oma/commit/81476054159b3b9cc935393b642b02671eb8b192))
    - Fix libapt get url ([`c2bd3eb`](https://github.com/AOSC-Dev/oma/commit/c2bd3eb3ccc2fc43bba42a77602f4200bc3005d6))
    - Multi thread download InRelease files ([`a92aab4`](https://github.com/AOSC-Dev/oma/commit/a92aab480c1c05328acae9fe55016f0aff7c34dc))
    - If local install error display file path ([`3cb03aa`](https://github.com/AOSC-Dev/oma/commit/3cb03aae0e36238b157b12bb088abae37e66c016))
    - Install_handle add comment ([`1b683a9`](https://github.com/AOSC-Dev/oma/commit/1b683a9982aaa0bfb1120905fcdfa155879cd981))
    - Fix local install .deb package ([`92cb689`](https://github.com/AOSC-Dev/oma/commit/92cb689dc9159b74324c38fdad99aae72f38e9ad))
    - Add oma info and root check ... ([`2a11445`](https://github.com/AOSC-Dev/oma/commit/2a114455a39402a5a77149eba747a07aff0c09de))
    - Update and set rust-apt to crate version ([`59d47a7`](https://github.com/AOSC-Dev/oma/commit/59d47a77f29e8cbd3854333993124ce23839dca4))
    - Correct a typo in download.rs ([`139cdf1`](https://github.com/AOSC-Dev/oma/commit/139cdf14ef7871c765314de927f1ad50c405ea1d))
    - Use info to tell user what package is installed ([`11296f3`](https://github.com/AOSC-Dev/oma/commit/11296f38e6b30470f7339e7eb5e0c36e0d6b19c6))
    - Improve install tips ... ([`a5eb1f1`](https://github.com/AOSC-Dev/oma/commit/a5eb1f11810f18f05d339d54165dadfdab422eba))
    - Use cargo clippy ([`54e6e7f`](https://github.com/AOSC-Dev/oma/commit/54e6e7f373f9c3e966be16dbc94563ad260d948c))
    - Abstract some step to try_download ([`0a8231e`](https://github.com/AOSC-Dev/oma/commit/0a8231e6c65fdf3f5dbabe1f47b45af307ab3f94))
    - Code clean up again ([`db0a78a`](https://github.com/AOSC-Dev/oma/commit/db0a78a4d99961e80bc73ae7f0b9afe1d644f842))
    - Clean again ([`5789940`](https://github.com/AOSC-Dev/oma/commit/57899403ca491166c9a695ad50a74fca24fa0358))
    - Use cargo clippy ([`4c9930b`](https://github.com/AOSC-Dev/oma/commit/4c9930b49dbcc0c88868bf9c33f959d29b4cb6a5))
    - Code all clean up ([`023080f`](https://github.com/AOSC-Dev/oma/commit/023080f6ba870a461a07ef70101d969efe342d2b))
    - Improve download logic ... ([`4f4b0af`](https://github.com/AOSC-Dev/oma/commit/4f4b0af7d0fc22aa64d54cc4eff26f96470ef3ea))
    - Add install .deb from local ([`0eb592c`](https://github.com/AOSC-Dev/oma/commit/0eb592c20f4e29e96db3559ce1bb9e42256951e0))
    - Improve display package version logic ([`e68970b`](https://github.com/AOSC-Dev/oma/commit/e68970b2b3002988b462d61ef60b98402da1f47a))
    - Improve download message ([`63477bf`](https://github.com/AOSC-Dev/oma/commit/63477bf49058f7455bd54e0d840bf915a36531c8))
    - Fix color in non-global bar ([`14a2d05`](https://github.com/AOSC-Dev/oma/commit/14a2d050f193ba0533969eabd282259dfe2b111a))
    - Fix global bar number color ([`02b919b`](https://github.com/AOSC-Dev/oma/commit/02b919bb33d375a51b27f360e7681860382d1de9))
    - If error exit code 1 ([`45d3de8`](https://github.com/AOSC-Dev/oma/commit/45d3de844455d2a09adff33f7dc7b35fde0fa919))
    - Improve error handle ([`6d0168a`](https://github.com/AOSC-Dev/oma/commit/6d0168a65d187f4d39ca7b95fd7991107a74fa63))
    - Add a comment ([`86066fe`](https://github.com/AOSC-Dev/oma/commit/86066fe7938332c1170d7786364e42b38827772c))
    - Handle file:// or cdrom:// mirror ([`2cdb7c8`](https://github.com/AOSC-Dev/oma/commit/2cdb7c8ec64128caef2854f6f5c8734605514539))
    - Update rust-apt to new git commit ([`154ccb7`](https://github.com/AOSC-Dev/oma/commit/154ccb7ca80060e68702beeb302e1b6a0cb02ffc))
    - Improve global progress bar ([`c65b9c5`](https://github.com/AOSC-Dev/oma/commit/c65b9c58ea3f92cc11f0b4bce10d6e2da8cfc635))
    - Action, pager: improve omakase ui ([`7b606f7`](https://github.com/AOSC-Dev/oma/commit/7b606f7bdde90a7e41d28c1ec2242cc1c26825c8))
    - Use more rust-apt library ([`320ec89`](https://github.com/AOSC-Dev/oma/commit/320ec8904db17c343071b3eb6e3a09eec99cf5ed))
    - Use rust-apt to read all new and old pkg database ([`4b2b92d`](https://github.com/AOSC-Dev/oma/commit/4b2b92dfe5251161d5fc09f4222ca4df61f501de))
    - Improve display result ([`661818a`](https://github.com/AOSC-Dev/oma/commit/661818a80e93370d07033ee80536db1b4062c2bd))
    - Fix remove result wrong issue ... ([`5717149`](https://github.com/AOSC-Dev/oma/commit/5717149aa40319896414a0dd24057274fe959c4c))
    - Remove useless function ([`9967e87`](https://github.com/AOSC-Dev/oma/commit/9967e879e2183efc04bba46a6665c97c601ee6b3))
    - Fix cargo clippy ([`b0f5ac9`](https://github.com/AOSC-Dev/oma/commit/b0f5ac9cd0f5e692efb019f07142308b95120201))
    - Use cargo clippy ([`86d6364`](https://github.com/AOSC-Dev/oma/commit/86d6364ff64947a1b0d63c2e035c3c0d93225858))
    - Oma install support glob ... ([`e353223`](https://github.com/AOSC-Dev/oma/commit/e353223aaf2dbb93fad9c1d5cbe175aef832b0f5))
    - Fix like oma upgrade fish=3.5.1-1 ([`ea26ae5`](https://github.com/AOSC-Dev/oma/commit/ea26ae54b9f37eb55aa78bf68a977e8db538d74a))
    - Fix downgrade color ([`b4deefd`](https://github.com/AOSC-Dev/oma/commit/b4deefd38d75393ce0d8c2c1cdc3688c19555347))
    - Protect mark_install with oma install PACKAGE/BRANCH ([`bbe0986`](https://github.com/AOSC-Dev/oma/commit/bbe0986c66bee67665241983b6df83ea722cf512))
    - Support like oma upgrade meowdict ... ([`6a654b0`](https://github.com/AOSC-Dev/oma/commit/6a654b01b89bc41e1e6b248a22101a6b9dc0fa47))
    - Fix display select package version ... ([`65345aa`](https://github.com/AOSC-Dev/oma/commit/65345aad43e65bd64a4d2fd9c28d2e6d325bd323))
    - Fix package download with version ([`bd4cc8c`](https://github.com/AOSC-Dev/oma/commit/bd4cc8c567a3dec4510d10d64c2f30a5c7ff9feb))
    - Use libapt-pkg to check download version ([`e551c8c`](https://github.com/AOSC-Dev/oma/commit/e551c8c4df264f6b03bb1a1344c6e84abb2d8bf3))
    - Pager add download size and disk_size display ([`acc9b36`](https://github.com/AOSC-Dev/oma/commit/acc9b36ada76d564baa40747299076cf4d15fcb6))
    - Fix packages_download ([`03a7f62`](https://github.com/AOSC-Dev/oma/commit/03a7f6256ab6c5efd1baa90096c662c06581b8a8))
    - Add result display ... ([`ef3468e`](https://github.com/AOSC-Dev/oma/commit/ef3468ec9690712125a61f4a7cf4151e350bc408))
    - Fix checksum eat memory issue ... ([`620abec`](https://github.com/AOSC-Dev/oma/commit/620abec6dfe41d854a2fbf83eb7ed549a3ef72b6))
    - Add more ouput ([`d7a47f2`](https://github.com/AOSC-Dev/oma/commit/d7a47f22e374472277a67ab85801b010e6073de9))
    - Add more info output ([`3eaffa1`](https://github.com/AOSC-Dev/oma/commit/3eaffa1a2921586bb1f21cd47c140d12747a6972))
    - Pb display branch ([`26c7262`](https://github.com/AOSC-Dev/oma/commit/26c7262b81ce8f9275e02c1bf95d628f3429fa0c))
    - Improve pb style ([`56ba0ef`](https://github.com/AOSC-Dev/oma/commit/56ba0efa8ca337de4a1ac31a6dc7153524109857))
    - Add fetch database multiprogress ([`9d43bae`](https://github.com/AOSC-Dev/oma/commit/9d43baed1cd41b1fdf1e1362cf5963017952af5d))
    - Set update feature subcommand name as upgrade ... ([`c48a9f9`](https://github.com/AOSC-Dev/oma/commit/c48a9f970768364dc4c2ec890dde1056bb23efdc))
    - Add global progress bar to global download packages progress ([`c804d58`](https://github.com/AOSC-Dev/oma/commit/c804d58436cd4f495b6d75dcee8419183d931b83))
    - Progressbar add count and len ([`ab1ff4f`](https://github.com/AOSC-Dev/oma/commit/ab1ff4ff44a5aedebbfff8f3cecfe014399a49e1))
    - Set name as oma (Oh My Ailurus) ([`f35200f`](https://github.com/AOSC-Dev/oma/commit/f35200ff50a6af7365296187c1cf6ee2b03f2f95))
    - Handle pb message if text width > 48 ([`896a416`](https://github.com/AOSC-Dev/oma/commit/896a41658025f2f9d939091ede7418b6830288fb))
    - Fix download filename ([`abda763`](https://github.com/AOSC-Dev/oma/commit/abda7634e77560f2fc9f761013e16303b4dcc725))
    - Add download thread limit ([`9d18cd4`](https://github.com/AOSC-Dev/oma/commit/9d18cd4d6321dd787f28d7a9e72d197f1a5156fa))
    - Use MultiProgress to display download progress ([`904685f`](https://github.com/AOSC-Dev/oma/commit/904685f69c9ced1f31c5cb258f355f5c11f5034d))
    - Add error check ([`ed996da`](https://github.com/AOSC-Dev/oma/commit/ed996daa4a0ad1d3a72ac7b72494bf4997e6e8b6))
    - Fix progress bar file size ([`8de611f`](https://github.com/AOSC-Dev/oma/commit/8de611fbc1e242c74a53de2229e9adb2b6689ce6))
    - Use async to partial download ([`4d75c7b`](https://github.com/AOSC-Dev/oma/commit/4d75c7b4c7ebbc19f1bb0fe2cfe395ead71aa5b0))
    - Add refresh to only update package database ([`0e295e8`](https://github.com/AOSC-Dev/oma/commit/0e295e8b834603a8bf0b43fa94c8470e8f2a8248))
    - Improve download code logic ([`53b3010`](https://github.com/AOSC-Dev/oma/commit/53b3010439a7b55051f1b30c27700d247a9ffc2f))
    - Fix if /var/lib/apt doesn't not exist ([`8757a02`](https://github.com/AOSC-Dev/oma/commit/8757a027ec7fb2a6d0fdc3a1a55ed8ab588ebb23))
    - Learn omakase ([`90cb592`](https://github.com/AOSC-Dev/oma/commit/90cb592cc07cd1e1b2f0fcc76952d8735255ae22))
    - Fix a bug, if arch = xxxnoarch ([`49fc96d`](https://github.com/AOSC-Dev/oma/commit/49fc96d6cfa29423f14945ad21f744d188705b46))
    - Fix filename to compatible apt download ([`0599374`](https://github.com/AOSC-Dev/oma/commit/05993741172c41d9a78e52b9a6beb1df7ef6bc31))
    - Add exit code ([`b23e90d`](https://github.com/AOSC-Dev/oma/commit/b23e90dd92db4c481c89bec5a836985b3eb75ea9))
    - Use cargo clippy ([`4896e4a`](https://github.com/AOSC-Dev/oma/commit/4896e4a92b9c05c48524849595bae22cfcba4cf4))
    - Improve retry mechanism ([`0c9ecc6`](https://github.com/AOSC-Dev/oma/commit/0c9ecc678d9819f23cfd6209b4d33ad0c83ea38e))
    - Download, formatter, update: add more comment ([`b826165`](https://github.com/AOSC-Dev/oma/commit/b826165bba6b6f61b658d3bf2296a62396ee0c08))
    - Run cargo clippy ([`e46d53e`](https://github.com/AOSC-Dev/oma/commit/e46d53e87b45b40584e2f4523a7408b3d10758f1))
    - Fix autoremove ([`34330ec`](https://github.com/AOSC-Dev/oma/commit/34330ecc236e2a85b735bbef2794ee9bdd4a126b))
    - Use clap to handle subcommand ([`4426499`](https://github.com/AOSC-Dev/oma/commit/44264992a753feba5ab80e95aee6a5a89c861ca6))
    - Improve install/update feature ([`aadf76b`](https://github.com/AOSC-Dev/oma/commit/aadf76bf87501f678751f9fbcff38403800e8592))
    - Add remove feature ([`19ee335`](https://github.com/AOSC-Dev/oma/commit/19ee335b14a5f4c0cef728ad7a704b4bfcbc4f1f))
    - Abstract apt install step to apt_install function ([`4537a17`](https://github.com/AOSC-Dev/oma/commit/4537a17c3b3cc85d826ff602a044873715afbf5f))
    - Support apt install fish/stable ([`8d58616`](https://github.com/AOSC-Dev/oma/commit/8d586168561a8b21e0bc7c5771c8f89212a66269))
    - Fix comment typo ([`cc15fae`](https://github.com/AOSC-Dev/oma/commit/cc15fae459ad8cff3505abe20bb66dd7029d3444))
    - Update and install done ([`2be8ef9`](https://github.com/AOSC-Dev/oma/commit/2be8ef9beec651c6d733961c7ae3fcffbe653f45))
    - Use rust-apt to calculate dep ([`79db780`](https://github.com/AOSC-Dev/oma/commit/79db780e333d718244b202f9f1e4e53479d89d80))
    - Use debcontrol to replace 8dparser ([`be660cf`](https://github.com/AOSC-Dev/oma/commit/be660cf3eb89fe2339fc753d846539a3df168604))
    - New, this is User Action Control ([`749eebf`](https://github.com/AOSC-Dev/oma/commit/749eebf42a2f0727f9e9ed765fa2048df7b35313))
    - Fill of remove and purge feature ([`75ad9a6`](https://github.com/AOSC-Dev/oma/commit/75ad9a6911e3e8b2115b566cc20224053fae9e3d))
    - Improve abstraction ([`d7e0541`](https://github.com/AOSC-Dev/oma/commit/d7e0541c1eefda41fca43f1178eb1ab345cb2203))
    - Dpkg_executer: retry 3 times to workround dpkg trigger cycle ([`b6e69ed`](https://github.com/AOSC-Dev/oma/commit/b6e69ed25af683d624abe88268a5dc7157578d4c))
    - All done ([`a2b921e`](https://github.com/AOSC-Dev/oma/commit/a2b921e76ff97d598cc47f9daa3f2c3ce7d15df8))
    - Download, update: all done ([`a7e5f4d`](https://github.com/AOSC-Dev/oma/commit/a7e5f4de7774a5ee9dcdca67f81c3557cc3ec650))
    - Update, blackbox, verify: add more comment ([`701e8d9`](https://github.com/AOSC-Dev/oma/commit/701e8d991676374dc5a04b9d5059d713e9c66ee0))
    - Add debug info ([`94358c8`](https://github.com/AOSC-Dev/oma/commit/94358c81cdefd194473ec751dd321bd164e9c140))
    - Add apt -s info parse ([`fa7331b`](https://github.com/AOSC-Dev/oma/commit/fa7331b6b86ac3287693c615eaf599ef5130d0de))
    - Add AptAction::Purge ([`663239e`](https://github.com/AOSC-Dev/oma/commit/663239ecbe758bbb68beea0d8ff4b9aa6a04763c))
    - Add apt_calc function ([`a7e0b0c`](https://github.com/AOSC-Dev/oma/commit/a7e0b0cb4acc3fca8aa188f97d364d5b2f4d17e0))
    - UpdatePackage add some size field ([`152e8fc`](https://github.com/AOSC-Dev/oma/commit/152e8fc9b5c3dfc45c77e97beeca7c760694009b))
    - UpdatePackage add filename and from field; fix var name mistake ([`3b40d16`](https://github.com/AOSC-Dev/oma/commit/3b40d166bbc1d8cb3ebbfeaa0a0b0853c5f6df66))
    - Handle if apt Installed-Size and dpkg mismatch ([`dcdd435`](https://github.com/AOSC-Dev/oma/commit/dcdd4357bb88d51d234b76dbeefd87de52f00f7d))
    - All done ([`786d02b`](https://github.com/AOSC-Dev/oma/commit/786d02b6569651433d99be7a84a9d5c5b1869d80))
    - Fill of download package list and contents ([`1a5c568`](https://github.com/AOSC-Dev/oma/commit/1a5c5688b2c746141d848c73d3612f544468f620))
    - Use vector to put ChecksumItem ([`467a501`](https://github.com/AOSC-Dev/oma/commit/467a501d425d404259a7d2a8c9023b0d61beeeae))
    - Init ([`b65db57`](https://github.com/AOSC-Dev/oma/commit/b65db576b86e7ce106119d600cfcfe52260f838b))
    - Initial commit ([`b7652ba`](https://github.com/AOSC-Dev/oma/commit/b7652ba83f650bdc87d19e273566ee6bd88aa78d))
</details>


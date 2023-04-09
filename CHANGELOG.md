# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.24.1 (2023-04-09)

### Chore

 - <csr-id-e70c3aef3d487593494fba25e33483ac7121477c/> update dep crossbeam-channel to 0.5.8

### Documentation

 - <csr-id-86b85ddc51ef22d00ed27ad5854ec4ca4fea7a0e/> update README

### Bug Fixes

 - <csr-id-1781711818be211e532bfd8a1094559361e26d96/> no additional version info tips
 - <csr-id-33f6a8e8cc3fd4ffed16c9ebc5e48343bdacb67b/> pick no_fix_broekn wrong argument name to panic
 - <csr-id-02de21592e80af57c1aea8013b4f600cc2370f88/> reinstall does not in repo version to panic
 - <csr-id-8a61940b26a790427a54b5dee04c16dadf310e1c/> oma dep output wrong grammar

### Refactor

 - <csr-id-08dddaf3882414c4c9b24484b5d36f7d99d48965/> improve list method code style

### Style

 - <csr-id-85deb9fd8b562fffa5dec0e762eb29a559639470/> use cargo clippy to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 1 calendar day.
 - 1 day passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Use cargo clippy to lint code ([`85deb9f`](https://github.com/AOSC-Dev/aoscpt/commit/85deb9fd8b562fffa5dec0e762eb29a559639470))
    - Update dep crossbeam-channel to 0.5.8 ([`e70c3ae`](https://github.com/AOSC-Dev/aoscpt/commit/e70c3aef3d487593494fba25e33483ac7121477c))
    - No additional version info tips ([`1781711`](https://github.com/AOSC-Dev/aoscpt/commit/1781711818be211e532bfd8a1094559361e26d96))
    - Pick no_fix_broekn wrong argument name to panic ([`33f6a8e`](https://github.com/AOSC-Dev/aoscpt/commit/33f6a8e8cc3fd4ffed16c9ebc5e48343bdacb67b))
    - Improve list method code style ([`08dddaf`](https://github.com/AOSC-Dev/aoscpt/commit/08dddaf3882414c4c9b24484b5d36f7d99d48965))
    - Reinstall does not in repo version to panic ([`02de215`](https://github.com/AOSC-Dev/aoscpt/commit/02de21592e80af57c1aea8013b4f600cc2370f88))
    - Oma dep output wrong grammar ([`8a61940`](https://github.com/AOSC-Dev/aoscpt/commit/8a61940b26a790427a54b5dee04c16dadf310e1c))
    - Update README ([`86b85dd`](https://github.com/AOSC-Dev/aoscpt/commit/86b85ddc51ef22d00ed27ad5854ec4ca4fea7a0e))
</details>

## v0.24.0 (2023-04-08)

<csr-id-66fb496cb5ea21f8288e04987c371bd1d2cbee90/>
<csr-id-9a7e556094c415cbadee46ca3a9a0c26d5c947d5/>

### Chore

 - <csr-id-66fb496cb5ea21f8288e04987c371bd1d2cbee90/> update all deps

### Documentation

 - <csr-id-fbe79f756566cbb54d9f8f20522e8be2cfd1b846/> improve help and man document
   - feat: --no-upgrade => --no-refresh and more argument name adjust

### Bug Fixes

 - <csr-id-0b937fee9e9740f1f98537a31652ab9504d98a3c/> fix wrong oma list info display
   `oma list a b` will not display additional version info
 - <csr-id-c99a6e63a0094101bbc3302d3cd367262e42ed1b/> set search arg name as pattern
 - <csr-id-06ea3cb5ba208625f9b4ac503dc231a6604a3341/> fix oma show needs packages argument
 - <csr-id-6e5cd72e262c3cfc68776e1913d341e9e890e720/> fix without dry-run argument subcommand run
 - <csr-id-e55554b3a134d2315c42d3d2fb52270b2d26bd2e/> use PossibleValues to fix oma-mark man document

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
    - Bump oma v0.24.0 ([`14a3d0d`](https://github.com/AOSC-Dev/aoscpt/commit/14a3d0d6a18d4dfe30a524bad2392c351de397bf))
    - Update all deps ([`66fb496`](https://github.com/AOSC-Dev/aoscpt/commit/66fb496cb5ea21f8288e04987c371bd1d2cbee90))
    - Fix wrong oma list info display ([`0b937fe`](https://github.com/AOSC-Dev/aoscpt/commit/0b937fee9e9740f1f98537a31652ab9504d98a3c))
    - Set search arg name as pattern ([`c99a6e6`](https://github.com/AOSC-Dev/aoscpt/commit/c99a6e63a0094101bbc3302d3cd367262e42ed1b))
    - Fix oma show needs packages argument ([`06ea3cb`](https://github.com/AOSC-Dev/aoscpt/commit/06ea3cb5ba208625f9b4ac503dc231a6604a3341))
    - Fix without dry-run argument subcommand run ([`6e5cd72`](https://github.com/AOSC-Dev/aoscpt/commit/6e5cd72e262c3cfc68776e1913d341e9e890e720))
    - Improve setup dry_tun flag logic ([`9a7e556`](https://github.com/AOSC-Dev/aoscpt/commit/9a7e556094c415cbadee46ca3a9a0c26d5c947d5))
    - Use PossibleValues to fix oma-mark man document ([`e55554b`](https://github.com/AOSC-Dev/aoscpt/commit/e55554b3a134d2315c42d3d2fb52270b2d26bd2e))
    - Improve help and man document ([`fbe79f7`](https://github.com/AOSC-Dev/aoscpt/commit/fbe79f756566cbb54d9f8f20522e8be2cfd1b846))
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

 - <csr-id-ddbe2d90c9ae78514e8482eff10e2ce7f50cf021/> oma pick do not autoremove by default
 - <csr-id-11b5c8ce664108f6f35b64e26832629fe29aa9b7/> oma install do not autoremove by default
 - <csr-id-fffcd13eacea342696a3dd50b53c5ee4ece7d11a/> add query packages database spinner
 - <csr-id-abd0d7481ce081623e681cc4d9dcd4a5a8ba2ad3/> add --no-autoremove argument for oma {install,upgrade,remove,pick}
 - <csr-id-b826d028295b12e3491aa0d8d59b6cdb9f047a32/> add cache.get_archives spinner
 - <csr-id-4355b06227b132e627294b75346906bd8575c6bc/> --debug argument now can run without dry-run mode

### Bug Fixes

 - <csr-id-17c1d7f023eba71d500660963ec23940f9a148e2/> fix query database zombie progress bar
 - <csr-id-e568b01446a4f36d53deafb546ecaf104784cb0a/> fix oma pick no_autoremove arg requires
 - <csr-id-df3b53b32ef4081292c6fae0de46b1ccdb0ad0ec/> fix refresh database file exist global bar progress
 - <csr-id-a38a6840cb0628b595ce9a380754fab0d5d71eef/> fix global bar progress percent color

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
    - Bump oma v0.23.0 ([`0a5bdf8`](https://github.com/AOSC-Dev/aoscpt/commit/0a5bdf8a301278b6389547a8c79d9c752918a3d4))
    - Update serde-yaml to 0.9.20 ([`d515455`](https://github.com/AOSC-Dev/aoscpt/commit/d5154552b077bae8744e9b14eae5aff91437bdc0))
    - Improve pending ui detail capitalize logic ([`fdf7f62`](https://github.com/AOSC-Dev/aoscpt/commit/fdf7f624eabb13f0493de60a90c3c393700e5c62))
    - Use cargo-clippy to lint code ([`82e7582`](https://github.com/AOSC-Dev/aoscpt/commit/82e758268836be0751125b6c7bcf629eea89192f))
    - Oma pick do not autoremove by default ([`ddbe2d9`](https://github.com/AOSC-Dev/aoscpt/commit/ddbe2d90c9ae78514e8482eff10e2ce7f50cf021))
    - Oma install do not autoremove by default ([`11b5c8c`](https://github.com/AOSC-Dev/aoscpt/commit/11b5c8ce664108f6f35b64e26832629fe29aa9b7))
    - Fix query database zombie progress bar ([`17c1d7f`](https://github.com/AOSC-Dev/aoscpt/commit/17c1d7f023eba71d500660963ec23940f9a148e2))
    - Set Multiprogress Bar as lazy var ([`119f059`](https://github.com/AOSC-Dev/aoscpt/commit/119f0594d44d938515ac82c20ae1a19d9a5499af))
    - Add query packages database spinner ([`fffcd13`](https://github.com/AOSC-Dev/aoscpt/commit/fffcd13eacea342696a3dd50b53c5ee4ece7d11a))
    - Run cargo clippy to lint code ([`1fef5a4`](https://github.com/AOSC-Dev/aoscpt/commit/1fef5a496acfa29f413bd4ecdcee4fd9ce7f44da))
    - Fix oma pick no_autoremove arg requires ([`e568b01`](https://github.com/AOSC-Dev/aoscpt/commit/e568b01446a4f36d53deafb546ecaf104784cb0a))
    - Fix refresh database file exist global bar progress ([`df3b53b`](https://github.com/AOSC-Dev/aoscpt/commit/df3b53b32ef4081292c6fae0de46b1ccdb0ad0ec))
    - Add --no-autoremove argument for oma {install,upgrade,remove,pick} ([`abd0d74`](https://github.com/AOSC-Dev/aoscpt/commit/abd0d7481ce081623e681cc4d9dcd4a5a8ba2ad3))
    - Add cache.get_archives spinner ([`b826d02`](https://github.com/AOSC-Dev/aoscpt/commit/b826d028295b12e3491aa0d8d59b6cdb9f047a32))
    - Fix global bar progress percent color ([`a38a684`](https://github.com/AOSC-Dev/aoscpt/commit/a38a6840cb0628b595ce9a380754fab0d5d71eef))
    - --debug argument now can run without dry-run mode ([`4355b06`](https://github.com/AOSC-Dev/aoscpt/commit/4355b06227b132e627294b75346906bd8575c6bc))
</details>

## v0.22.0 (2023-04-05)

<csr-id-a87a40d79c10a7acbf2f76ba6e4fa413e206ad46/>
<csr-id-46b75d978dd0e8fe93847086f1040b3a22a1603c/>
<csr-id-05baa26ff7c75c4c103ad5f1b78f53f8b571c52c/>

### Chore

 - <csr-id-a87a40d79c10a7acbf2f76ba6e4fa413e206ad46/> update all deps

### New Features

 - <csr-id-b44035d76b13675b85d6a2c648f6b6ab76eea448/> error output message adjust
 - <csr-id-3bb8b317af8eaacd7a03eb5a0939a15f3449b37e/> if needs run dpkg --configure -a, run it
 - <csr-id-7ae0faca3b79ecbbbf50f4f28e369bbeca4d13fb/> build all subcommand man

### Bug Fixes

 - <csr-id-4df750e8587145d44bca5f112aace690e08c6107/> fix autoremove/non-autoremove pkg pending ui wrong detail

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
    - Bump oma v0.22.0 ([`c8b5139`](https://github.com/AOSC-Dev/aoscpt/commit/c8b513957c644802fa8b88433c7965760666608d))
    - Update all deps ([`a87a40d`](https://github.com/AOSC-Dev/aoscpt/commit/a87a40d79c10a7acbf2f76ba6e4fa413e206ad46))
    - Use cargo-fmt to format code ([`05baa26`](https://github.com/AOSC-Dev/aoscpt/commit/05baa26ff7c75c4c103ad5f1b78f53f8b571c52c))
    - Error output message adjust ([`b44035d`](https://github.com/AOSC-Dev/aoscpt/commit/b44035d76b13675b85d6a2c648f6b6ab76eea448))
    - Fix autoremove/non-autoremove pkg pending ui wrong detail ([`4df750e`](https://github.com/AOSC-Dev/aoscpt/commit/4df750e8587145d44bca5f112aace690e08c6107))
    - If needs run dpkg --configure -a, run it ([`3bb8b31`](https://github.com/AOSC-Dev/aoscpt/commit/3bb8b317af8eaacd7a03eb5a0939a15f3449b37e))
    - Build all subcommand man ([`7ae0fac`](https://github.com/AOSC-Dev/aoscpt/commit/7ae0faca3b79ecbbbf50f4f28e369bbeca4d13fb))
    - Improve capitalize output message logic in apt_handler mehod ([`46b75d9`](https://github.com/AOSC-Dev/aoscpt/commit/46b75d978dd0e8fe93847086f1040b3a22a1603c))
</details>

## v0.21.0 (2023-04-03)

<csr-id-f80adf68ef978cdaee19eb6072ccb4449207c93c/>
<csr-id-b719e78fde46d4fd1b08bb3a87a8b8470e0cd827/>

### Chore

 - <csr-id-f80adf68ef978cdaee19eb6072ccb4449207c93c/> update all deps

### New Features

 - <csr-id-cc6546548e526d88360c0ed0750d4d5953a07c82/> if update dpkg-force-all mode after has broken count, return error
 - <csr-id-d7d0253dbfcd7dbea00d5da68be16e55e3f82577/> if retry 2 times apt has error, go to dpkg-force-all mode

### Bug Fixes

 - <csr-id-8cfc2cf9efd57e8a353a81f985695a24b63f901e/> fix a typo

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
    - Bump oma v0.21.0 ([`5f50847`](https://github.com/AOSC-Dev/aoscpt/commit/5f50847782c13f83c158e1412a2f2d41dbbcf522))
    - Update all deps ([`f80adf6`](https://github.com/AOSC-Dev/aoscpt/commit/f80adf68ef978cdaee19eb6072ccb4449207c93c))
    - Fix a typo ([`8cfc2cf`](https://github.com/AOSC-Dev/aoscpt/commit/8cfc2cf9efd57e8a353a81f985695a24b63f901e))
    - If update dpkg-force-all mode after has broken count, return error ([`cc65465`](https://github.com/AOSC-Dev/aoscpt/commit/cc6546548e526d88360c0ed0750d4d5953a07c82))
    - If retry 2 times apt has error, go to dpkg-force-all mode ([`d7d0253`](https://github.com/AOSC-Dev/aoscpt/commit/d7d0253dbfcd7dbea00d5da68be16e55e3f82577))
    - Use cargo fmt and cargo clippy to lint code ([`b719e78`](https://github.com/AOSC-Dev/aoscpt/commit/b719e78fde46d4fd1b08bb3a87a8b8470e0cd827))
</details>

## v0.20.0 (2023-04-02)

### New Features

 - <csr-id-e7929f3296eb063e189cbb1604bbae35f072e263/> improve progress bar style again
 - <csr-id-d931d46d1eb9397257eb052194723db49e102930/> improve progress bar style
 - <csr-id-e7f9c50150305aa25da0893936a45fd6c80266ef/> improve error message display

### Bug Fixes

 - <csr-id-20ad91331ba0e5f8e35f1a23b975e06f316d3549/> fix /run/lock directory does not exist
 - <csr-id-5e699c2518112fa335cbf40681e9eea59a434456/> fix oma subcommand history run

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
    - Bump oma v0.20.0 ([`e08795a`](https://github.com/AOSC-Dev/aoscpt/commit/e08795ad39b00ec6051ce02e82e95003c176a0c2))
    - Improve progress bar style again ([`e7929f3`](https://github.com/AOSC-Dev/aoscpt/commit/e7929f3296eb063e189cbb1604bbae35f072e263))
    - Correct typos in args.rs ([`acd0ad6`](https://github.com/AOSC-Dev/aoscpt/commit/acd0ad6b0f7c6b91a8c220fd69215bc94f8dd5ea))
    - Improve progress bar style ([`d931d46`](https://github.com/AOSC-Dev/aoscpt/commit/d931d46d1eb9397257eb052194723db49e102930))
    - Fix /run/lock directory does not exist ([`20ad913`](https://github.com/AOSC-Dev/aoscpt/commit/20ad91331ba0e5f8e35f1a23b975e06f316d3549))
    - Fix oma subcommand history run ([`5e699c2`](https://github.com/AOSC-Dev/aoscpt/commit/5e699c2518112fa335cbf40681e9eea59a434456))
    - Improve error message display ([`e7f9c50`](https://github.com/AOSC-Dev/aoscpt/commit/e7f9c50150305aa25da0893936a45fd6c80266ef))
</details>

## v0.19.0 (2023-04-01)

<csr-id-4de171ac886e50ed1496337b513f0935717b5a8f/>
<csr-id-1e8d8ab5d2b8f7955c40945de32838f47606a010/>

### Chore

 - <csr-id-4de171ac886e50ed1496337b513f0935717b5a8f/> update rustix dep

### New Features

 - <csr-id-39e066e3adf8a5f20769e6c083b74238b8c8321b/> add {upgrade,install,fix-broken} subcommand --dpkg-force-all argument

### Bug Fixes

 - <csr-id-13f4bf7fd1dccb1f4b7641f7c6b084dcd20a0d37/> add missing progress bar logic

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
    - Bump oma v0.19.0 ([`6a673d3`](https://github.com/AOSC-Dev/aoscpt/commit/6a673d352453af8b8fd704ad9d9ed69d35aeced8))
    - Update rustix dep ([`4de171a`](https://github.com/AOSC-Dev/aoscpt/commit/4de171ac886e50ed1496337b513f0935717b5a8f))
    - Use cargo-fmt to format code ([`1e8d8ab`](https://github.com/AOSC-Dev/aoscpt/commit/1e8d8ab5d2b8f7955c40945de32838f47606a010))
    - Add {upgrade,install,fix-broken} subcommand --dpkg-force-all argument ([`39e066e`](https://github.com/AOSC-Dev/aoscpt/commit/39e066e3adf8a5f20769e6c083b74238b8c8321b))
    - Add missing progress bar logic ([`13f4bf7`](https://github.com/AOSC-Dev/aoscpt/commit/13f4bf7fd1dccb1f4b7641f7c6b084dcd20a0d37))
    - Revert "fix: do not display download progress in retry" ([`0848a19`](https://github.com/AOSC-Dev/aoscpt/commit/0848a191a80e5f3aa5ca76124a233a16998b2643))
    - Revert "fix: fix yes argument download" ([`5736203`](https://github.com/AOSC-Dev/aoscpt/commit/5736203f0fa697918f4d30cbf0605d02af48f971))
</details>

## v0.18.1 (2023-04-01)

<csr-id-3f8df6ab624f073948505739c9e8c7ee3731e242/>

### Bug Fixes

 - <csr-id-cc54069158c0a44c01ee7ffb5ac5c1256deee750/> fix yes argument download
 - <csr-id-e346e06a30ddc829c73a8c981498ada4ce6844ab/> do not display download progress in retry
 - <csr-id-de55a3218a0bf0bd79464f82787c548ad6a20749/> pending ui message too loong to panic

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
    - Bump oma v0.18.1 ([`74c2dd8`](https://github.com/AOSC-Dev/aoscpt/commit/74c2dd89137b903532c5be2aa2db8320f7d70b61))
    - Optmize download before check file is exist logic ([`3f8df6a`](https://github.com/AOSC-Dev/aoscpt/commit/3f8df6ab624f073948505739c9e8c7ee3731e242))
    - Fix yes argument download ([`cc54069`](https://github.com/AOSC-Dev/aoscpt/commit/cc54069158c0a44c01ee7ffb5ac5c1256deee750))
    - Do not display download progress in retry ([`e346e06`](https://github.com/AOSC-Dev/aoscpt/commit/e346e06a30ddc829c73a8c981498ada4ce6844ab))
    - Pending ui message too loong to panic ([`de55a32`](https://github.com/AOSC-Dev/aoscpt/commit/de55a3218a0bf0bd79464f82787c548ad6a20749))
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

 - <csr-id-5960c70495227cb80ca9a991352857fde09ee60f/> improve command short help

### Bug Fixes

 - <csr-id-6efcc4fb282384c1ec8f9db623cac57964c6a230/> add missing oma mark help message
 - <csr-id-1d74e7083e36509cd54d636773c4b633c8c58973/> add missing subcommand ...
   ... Also log subcommand rename history
 - <csr-id-67f099ac2d618a6a0aab7e99ea5ea708e8bee151/> fix package name ends_with deb install

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
    - Bump oma v0.18.0 ([`f944f91`](https://github.com/AOSC-Dev/aoscpt/commit/f944f91f2d23b20b0c0a860dd1d84ca9651af743))
    - Update all deps ([`374b8e5`](https://github.com/AOSC-Dev/aoscpt/commit/374b8e51c2b5169cb497e08cb5e1f9163083ead1))
    - Use cargo clippy to lint code ([`f57d97f`](https://github.com/AOSC-Dev/aoscpt/commit/f57d97ffefb244e74890386ed720a3e026238907))
    - Add missing oma mark help message ([`6efcc4f`](https://github.com/AOSC-Dev/aoscpt/commit/6efcc4fb282384c1ec8f9db623cac57964c6a230))
    - Improve command short help ([`5960c70`](https://github.com/AOSC-Dev/aoscpt/commit/5960c70495227cb80ca9a991352857fde09ee60f))
    - Add missing subcommand ... ([`1d74e70`](https://github.com/AOSC-Dev/aoscpt/commit/1d74e7083e36509cd54d636773c4b633c8c58973))
    - Remove useless file ([`00ab061`](https://github.com/AOSC-Dev/aoscpt/commit/00ab06101ae2b211e2cbaea1fca7c76df2e6b1ac))
    - Fix package name ends_with deb install ([`67f099a`](https://github.com/AOSC-Dev/aoscpt/commit/67f099ac2d618a6a0aab7e99ea5ea708e8bee151))
    - Add man to .gitignore ([`eb9cae9`](https://github.com/AOSC-Dev/aoscpt/commit/eb9cae9c30805f5d80a6168b71a472bd53db9d2d))
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

 - <csr-id-438b89df8796ab1b74f2c2f69ac818e5f5ae764c/> output man pages to /man
 - <csr-id-f3f2ebba5b4feb23ecb3817c2571884d8588312b/> try use clap to gen man
 - <csr-id-3d572de0735df4fa193b6168e71657317b50995d/> add extract and verify database progress bar

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
    - Bump oma v0.17.1 ([`7e80de5`](https://github.com/AOSC-Dev/aoscpt/commit/7e80de5aad6cf344e5e78d770f163c7d7f184528))
    - Run cargo clippy ([`a8e1ca9`](https://github.com/AOSC-Dev/aoscpt/commit/a8e1ca91d85eafa1acd91994640eb029b86669d5))
    - Update all deps ([`08eb9d4`](https://github.com/AOSC-Dev/aoscpt/commit/08eb9d4349fe67e3fdb49f299023a3b29d385689))
    - Output man pages to /man ([`438b89d`](https://github.com/AOSC-Dev/aoscpt/commit/438b89df8796ab1b74f2c2f69ac818e5f5ae764c))
    - Use clap build api to build argument ([`f4318cb`](https://github.com/AOSC-Dev/aoscpt/commit/f4318cb27202a3e99d30e718560254d8ae4a1449))
    - Clap_cli.rs => args.rs ([`ca593f5`](https://github.com/AOSC-Dev/aoscpt/commit/ca593f5ffdb80db1e159ce7dc59dbcb7d4acd921))
    - Try use clap to gen man ([`f3f2ebb`](https://github.com/AOSC-Dev/aoscpt/commit/f3f2ebba5b4feb23ecb3817c2571884d8588312b))
    - Move single_handler code line location ([`5e2a8ad`](https://github.com/AOSC-Dev/aoscpt/commit/5e2a8adb0e1b7dce39a74b0f3d7aaaca4d3756ab))
    - Add extract and verify database progress bar ([`3d572de`](https://github.com/AOSC-Dev/aoscpt/commit/3d572de0735df4fa193b6168e71657317b50995d))
    - Update README ([`5e8d086`](https://github.com/AOSC-Dev/aoscpt/commit/5e8d086e7809ac3cd039c83eb4f618a25aa69e16))
    - Remove useless tracing-subscriber envfilter dep ([`595f777`](https://github.com/AOSC-Dev/aoscpt/commit/595f7772d54b17b5f46d125140073597b67551f3))
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

 - <csr-id-355746f2e38a2832e4ae645862eeef7f61387eea/> if fetch last url has error, output error prefix
 - <csr-id-33cb84a6eddfb5b2a507dff3361a09a1c874d56f/> add .policy file to add policykit oma infomation
 - <csr-id-a6ae5aede440aa515bd05257cc6f9701f491cdaf/> add policykit support

### Bug Fixes

 - <csr-id-980480288e2a2793ac2164994076c12ee814d865/> fix warning message before global bar draw display
 - <csr-id-b6006ee97d648426e4b70dc32be45ebe0a50d0c5/> try to fix download progress bar count
 - <csr-id-12f4e4cb784d2a7c93dd394326b493b74fa92005/> fix download database global bar display in file:// prefix local mirror
 - <csr-id-388a00416cee98107d89d7d38f6196a1223053c4/> fix exit code with policykit run

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
    - Bump oma v0.17.0 ([`7562a18`](https://github.com/AOSC-Dev/aoscpt/commit/7562a18ed823cb51ce23007621b347d6e7761d16))
    - Update all deps ([`0dca249`](https://github.com/AOSC-Dev/aoscpt/commit/0dca24982887115dd11b30f58a65d5014b2e1419))
    - Run cargo fmt and clippy ([`ed199fd`](https://github.com/AOSC-Dev/aoscpt/commit/ed199fddf44d379e9fa04af24ead28568814f9d6))
    - If fetch last url has error, output error prefix ([`355746f`](https://github.com/AOSC-Dev/aoscpt/commit/355746f2e38a2832e4ae645862eeef7f61387eea))
    - Fix warning message before global bar draw display ([`9804802`](https://github.com/AOSC-Dev/aoscpt/commit/980480288e2a2793ac2164994076c12ee814d865))
    - Try to fix download progress bar count ([`b6006ee`](https://github.com/AOSC-Dev/aoscpt/commit/b6006ee97d648426e4b70dc32be45ebe0a50d0c5))
    - Fix download database global bar display in file:// prefix local mirror ([`12f4e4c`](https://github.com/AOSC-Dev/aoscpt/commit/12f4e4cb784d2a7c93dd394326b493b74fa92005))
    - Default to /bin/oma ([`0679301`](https://github.com/AOSC-Dev/aoscpt/commit/06793013e11364e1aa64f81e9183e930888ba8bc))
    - Improve UI strings ([`50b9512`](https://github.com/AOSC-Dev/aoscpt/commit/50b9512f1e270012888c720a521a9354402fcf34))
    - Add .policy file to add policykit oma infomation ([`33cb84a`](https://github.com/AOSC-Dev/aoscpt/commit/33cb84a6eddfb5b2a507dff3361a09a1c874d56f))
    - Decompress database do not block tokio runner ([`fddc86f`](https://github.com/AOSC-Dev/aoscpt/commit/fddc86f2e61261f1e842c5a24eb94e6c49660bff))
    - Refactor content::file_handle method; rename to remove_prefix ([`6bc254e`](https://github.com/AOSC-Dev/aoscpt/commit/6bc254e135c6613852f3a221df30694988310a50))
    - Update rust-apt to newest git snapshot ([`6def641`](https://github.com/AOSC-Dev/aoscpt/commit/6def641dc68e68b54970090dea4d618f19b60787))
    - Add dependencies comment in Cargo.toml ([`58fd8c6`](https://github.com/AOSC-Dev/aoscpt/commit/58fd8c6a8eea2aa918ed9d271f995bfc1b598851))
    - OmaAction => Oma ([`4f8a5ff`](https://github.com/AOSC-Dev/aoscpt/commit/4f8a5ffdc160319ca28d6251bc2f6e5ccef0ceb7))
    - Refactor some code style ([`9bb7175`](https://github.com/AOSC-Dev/aoscpt/commit/9bb717573b5ee6928f139613cd0145dd00f120d5))
    - Do not always in running in async runtime ([`6b90abd`](https://github.com/AOSC-Dev/aoscpt/commit/6b90abdd8e1cd2a7140dc2c0ea881d8a2e875bd0))
    - Fix exit code with policykit run ([`388a004`](https://github.com/AOSC-Dev/aoscpt/commit/388a00416cee98107d89d7d38f6196a1223053c4))
    - Add policykit support ([`a6ae5ae`](https://github.com/AOSC-Dev/aoscpt/commit/a6ae5aede440aa515bd05257cc6f9701f491cdaf))
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

 - <csr-id-3c1d8355ced332e3c0249597cd6067798d21d9f5/> oma dep/rdep PreDepended by => Pre-Depended by
 - <csr-id-059e770325bf9c72b616324435b73185fa836859/> oma dep/rdep improve grammar display
 - <csr-id-6dd009b0aed38352c24e5d36c9389a662f8ee8d2/> do not display JSON-like args info in dry-run mode
 - <csr-id-99e0d9c46c82ebe8f7882ef14a1f1242ebddf626/> add oma --debug argument to see dry-run debug message
 - <csr-id-20368da4ca497bb5bbbf93e68599bf94f6c755b8/> log file remove 'Resolver' word
 - <csr-id-6f1c53a2d60bced6abc07128256866559874d227/> improve oma log Resolver key style
 - <csr-id-02217a97c880bee13efcefba8de8754f5e79a965/> log user actions in a human-readable fashion
 - <csr-id-88996e144efe83d63e20b4b71779cdde6d65fe5d/> find all provides in search.rs method to improve search result
 - <csr-id-afd404efad65a2089f0e12bdf3f22081dcd3da43/> fix reset is_provide status in search_pkgs method
 - <csr-id-3b4edeb9138f6b00547e14f967754193186490f2/> improve oma search result sort
 - <csr-id-d44c3469404f075a831332f3018f2b91a81a793a/> Read Dir::Cache::Archives value to get apt set download dir config
 - <csr-id-6b2a70a025e91f3e7b9c3f528ec487cc08e8c719/> oma download success will display download packages path
 - <csr-id-45d2dd15e3693adcbe0a6a9e032184a3e2d3e228/> oma download add argument --path (-p)
 - <csr-id-53e1d9e88c83113980483074016f552d1612e452/> support provides package install

### Bug Fixes

 - <csr-id-93835ef2382f6dfde75e59c35061f7ef5ed12f0b/> Fix oma start-date/end-date localtime offset
 - <csr-id-883dc2bb19810972b49c7b9129287b29c2b0f6d4/> Fix local package install
   But not using a better approach, wait https://gitlab.com/volian/rust-apt/-/issues/23
 - <csr-id-36cd76ffa6646d302fa2e5ad416b61a9c7c2fac3/> Add Dir::Cache::Archives fallback logic
 - <csr-id-3c62b065befd8e2cf9819ea41e3d6d2cee4e63e4/> Fix archive dir read logic
 - <csr-id-4e71663770db750e45ed749a645689ae9f3c4b1d/> only virtual pkg get provides to get real pkg
 - <csr-id-ded754f888c1587fcfab76353ba8a92008fd6019/> oma download do not download non candidate pkg
 - <csr-id-932b17beff2a4e3878833a7a45ee5f9e90dd9f1c/> local mirror progress display

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
    - Bump oma v0.16.0 ([`24c6dad`](https://github.com/AOSC-Dev/aoscpt/commit/24c6dadb4e50c634490f77ddabd5be05f82068c5))
    - Update all deps ([`85ad4b7`](https://github.com/AOSC-Dev/aoscpt/commit/85ad4b758dea42854c920957b69ae544f6d91a17))
    - Run cargo clippy and cargo fmt to lint code ([`cf804b9`](https://github.com/AOSC-Dev/aoscpt/commit/cf804b98d6220921a6b585b8da7bd3512c3268d9))
    - Oma dep/rdep PreDepended by => Pre-Depended by ([`3c1d835`](https://github.com/AOSC-Dev/aoscpt/commit/3c1d8355ced332e3c0249597cd6067798d21d9f5))
    - Oma dep/rdep improve grammar display ([`059e770`](https://github.com/AOSC-Dev/aoscpt/commit/059e770325bf9c72b616324435b73185fa836859))
    - Do not display JSON-like args info in dry-run mode ([`6dd009b`](https://github.com/AOSC-Dev/aoscpt/commit/6dd009b0aed38352c24e5d36c9389a662f8ee8d2))
    - Add oma --debug argument to see dry-run debug message ([`99e0d9c`](https://github.com/AOSC-Dev/aoscpt/commit/99e0d9c46c82ebe8f7882ef14a1f1242ebddf626))
    - Log file remove 'Resolver' word ([`20368da`](https://github.com/AOSC-Dev/aoscpt/commit/20368da4ca497bb5bbbf93e68599bf94f6c755b8))
    - Improve oma log Resolver key style ([`6f1c53a`](https://github.com/AOSC-Dev/aoscpt/commit/6f1c53a2d60bced6abc07128256866559874d227))
    - Log user actions in a human-readable fashion ([`02217a9`](https://github.com/AOSC-Dev/aoscpt/commit/02217a97c880bee13efcefba8de8754f5e79a965))
    - Lint code use cargo clippy ([`dfbb9e4`](https://github.com/AOSC-Dev/aoscpt/commit/dfbb9e418273d2f20f08a5c4b9b6b09a82935110))
    - Fix oma start-date/end-date localtime offset ([`93835ef`](https://github.com/AOSC-Dev/aoscpt/commit/93835ef2382f6dfde75e59c35061f7ef5ed12f0b))
    - Find all provides in search.rs method to improve search result ([`88996e1`](https://github.com/AOSC-Dev/aoscpt/commit/88996e144efe83d63e20b4b71779cdde6d65fe5d))
    - Fix reset is_provide status in search_pkgs method ([`afd404e`](https://github.com/AOSC-Dev/aoscpt/commit/afd404efad65a2089f0e12bdf3f22081dcd3da43))
    - Improve oma search result sort ([`3b4edeb`](https://github.com/AOSC-Dev/aoscpt/commit/3b4edeb9138f6b00547e14f967754193186490f2))
    - Use cargo clippy and cargo fmt ([`948923d`](https://github.com/AOSC-Dev/aoscpt/commit/948923de893b131e4fbd59bc5ed4be1ca383b69d))
    - Fix local package install ([`883dc2b`](https://github.com/AOSC-Dev/aoscpt/commit/883dc2bb19810972b49c7b9129287b29c2b0f6d4))
    - Add Dir::Cache::Archives fallback logic ([`36cd76f`](https://github.com/AOSC-Dev/aoscpt/commit/36cd76ffa6646d302fa2e5ad416b61a9c7c2fac3))
    - Drop useless line ([`2dde83d`](https://github.com/AOSC-Dev/aoscpt/commit/2dde83d82417506b9fa0f4a39237e8596dc6f530))
    - Fix archive dir read logic ([`3c62b06`](https://github.com/AOSC-Dev/aoscpt/commit/3c62b065befd8e2cf9819ea41e3d6d2cee4e63e4))
    - Read Dir::Cache::Archives value to get apt set download dir config ([`d44c346`](https://github.com/AOSC-Dev/aoscpt/commit/d44c3469404f075a831332f3018f2b91a81a793a))
    - Run cargo clippy' ([`7adb76c`](https://github.com/AOSC-Dev/aoscpt/commit/7adb76c9fe0c33b02ada9e856306db6855459227))
    - Oma download success will display download packages path ([`6b2a70a`](https://github.com/AOSC-Dev/aoscpt/commit/6b2a70a025e91f3e7b9c3f528ec487cc08e8c719))
    - Oma download add argument --path (-p) ([`45d2dd1`](https://github.com/AOSC-Dev/aoscpt/commit/45d2dd15e3693adcbe0a6a9e032184a3e2d3e228))
    - Only virtual pkg get provides to get real pkg ([`4e71663`](https://github.com/AOSC-Dev/aoscpt/commit/4e71663770db750e45ed749a645689ae9f3c4b1d))
    - Improve query pkg method ([`c411087`](https://github.com/AOSC-Dev/aoscpt/commit/c411087f4cf27b06f87b9db9ef8701f5c787ad81))
    - Improve get local pkgs ([`bbfc384`](https://github.com/AOSC-Dev/aoscpt/commit/bbfc384d3cb5289743014bf7a5e2805bb69dc4d0))
    - Support provides package install ([`53e1d9e`](https://github.com/AOSC-Dev/aoscpt/commit/53e1d9e88c83113980483074016f552d1612e452))
    - Oma download do not download non candidate pkg ([`ded754f`](https://github.com/AOSC-Dev/aoscpt/commit/ded754f888c1587fcfab76353ba8a92008fd6019))
    - Local mirror progress display ([`932b17b`](https://github.com/AOSC-Dev/aoscpt/commit/932b17beff2a4e3878833a7a45ee5f9e90dd9f1c))
</details>

## v0.15.0 (2023-03-24)

<csr-id-44235f94ccb41303e33e308a6b75215d2d2f2f48/>
<csr-id-698396ab0be30c4d815138fa6ebc5cda4a41df43/>
<csr-id-4f9782f2112188ce689133654d4122835b57742e/>

### Chore

 - <csr-id-44235f94ccb41303e33e308a6b75215d2d2f2f48/> update all deps

### New Features

 - <csr-id-3be76aab9e4380467e050acd0b6dd00613c99b10/> dry-run read RUST_LOG env to see debug message (default RUST_LOG is info)
 - <csr-id-8aafec43ec24de10bcf0544ec078ceb8f8d4ad02/> set oma and os dry-run info as debug
 - <csr-id-ce234a3d9ffc2622235d5ca6d1e762d1629a6d23/> improve log user-family output
 - <csr-id-900a15564cf0a88ade4efab875fb080fe334a9d1/> dry-run mode display oma and OS info
   - Also fix dry-run downgrade typo

### Bug Fixes

 - <csr-id-6e100d38ecd6a2787bb2b94a9d531a706bfa27db/> fix dry-run default display log
 - <csr-id-1276493d91b129fa63307f872522071c4d239f8a/> fix dry-run in fix-broken subcommand argument
 - <csr-id-526bc122ea477eb36c99415892e11df14ae3e452/> do not real run {mark,clean,download} in dry-run mode

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
    - Bump oma v0.15.0 ([`679d8e7`](https://github.com/AOSC-Dev/aoscpt/commit/679d8e7f6ff23d1c848c09a46891e02f2f492681))
    - Update all deps ([`44235f9`](https://github.com/AOSC-Dev/aoscpt/commit/44235f94ccb41303e33e308a6b75215d2d2f2f48))
    - Fix dry-run default display log ([`6e100d3`](https://github.com/AOSC-Dev/aoscpt/commit/6e100d38ecd6a2787bb2b94a9d531a706bfa27db))
    - Use cargo fmt to lint code style ([`698396a`](https://github.com/AOSC-Dev/aoscpt/commit/698396ab0be30c4d815138fa6ebc5cda4a41df43))
    - Dry-run read RUST_LOG env to see debug message (default RUST_LOG is info) ([`3be76aa`](https://github.com/AOSC-Dev/aoscpt/commit/3be76aab9e4380467e050acd0b6dd00613c99b10))
    - Fix dry-run in fix-broken subcommand argument ([`1276493`](https://github.com/AOSC-Dev/aoscpt/commit/1276493d91b129fa63307f872522071c4d239f8a))
    - Do not real run {mark,clean,download} in dry-run mode ([`526bc12`](https://github.com/AOSC-Dev/aoscpt/commit/526bc122ea477eb36c99415892e11df14ae3e452))
    - Set oma and os dry-run info as debug ([`8aafec4`](https://github.com/AOSC-Dev/aoscpt/commit/8aafec43ec24de10bcf0544ec078ceb8f8d4ad02))
    - Improve log user-family output ([`ce234a3`](https://github.com/AOSC-Dev/aoscpt/commit/ce234a3d9ffc2622235d5ca6d1e762d1629a6d23))
    - Dry-run mode display oma and OS info ([`900a155`](https://github.com/AOSC-Dev/aoscpt/commit/900a15564cf0a88ade4efab875fb080fe334a9d1))
    - Improve pick method code style ([`4f9782f`](https://github.com/AOSC-Dev/aoscpt/commit/4f9782f2112188ce689133654d4122835b57742e))
</details>

## v0.14.0 (2023-03-23)

<csr-id-ff6749dce59ae4fa50cac786cfaecc17e828c040/>
<csr-id-89ea59750fccbc487ea7826193e85bbfb39ce14b/>

### New Features

<csr-id-4e6bb0a59edd36b02f7fd0e656f37d5acc5bb0db/>

 - <csr-id-903748b5ce82e5bb0c7b78a75b9f1dbe3d32d1ea/> dry-run mode add args tracing
 - <csr-id-123da2c985075fe3ae4a04be55e007fdede83460/> add oma --dry-run argument
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
    - Bump oma v0.14.0 ([`241a797`](https://github.com/AOSC-Dev/aoscpt/commit/241a797fa428db235522848a2c9ffb28b3dfedbf))
    - Dry-run mode add args tracing ([`903748b`](https://github.com/AOSC-Dev/aoscpt/commit/903748b5ce82e5bb0c7b78a75b9f1dbe3d32d1ea))
    - Add oma --dry-run argument ([`123da2c`](https://github.com/AOSC-Dev/aoscpt/commit/123da2c985075fe3ae4a04be55e007fdede83460))
    - Improve DOWNLOAD_DIR var use ([`ff6749d`](https://github.com/AOSC-Dev/aoscpt/commit/ff6749dce59ae4fa50cac786cfaecc17e828c040))
    - Use fs::read to replace fs::File::open and read_buf ([`89ea597`](https://github.com/AOSC-Dev/aoscpt/commit/89ea59750fccbc487ea7826193e85bbfb39ce14b))
    - If pkg is essential, oma will reject it mark to delete ([`4e6bb0a`](https://github.com/AOSC-Dev/aoscpt/commit/4e6bb0a59edd36b02f7fd0e656f37d5acc5bb0db))
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
    - Bump oma v0.13.2 ([`490b22f`](https://github.com/AOSC-Dev/aoscpt/commit/490b22fe4aa5a3fc5dc6988c42c0755a8abf9b60))
    - New ([`1341b47`](https://github.com/AOSC-Dev/aoscpt/commit/1341b47750c8541fb6cdfabe4b0191443c407a10))
    - Bump version to 0.13.2 for adapt cargo-smart-release ([`600f7d6`](https://github.com/AOSC-Dev/aoscpt/commit/600f7d65ca44dc2e91813db55a47ee3e63c7628a))
    - Cargo fmt ([`eb77d99`](https://github.com/AOSC-Dev/aoscpt/commit/eb77d991a3c2bf21a784af41b5cc92fc0792af42))
    - Find_unmet_deps_with_markinstall if apt cache could not find pkg, add to UnmetTable list ([`1c119f3`](https://github.com/AOSC-Dev/aoscpt/commit/1c119f3e53f2e0bb5a5ca55c1d53c9431fd60caf))
    - Find_unmet_deps_with_markinstall method do not display 'User aborted the operation' info ([`4431915`](https://github.com/AOSC-Dev/aoscpt/commit/4431915c4fa66237e93705b19771147d1d660ad8))
    - Add find_unmet_deps_with_markinstall method to handle if mark_install can't success ([`3e9764b`](https://github.com/AOSC-Dev/aoscpt/commit/3e9764b82afaf587fb207edc00bb94202117d181))
    - If find_unmet_deps can't find any dependencies problem, return apt's error ([`ced63ab`](https://github.com/AOSC-Dev/aoscpt/commit/ced63ab66de0027cd072cf28457bfe9af7091835))
    - Update ([`942a984`](https://github.com/AOSC-Dev/aoscpt/commit/942a98490c370741f08b34d0fc4f0ee49c3cb904))
    - Fake clap more like real clap ([`c147f6f`](https://github.com/AOSC-Dev/aoscpt/commit/c147f6f66b54f51dcbbc95a84af04764602913ab))
    - Revert "main: fake clap more like real clap" ([`9f19391`](https://github.com/AOSC-Dev/aoscpt/commit/9f19391cf0a06bb08c2bebdbe075360e12fff62b))
    - Fake clap more like real clap ([`2b19d1d`](https://github.com/AOSC-Dev/aoscpt/commit/2b19d1d1d43503b696d1f68e825e8db62e940851))
    - Use cargo fmt ([`35e27cf`](https://github.com/AOSC-Dev/aoscpt/commit/35e27cf6eb267da34f6d07e7f0df8ac6564befa0))
    - Add fake clap output for wrong --ailurus argument count ([`acf7e43`](https://github.com/AOSC-Dev/aoscpt/commit/acf7e43838811177f4838cf2a97a217540803e86))
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
    - 0.13.1 ([`3203ddf`](https://github.com/AOSC-Dev/aoscpt/commit/3203ddf3619c2d5680d9c30e872675e98e752a56))
    - Improve pending ui style ([`94c2694`](https://github.com/AOSC-Dev/aoscpt/commit/94c2694ff1dfd8eccccd01e1e280b9418e83ae1b))
    - 0.13.0 ([`0ae5460`](https://github.com/AOSC-Dev/aoscpt/commit/0ae546044d7a3c5b5dedf971d44f485e5b8dd270))
    - Progress spinner use oma style ([`cd0eeeb`](https://github.com/AOSC-Dev/aoscpt/commit/cd0eeeb92dcd0f037e4ea4ff55430129b29bc551))
    - Unmet ui do not right align ([`70e34c7`](https://github.com/AOSC-Dev/aoscpt/commit/70e34c78bc001ed51eec97a0ae340ba78a8d75b6))
    - Improve unmet pending ui style ([`0facb38`](https://github.com/AOSC-Dev/aoscpt/commit/0facb38b8edee4c8e1dcf9448c0fe5da7ae87600))
    - Add PreDepends to unmet dependencies table ([`d693d2b`](https://github.com/AOSC-Dev/aoscpt/commit/d693d2be4c2690182faf4121af71ff93b513159f))
    - Add Break and Conflict to unmet dependencies table ([`38d8386`](https://github.com/AOSC-Dev/aoscpt/commit/38d8386882bd863da094a8b5ca6dadc0f53a41b7))
    - Use cargo clippy ([`bc35416`](https://github.com/AOSC-Dev/aoscpt/commit/bc35416bf773ec00df6e7de4efdbc1fffe54d83c))
    - Use OnceCell::Lazy<PathBuf> to replace Path String ([`02ab2e5`](https://github.com/AOSC-Dev/aoscpt/commit/02ab2e5aeed834cb26a660b8e88a3081f838ae92))
    - Move mark_install method to pkg.rs ([`feb5116`](https://github.com/AOSC-Dev/aoscpt/commit/feb5116fda77e1e67e68853adfae7c2189fa77c9))
    - Adjust code stract ([`5e24eac`](https://github.com/AOSC-Dev/aoscpt/commit/5e24eaccd5828d51439ef7d18debf5f192559e46))
    - Improve find unmet dep logic ([`64fedc8`](https://github.com/AOSC-Dev/aoscpt/commit/64fedc84eb38d0640239ea559b67110547aa63be))
    - Do not display user abort op in find_unmet dep method ([`c7fe71c`](https://github.com/AOSC-Dev/aoscpt/commit/c7fe71c710e8284549913abbf16eb81be2c38a43))
    - Add unmet dependency error output ([`c038023`](https://github.com/AOSC-Dev/aoscpt/commit/c0380232311a6ee7871ddb17d8b6f396831a34e3))
    - Use Lazy<Writer> replaced OnceCell<Writer> ([`15b0228`](https://github.com/AOSC-Dev/aoscpt/commit/15b02280f87894e3c0367cda82b03e1f629f22ee))
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
    - 0.1.0-alpha.12 ([`3864bf0`](https://github.com/AOSC-Dev/aoscpt/commit/3864bf049d5dd1871d2eb3ed2d437529249f5532))
    - Adjust log format ([`717685f`](https://github.com/AOSC-Dev/aoscpt/commit/717685f8ca5301d01a5ed493b64d75cfc4dd6edf))
    - Use cargo clippy ([`7361f3f`](https://github.com/AOSC-Dev/aoscpt/commit/7361f3f1bb04c027b46dfcbdbf1ea20ef2304e90))
    - Use once_cell replaced lazy_static ([`bd1b454`](https://github.com/AOSC-Dev/aoscpt/commit/bd1b4542b32a0261e20220d8e013eb3baca13ec5))
    - Main, action: add oma 'log' subcommand ([`dce41a7`](https://github.com/AOSC-Dev/aoscpt/commit/dce41a7597180244f25187f55ef2324daa1124d3))
    - Log format adjust ([`f828910`](https://github.com/AOSC-Dev/aoscpt/commit/f828910e466a282b194d82f833954d46a5736a06))
    - Fix install loop ([`63355e4`](https://github.com/AOSC-Dev/aoscpt/commit/63355e40544b1ae8fd6741dda9ecd1f412bf0c03))
    - Rewrite log write ([`97f8c98`](https://github.com/AOSC-Dev/aoscpt/commit/97f8c985bed1c5615d16009ae4deb45339d5ba9e))
    - Set log filename as history ([`ac1b745`](https://github.com/AOSC-Dev/aoscpt/commit/ac1b745bdf1e3c7573e8fc7b2ac8356a92ad9c82))
    - Add oma log feature ... ([`9431bfc`](https://github.com/AOSC-Dev/aoscpt/commit/9431bfc5dead6109a5432c89cb49afe014f68f60))
    - Action, main: add oma remove --keep-config argument ([`2ad9b61`](https://github.com/AOSC-Dev/aoscpt/commit/2ad9b616c8aaffbea5067001ad58866fc07ac502))
    - Improve remove table ui statement ([`fd75161`](https://github.com/AOSC-Dev/aoscpt/commit/fd75161f7445829ef7757342eb290328b00bef26))
    - Adjust upgrade table color again ([`69055f2`](https://github.com/AOSC-Dev/aoscpt/commit/69055f2ab5a43d8691d675203343a6eb41b0fd9b))
    - Fix pending ui upgrade/install style ([`72f2749`](https://github.com/AOSC-Dev/aoscpt/commit/72f274938a65398e3fb40fe8be3cfc37d4eb6303))
    - Improve pending ui style ... ([`538bd24`](https://github.com/AOSC-Dev/aoscpt/commit/538bd24f670ea4b8d89480691b3757a34efc8ad5))
    - Update TODO ([`a3838cb`](https://github.com/AOSC-Dev/aoscpt/commit/a3838cb3522da32f7c8eb6fa26d792609765f3cc))
    - Action, main: add 'rdepends' subcommand ([`b338c34`](https://github.com/AOSC-Dev/aoscpt/commit/b338c3414021a99affaaae39dab10b4a333a80c9))
    - Remove redundant reqwest error handle ([`471d218`](https://github.com/AOSC-Dev/aoscpt/commit/471d21899858cdd5e07cbe6ab2231a8fe36ae4e1))
    - Improve 'download' method logic ... ([`44668bf`](https://github.com/AOSC-Dev/aoscpt/commit/44668bf051abdde163e2a7661e61cb0520b121a8))
    - Add some ailurus ([`06ee6f6`](https://github.com/AOSC-Dev/aoscpt/commit/06ee6f61df9ac195cea1c63b7d78f647e5361c87))
    - Code clean up ([`fc240c9`](https://github.com/AOSC-Dev/aoscpt/commit/fc240c97d13ecfcfe619ca0cb964b0bdb2b12f65))
    - Use bouncingBall spinner style ([`453f080`](https://github.com/AOSC-Dev/aoscpt/commit/453f08050dd3cce2bfb81fbc9e663b02895d12b7))
    - Improve download InRelease ProgressBar ([`eceba98`](https://github.com/AOSC-Dev/aoscpt/commit/eceba98e0caa91323b3e9b613ee2917975f56e35))
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
    - Release 0.1.0-alpha.11 ([`5f80ef1`](https://github.com/AOSC-Dev/aoscpt/commit/5f80ef1b06077f63919d8b26e57c1838d87147ed))
    - Fix multi key in one cert file error handle (2) ([`ee75844`](https://github.com/AOSC-Dev/aoscpt/commit/ee758444c7467afd607bbef7f29ccb7efe412284))
    - Fix multi key in one cert file  error handle ([`7c137d6`](https://github.com/AOSC-Dev/aoscpt/commit/7c137d68e90b2f83d03305a8ae703d860a2ab1a1))
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
    - Release 0.1.0-alpha.10 ([`f2eb643`](https://github.com/AOSC-Dev/aoscpt/commit/f2eb6433884ac7b989283297bcd4c805db5c5b7c))
    - Fix multi key in one cert file parser ([`0535b85`](https://github.com/AOSC-Dev/aoscpt/commit/0535b85f85e53257c1464b4f87c2736ad54f680c))
    - Use cargo clippy ([`8884e41`](https://github.com/AOSC-Dev/aoscpt/commit/8884e41d50f596f825f9a765b29fdc9ba2dd2008))
    - Use own debcontrol-rs fork to fix rustc warning ([`62c0a2f`](https://github.com/AOSC-Dev/aoscpt/commit/62c0a2f36b05ccafea3f1e96937ec75bc90326e7))
    - Fix global pb style ([`d2797a0`](https://github.com/AOSC-Dev/aoscpt/commit/d2797a0cf8a9942ec84c947e92ffdc98de791e46))
    - Add args comment ([`3a825ee`](https://github.com/AOSC-Dev/aoscpt/commit/3a825ee033fa29705cc4d0d09dd6b910085c3007))
    - If no --yes do not set --force-confold ([`013281f`](https://github.com/AOSC-Dev/aoscpt/commit/013281f587ac0481e8a61598b544fe50b9b83753))
    - Add missing key dueto ([`ae7fb4c`](https://github.com/AOSC-Dev/aoscpt/commit/ae7fb4c3c1ea19dfc9978236933a354b540e97d7))
    - Support .asc file verify ([`7ea894d`](https://github.com/AOSC-Dev/aoscpt/commit/7ea894d28bebe7bbfa99a4a30e1e470aab0a60e8))
    - Add apt sources.list signed-by support ([`5065a42`](https://github.com/AOSC-Dev/aoscpt/commit/5065a424240d8bd8b964b9753fac7efb0582c9ae))
    - {Yes,Apt}InstallProgress -> OmaAptInstallProgress ... ([`c9de9df`](https://github.com/AOSC-Dev/aoscpt/commit/c9de9df27acd61a1a91fca765dfadf63ed77e28f))
    - Set global pb steady_tick as 100ms ([`a81c4be`](https://github.com/AOSC-Dev/aoscpt/commit/a81c4be8b023a1b2146c12d3ae86897f0219a157))
    - Optimize update_db logic ([`5d35833`](https://github.com/AOSC-Dev/aoscpt/commit/5d3583346a4e85e7e89cda63e6ec889fbf01777c))
    - Optimize down_package method logic ([`2fa253c`](https://github.com/AOSC-Dev/aoscpt/commit/2fa253c1fadfe55e8637cc0c9f5828021d351e79))
    - Download global progress bar use bytes/total_bytes, not speed ([`a3734d0`](https://github.com/AOSC-Dev/aoscpt/commit/a3734d0a3dd954a730f7506e4bf654b1cb3e2e81))
    - Add 'force_yes' argument to apt_handler method ([`7ee782e`](https://github.com/AOSC-Dev/aoscpt/commit/7ee782ef5f0a45bf82d2f38039e25707fe566b8e))
    - Make cargo clippy happy ([`5b44b13`](https://github.com/AOSC-Dev/aoscpt/commit/5b44b139fd2ba2b5bd8cf85cb9627a9841a3ef13))
    - Action, main: add --force-yes option ([`fbcb1d1`](https://github.com/AOSC-Dev/aoscpt/commit/fbcb1d1eb0c130980bb1062a2ea6299a99f7fb89))
    - Fix install need root tips ([`c008b8c`](https://github.com/AOSC-Dev/aoscpt/commit/c008b8c921af22aa31645fd23273cb9900730be6))
    - Add yes warn ([`b2cff14`](https://github.com/AOSC-Dev/aoscpt/commit/b2cff14f4093678e286739237ad2b5e0932812bb))
    - Oma-yes =? oma --yes/-y ([`7aed08d`](https://github.com/AOSC-Dev/aoscpt/commit/7aed08d72501cb8de3d133968d0ba1bb38344a5d))
    - Try to fix apt automatic install (2) ([`a8b65ee`](https://github.com/AOSC-Dev/aoscpt/commit/a8b65ee6bb60020ad3be1a56019382547dc62be6))
    - Try to fix apt automatic install ([`6e94e6b`](https://github.com/AOSC-Dev/aoscpt/commit/6e94e6b2b9fab1377f2a21af79c075777dc34b6d))
    - Add yes warn ([`5823364`](https://github.com/AOSC-Dev/aoscpt/commit/5823364d2177b07890fe21a0d3664f2cac2f5658))
    - Fix dead forloop ([`4d60070`](https://github.com/AOSC-Dev/aoscpt/commit/4d600702e04d948f7758a533dfc8323feca8a54e))
    - Use cargo clippy ([`8103ebb`](https://github.com/AOSC-Dev/aoscpt/commit/8103ebbe8200983671e26f1599e6439515c6c1ab))
    - Allow yes option ([`532a184`](https://github.com/AOSC-Dev/aoscpt/commit/532a184b8f958d211b671051eda44fc898888d50))
    - Add 'yes' option ([`d1be583`](https://github.com/AOSC-Dev/aoscpt/commit/d1be5839fc6e6fe61df690d21bc929818363dd36))
    - Move main.rs to bin/oma.rs ([`824ccaf`](https://github.com/AOSC-Dev/aoscpt/commit/824ccaf4a43e6cc2df863fad944a899aa86e319a))
    - Add oma list argument --upgradable (-u) ([`d901129`](https://github.com/AOSC-Dev/aoscpt/commit/d9011297c7dd215992e47eea629536aefc2b2c8a))
    - Use cargo clippy ([`9bf4c2f`](https://github.com/AOSC-Dev/aoscpt/commit/9bf4c2fc3d19bf83a0632ca20ac2b75433ca5cb0))
    - Try to fix ci ([`bc484e4`](https://github.com/AOSC-Dev/aoscpt/commit/bc484e40dad02e3965f4d8ffe719a6558b0a5a91))
    - Revert "main: try to fix random segfault" ([`8558ff6`](https://github.com/AOSC-Dev/aoscpt/commit/8558ff628377e4c53bfc2d1f3e0e5792fbe1accc))
    - Try to fix random segfault ([`714529d`](https://github.com/AOSC-Dev/aoscpt/commit/714529da1fa6face96e866fc86793b8c21e62abd))
    - Fix oma pick select pos ([`0f2b95d`](https://github.com/AOSC-Dev/aoscpt/commit/0f2b95d9f842840f6fb5870157ab265d30622852))
    - Improve pick version display ([`6564448`](https://github.com/AOSC-Dev/aoscpt/commit/656444875d5d74864ddda55faff5df7b40cac05d))
    - Fix pick display wrong branch ([`a4fcfac`](https://github.com/AOSC-Dev/aoscpt/commit/a4fcfac05d6719b43124b4cac5022a115014df19))
    - Fix pick panic ([`3e21fe2`](https://github.com/AOSC-Dev/aoscpt/commit/3e21fe2cbc820b911ab915a8eff098047d6a2601))
    - Pick display branch if version is equal ([`7acad78`](https://github.com/AOSC-Dev/aoscpt/commit/7acad788ab7be3c890d325aaf55a88a50ba3f8e1))
    - Improve pick select version ([`3dc170c`](https://github.com/AOSC-Dev/aoscpt/commit/3dc170c3d89f2ea2c73e6fbbaf4b0091a0a7be6f))
    - Improve install version select ([`bfbd490`](https://github.com/AOSC-Dev/aoscpt/commit/bfbd490691e7458e51d4a47e9167ea1afc501a6c))
    - Use cargo clippy ([`ee54410`](https://github.com/AOSC-Dev/aoscpt/commit/ee544105388ac5d2cad755a640f772df62ee5d2e))
    - Fix the version selection problem of the same version but different sources ([`9b315b9`](https://github.com/AOSC-Dev/aoscpt/commit/9b315b9713f49b2d0f615128785e6e6564f5ed25))
    - Fix local source twice fetch ([`7a3caed`](https://github.com/AOSC-Dev/aoscpt/commit/7a3caedeab4634d2dc6e18a102f91fd9ba2eaa8a))
    - Fix a typo ([`cfd533d`](https://github.com/AOSC-Dev/aoscpt/commit/cfd533dac3f5372f4dbb67262f8f82f821044eec))
    - Update TODO and usage ([`6962e1c`](https://github.com/AOSC-Dev/aoscpt/commit/6962e1c589f42b7c01f92f2b414abb7539991e8d))
    - Update TODO ([`5ec7732`](https://github.com/AOSC-Dev/aoscpt/commit/5ec77326f4c7531d6e0fead9766327d584c5675a))
    - Add clean subcommand description ([`83d7b60`](https://github.com/AOSC-Dev/aoscpt/commit/83d7b60ce48111e2c6984f45124749e2507add39))
    - Update ([`7bdf019`](https://github.com/AOSC-Dev/aoscpt/commit/7bdf0190216f03590af9f73a0e95159537947bb3))
    - Add 'oma clean' subcommand ([`177213b`](https://github.com/AOSC-Dev/aoscpt/commit/177213b5b35a2ac7a35c59272fcc82cdc2516e1c))
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
    - 0.1.0-alpha.9 ([`8cae498`](https://github.com/AOSC-Dev/aoscpt/commit/8cae498183782659c2a4dec3f70cc28406d03d88))
    - Comment unuse code ([`dc18621`](https://github.com/AOSC-Dev/aoscpt/commit/dc18621dbf7ee163c77f608cfb11c63dd8f15950))
    - Use fs4 replaced fs2 crate ([`3942a49`](https://github.com/AOSC-Dev/aoscpt/commit/3942a495a6839b7e86c00c454d21d73eed578c47))
    - Size_checker display human bytes ([`14a0cd2`](https://github.com/AOSC-Dev/aoscpt/commit/14a0cd28bbbf3e2fcfe4a4e644f6fabe711b594b))
    - Try to fix install count == 0 but configure count != 0, oma exit ([`2f2e883`](https://github.com/AOSC-Dev/aoscpt/commit/2f2e883024b18b9329dbebb60dd0f67193c09b2d))
    - Main, action, pkg: add 'oma depends' subcommand ([`988ff6c`](https://github.com/AOSC-Dev/aoscpt/commit/988ff6c65db6b8a52a0d5403195c353872b07e8d))
    - Fix mark hold/unhold pkgs can't unlock dpkg ([`87726c6`](https://github.com/AOSC-Dev/aoscpt/commit/87726c6f0c8a7bbf6e8aa94b790ffea0bed6f4b1))
    - Search, main: support oma search multi keyword ([`86befbd`](https://github.com/AOSC-Dev/aoscpt/commit/86befbde0daab0deb12b18a497a9e593fff0b673))
    - Move packages_download function from db.rs ([`431e1b9`](https://github.com/AOSC-Dev/aoscpt/commit/431e1b926e9fb7ff5e44a17b346ac14347a84a56))
    - Fix a typo ([`bccae00`](https://github.com/AOSC-Dev/aoscpt/commit/bccae00191a00804a3dedc42313c4baa1d4885b7))
    - Oma list/show/search if results is empty, return error ([`fb78a8e`](https://github.com/AOSC-Dev/aoscpt/commit/fb78a8ed0cb5fe33eab90739c55f2a98bbb9d9ed))
    - Add tips to oma install doesn't not exist pkg ([`baf917e`](https://github.com/AOSC-Dev/aoscpt/commit/baf917eac0481fe84f8717368d101d93f4c3824b))
    - Add some comment; improve display_result logic ([`df275e3`](https://github.com/AOSC-Dev/aoscpt/commit/df275e38f5b9eed2eda25adc1f689ebdbff7af6f))
    - Add oma install --no-fix-broken and --no-upgrade argument ([`17c6d8e`](https://github.com/AOSC-Dev/aoscpt/commit/17c6d8ef7b116e141c578b3cab1ff82c0f623c62))
    - Oma install default fix_broken pkg ([`681f838`](https://github.com/AOSC-Dev/aoscpt/commit/681f8382e51aede9f8ca63c16e82df1741435989))
    - Improve fix-broken feature ([`83da175`](https://github.com/AOSC-Dev/aoscpt/commit/83da175abc0882f1a4b79eee90df640448caa480))
    - Fix list installed display ([`8e503f2`](https://github.com/AOSC-Dev/aoscpt/commit/8e503f2a000b38597c2c28d2641800bb575bcd51))
    - Update usage and fix  typo ([`38e5c4d`](https://github.com/AOSC-Dev/aoscpt/commit/38e5c4d5600d9016fc44964e9cbadcd927b78bf9))
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
    - 0.1.0-alpha.8 ([`5fcc486`](https://github.com/AOSC-Dev/aoscpt/commit/5fcc486b75fb3004545197666012ae89d5f2ef79))
    - Use cargo fmt and clippy ([`f3f7367`](https://github.com/AOSC-Dev/aoscpt/commit/f3f7367e650e05fe12923252374a3b66e5e4a587))
    - Improve cmp logic ([`bb02829`](https://github.com/AOSC-Dev/aoscpt/commit/bb028296f2434d21b033a70433538c7336e3bc0e))
    - If input equal provide name, sort to top ([`2672aa8`](https://github.com/AOSC-Dev/aoscpt/commit/2672aa86b3c88bbd03b7b41bea48843f08b16062))
    - Fix install wrong pkg version ([`3877d82`](https://github.com/AOSC-Dev/aoscpt/commit/3877d8221fc0d95f8e10961b140d42b2209c70c1))
    - Different pages display different tips ([`0df060b`](https://github.com/AOSC-Dev/aoscpt/commit/0df060b6cb008a7f0d908b74ffb73d3390a683fb))
    - Action, search: fix writeln broken pipe ([`63ff207`](https://github.com/AOSC-Dev/aoscpt/commit/63ff207a4691dfbd7abdbf1bcec72457f282a2bc))
    - If height > 1 page, use less to display ([`0df66e5`](https://github.com/AOSC-Dev/aoscpt/commit/0df66e56c5ed025f45e1dbfe147ddacf9f3d1d3d))
    - Action, cli, main: support oma provides/list-files use pages ([`105e120`](https://github.com/AOSC-Dev/aoscpt/commit/105e12085f1acf6c6a01af7bc17860a8ffcefce9))
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
    - 0.1.0-alpha.7 ([`4a2af1c`](https://github.com/AOSC-Dev/aoscpt/commit/4a2af1ceca9ef77f0df2fe4fdd77824b8a836ca0))
    - Use cargo clippy, fmt ([`aee0f98`](https://github.com/AOSC-Dev/aoscpt/commit/aee0f984b2b15f3f92d64bf54327a6dc42bd773c))
    - Improve logic ([`6d96ef8`](https://github.com/AOSC-Dev/aoscpt/commit/6d96ef8b3e161cb87fc662e54a6a221c6ce151ec))
    - Improve local deb install logic ([`9c161b5`](https://github.com/AOSC-Dev/aoscpt/commit/9c161b55025bf8cbf4322390ad94e15f99fa48af))
    - Adjust pb steady_tick and if rg return non-zero code return error ([`ee829d3`](https://github.com/AOSC-Dev/aoscpt/commit/ee829d300062c465416d9130ba96d664feb74032))
    - Add progress spinner output ([`8653cad`](https://github.com/AOSC-Dev/aoscpt/commit/8653cad6188a014c5edc7c331e344049ad52bbfc))
    - Subcommand 'mark' adjust ([`e6eaa16`](https://github.com/AOSC-Dev/aoscpt/commit/e6eaa163a6bc9588406f5ecdb80d6fe01c6bfd4c))
    - If oma remove package does not exist display info ([`ca5b43a`](https://github.com/AOSC-Dev/aoscpt/commit/ca5b43a45943d8681f81b6d026de58cf5ef94158))
    - Fetch done display info ([`c18e3c5`](https://github.com/AOSC-Dev/aoscpt/commit/c18e3c52059ce45bb6957e78c20e104d5865bc4c))
    - Check root after lock oma to fix need root tips ([`2e5e068`](https://github.com/AOSC-Dev/aoscpt/commit/2e5e068e6bac2ad4cb180402cd5b496d6b45be39))
    - Show add display additional version info output ([`18aa896`](https://github.com/AOSC-Dev/aoscpt/commit/18aa896bb7d19c913d1901d77973d29afc6b3731))
    - Add oma show -a argument ([`344147f`](https://github.com/AOSC-Dev/aoscpt/commit/344147f8a054d19ad3365a413d23b1dbaab1329d))
    - Fix another_version info display again ([`e8781aa`](https://github.com/AOSC-Dev/aoscpt/commit/e8781aae38372f65a116e19da7e289f5282b4d97))
    - Fix another_version info display ([`fcb986c`](https://github.com/AOSC-Dev/aoscpt/commit/fcb986c886ebb0b3c32862c92f189992bb226e8c))
    - List add display additional version info output ([`a1713ea`](https://github.com/AOSC-Dev/aoscpt/commit/a1713ea7f1a4dd17f93a16a64e0232cd8a36fa0d))
    - List add automatic status display ([`9044526`](https://github.com/AOSC-Dev/aoscpt/commit/904452659fa2a81aa27ace515fbde92c261ddcf3))
    - Oma remove add 'purge' alias ([`3b090cb`](https://github.com/AOSC-Dev/aoscpt/commit/3b090cb746d392293f1b8c010b4d4aa7af56f612))
    - Fix local source metadata fetch ([`047a967`](https://github.com/AOSC-Dev/aoscpt/commit/047a967352020806bf71408f3f8b5dd7063aae0f))
    - Cargo clippy ([`76b4fda`](https://github.com/AOSC-Dev/aoscpt/commit/76b4fda0c7c692360054661519dd6225a7a953b0))
    - Fix install local pkg version select ([`27d07d4`](https://github.com/AOSC-Dev/aoscpt/commit/27d07d4a2e09159ad09686ce1488c44d1d014f3b))
    - Output package file path when local installation returns an error ([`b1c73e6`](https://github.com/AOSC-Dev/aoscpt/commit/b1c73e61277fa309fce1cd090a1233895ae9b600))
    - Abstract some step to mark_install function ([`cec450e`](https://github.com/AOSC-Dev/aoscpt/commit/cec450e059935b21f5cd259c8b018bcff6fa6a8e))
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
    - 0.1.0-alpha.6 ([`3d73b83`](https://github.com/AOSC-Dev/aoscpt/commit/3d73b8332f3c2406d419c248eb38890b7e3a8930))
    - Remove debug output ([`f71e8ca`](https://github.com/AOSC-Dev/aoscpt/commit/f71e8ca35774cc90f696d3029b2585fc1797ea23))
    - Fix download need sudo ([`345e4ef`](https://github.com/AOSC-Dev/aoscpt/commit/345e4efdfc80becb54446f1203b576ad8fc2d85a))
    - Fix marked upgrade/downgrade check ([`3231d16`](https://github.com/AOSC-Dev/aoscpt/commit/3231d16356dcc484afaa8629b5585f698da2360d))
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
    - 0.1.0-alpha.5 ([`8922d97`](https://github.com/AOSC-Dev/aoscpt/commit/8922d97c428e4e8d0a630aa275157d2f31b90d23))
    - Use cargo clippy ([`4cfdcf0`](https://github.com/AOSC-Dev/aoscpt/commit/4cfdcf017625363780eb8e0b2be66afd8a3f6ad9))
    - Fix a typo ([`3a414ac`](https://github.com/AOSC-Dev/aoscpt/commit/3a414ac90d07668987b15812cfacecea681df984))
    - Add TODO ([`f8e82e2`](https://github.com/AOSC-Dev/aoscpt/commit/f8e82e297f1d5244c03cedc2f137be347abf5604))
    - Improve local package reinstall logic ([`a7862b0`](https://github.com/AOSC-Dev/aoscpt/commit/a7862b065f6817c343bbbde40ee7af273ed118ba))
    - Support reinstall local package ([`0f1d7b7`](https://github.com/AOSC-Dev/aoscpt/commit/0f1d7b78cb1e0f1f90e43f054e526385c479b975))
    - Fix handle if package depends does not exist ([`f441a12`](https://github.com/AOSC-Dev/aoscpt/commit/f441a12f606fe24aa104bc9f677f2bb50965c9bd))
    - Update dep ([`fcb8501`](https://github.com/AOSC-Dev/aoscpt/commit/fcb85010376e8deb0179d1d447cabe8f9f4524ee))
    - Update README.md ([`7ad4f95`](https://github.com/AOSC-Dev/aoscpt/commit/7ad4f95785fd1463ec7fa764eab62b7937e7795d))
    - Update rust.yml ([`faea0c5`](https://github.com/AOSC-Dev/aoscpt/commit/faea0c56300474c1235143c828284bacf1069653))
    - Try fix ci ([`e1912a3`](https://github.com/AOSC-Dev/aoscpt/commit/e1912a322a7fc6f3531b0b087ce7beb572610d48))
    - Add rust templete ([`3d64ab6`](https://github.com/AOSC-Dev/aoscpt/commit/3d64ab6700898fd2f4b82cc80db5e115e54b9da1))
    - Add 'oma refresh' tips to tell user can upgradable and auto removable package ([`a2cd0d7`](https://github.com/AOSC-Dev/aoscpt/commit/a2cd0d74e67c86ea937f348a38640c4668309dc2))
    - Bump version to 0.1.0-alpha.4 ([`103d9af`](https://github.com/AOSC-Dev/aoscpt/commit/103d9af49716a724f0c5d7d5d17c53f12668219b))
    - Use cargo clippy ([`7495109`](https://github.com/AOSC-Dev/aoscpt/commit/7495109719c1b261c341709c5a504ca6107db35c))
    - Update README.md ([`10b7031`](https://github.com/AOSC-Dev/aoscpt/commit/10b7031d97b57421afe473bcfaf34a6cec5599e6))
    - Update README.md ([`11581bb`](https://github.com/AOSC-Dev/aoscpt/commit/11581bb576a0410bf84d8ba34ceb7f023353fb5c))
    - Action add --reinstall argument for install subcommand ([`c833308`](https://github.com/AOSC-Dev/aoscpt/commit/c8333086d5cb5c48a4ebf607213058b9e5628d3b))
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
    - Bump version to 0.1.0.alpha.3 ([`5269b3a`](https://github.com/AOSC-Dev/aoscpt/commit/5269b3a778e94b7b94461883ee572783138f26a0))
    - List function improve code style ([`fe93a48`](https://github.com/AOSC-Dev/aoscpt/commit/fe93a4893ab5acc53238cdc70f0597e5d88e107d))
    - Fix list installed display logic ([`34a1c9d`](https://github.com/AOSC-Dev/aoscpt/commit/34a1c9d71b9aafd6a825aaaf1723adc5fc755ed1))
    - List display package arch ([`3e7b99e`](https://github.com/AOSC-Dev/aoscpt/commit/3e7b99e38168891bf1ec32ce11d0124df601ce1b))
    - Fix next line output logic ([`79a1387`](https://github.com/AOSC-Dev/aoscpt/commit/79a1387b98365135cf04b14dc703db4dc832acd1))
    - Sort output order and add --installed argument ([`0b84a91`](https://github.com/AOSC-Dev/aoscpt/commit/0b84a911028dccfc600fff405366e35000102883))
    - Fix list preformance and style ([`5c05d54`](https://github.com/AOSC-Dev/aoscpt/commit/5c05d54e624ccd306a557b407fc31bb31acec60f))
    - Revert "Revert "pager: handle ctrlc exit status"" ([`1806e3a`](https://github.com/AOSC-Dev/aoscpt/commit/1806e3a4686489185e269165f4d15ebfac2add00))
    - Revert "pager: handle ctrlc exit status" ([`0852f93`](https://github.com/AOSC-Dev/aoscpt/commit/0852f93afd3167eb383758394c7f0e5f3e17a18c))
    - Use cargo clippy ([`06d773f`](https://github.com/AOSC-Dev/aoscpt/commit/06d773f4c90978c1c4e135c476470ca839ece053))
    - Handle ctrlc exit status ([`2e28a7c`](https://github.com/AOSC-Dev/aoscpt/commit/2e28a7c902c3a2fb79d837625e6ea7d351c51271))
    - Buml ver to 0.1.0-alpha.2 ([`0f35967`](https://github.com/AOSC-Dev/aoscpt/commit/0f3596776e61d61dd7c89a3c25d248cb57b925da))
    - Fix list display ([`d741253`](https://github.com/AOSC-Dev/aoscpt/commit/d741253d4f722c5db724712761599d1568de39e8))
    - Improve flat/non-flat mirror refresh logic again ([`131e723`](https://github.com/AOSC-Dev/aoscpt/commit/131e723592ae1771694d538320bd5d749b351e4b))
    - Improve flat/non-flat mirror refresh logic ([`224baa9`](https://github.com/AOSC-Dev/aoscpt/commit/224baa9afb74d4631645f1382711f1a0ab422d92))
    - Fix non-flat local mirror refresh logic ([`cebf083`](https://github.com/AOSC-Dev/aoscpt/commit/cebf0830c2c64c861a40b9467aab7f1cc23ac3c7))
    - Remove useless dbg ([`f2ea925`](https://github.com/AOSC-Dev/aoscpt/commit/f2ea925ff3d71feabe8cc5b93c617c95250c7db7))
    - Fix flat repo refresh logic ([`e9c5ed4`](https://github.com/AOSC-Dev/aoscpt/commit/e9c5ed49884cd04eddb5ceaf1ef51dca765c31fc))
    - Fix update_db checksum logic ([`ce3c905`](https://github.com/AOSC-Dev/aoscpt/commit/ce3c905cecec29c8ab78001c1b758d681e94333f))
    - Use cargo clippy ([`05d5fcd`](https://github.com/AOSC-Dev/aoscpt/commit/05d5fcddd1c7ac5456660b8248b8662f5d3852b6))
    - Support flat and local mirror ([`e122fbc`](https://github.com/AOSC-Dev/aoscpt/commit/e122fbc9c0148386683bacfb0718589a6a5a768c))
    - Fix APT-Source field display ... ([`eba69a3`](https://github.com/AOSC-Dev/aoscpt/commit/eba69a38f1f57b790894cab31ec10e8d94b76266))
    - Fix area/section/package line ([`11680fd`](https://github.com/AOSC-Dev/aoscpt/commit/11680fd5ab84541e0f996348bdb49109734db451))
    - Correct a typo in search.rs ([`30f720e`](https://github.com/AOSC-Dev/aoscpt/commit/30f720e3cd02b236f3ac2c2a76261dba9f91b694))
    - Adapt command-not-found subcommand ([`8a7ecd8`](https://github.com/AOSC-Dev/aoscpt/commit/8a7ecd884b319d54297b22ee14e81d693edc4e49))
    - Add 'command-not-found' subcommand ([`c73af01`](https://github.com/AOSC-Dev/aoscpt/commit/c73af01567cb41424d516fd6b042b25751b1a845))
    - Remove useless char ([`af00ee7`](https://github.com/AOSC-Dev/aoscpt/commit/af00ee77ef5e8d5ddb7cfc6d40948a6f38aaa4d9))
    - Fix contents line is pkg_group logic ([`b93ea53`](https://github.com/AOSC-Dev/aoscpt/commit/b93ea53fa7955c821e070dc5f5603b832e318772))
    - Use anyhow to handle non-apt errors in cache.upgrade ([`fe229e1`](https://github.com/AOSC-Dev/aoscpt/commit/fe229e1156810e1ef00b5930a5f328002940f729))
    - Use thiserror to control retry ([`813aa51`](https://github.com/AOSC-Dev/aoscpt/commit/813aa511fda8f0232c4f562ec7fc2b8ec9930092))
    - Fix a typo ([`32657b9`](https://github.com/AOSC-Dev/aoscpt/commit/32657b9a326a9d5b93458479266ab40c5f9bf123))
    - Fix size_checker in chroot ([`350835b`](https://github.com/AOSC-Dev/aoscpt/commit/350835baf02b8fa04d1a71cd7d3fe5531437c342))
    - Check available space before download and installation ([`1f67a9d`](https://github.com/AOSC-Dev/aoscpt/commit/1f67a9d68987a2f99543b88989215a723a8f2c81))
    - Size byte display B -> iB ([`49082ee`](https://github.com/AOSC-Dev/aoscpt/commit/49082ee51f38e6cf56a68cb24d0e8faed7c6f7c2))
    - Fix install size calculate display ([`f970705`](https://github.com/AOSC-Dev/aoscpt/commit/f97070523b19d16523870c57ff26f5202fdd5474))
    - Rm useless line ([`a0aa5ba`](https://github.com/AOSC-Dev/aoscpt/commit/a0aa5ba604fc2928b1d6e17cfecbf8c746c13e70))
    - Add 'mark' command ([`f714c96`](https://github.com/AOSC-Dev/aoscpt/commit/f714c963bbc453c89828bd4513a871a0bdf8aa23))
    - Use cargo clippy ([`7850ce3`](https://github.com/AOSC-Dev/aoscpt/commit/7850ce3df097280dcd5d1ddc5527ec8ee00acdb6))
    - Set section bases package as blue color ([`f7ef7e6`](https://github.com/AOSC-Dev/aoscpt/commit/f7ef7e65608bf17b3aca9fa45279a896314ac437))
    - Move unlock step from try_main to main ([`e3e2bd7`](https://github.com/AOSC-Dev/aoscpt/commit/e3e2bd739d9a37256811b1e02196184ba4fc466c))
    - Unlock_oma with has error ([`7307840`](https://github.com/AOSC-Dev/aoscpt/commit/7307840cdd446b9daecfbb6475ec138a1efb39cd))
    - Fix autoremove step ([`38e8eaf`](https://github.com/AOSC-Dev/aoscpt/commit/38e8eaf0a94057d8b149dea7415f3465c94ab638))
    - Remove useless line ([`31bdfea`](https://github.com/AOSC-Dev/aoscpt/commit/31bdfea72798adffbcc6c893ea6c54138e0b250f))
    - Move cache.resolve(true) to apt_handle function inner ([`c624f5c`](https://github.com/AOSC-Dev/aoscpt/commit/c624f5c5d424a47d6770e7f12b83244acecf6ca6))
    - Move cache.resolve(true) to apt_install function inner ([`82ff44f`](https://github.com/AOSC-Dev/aoscpt/commit/82ff44f216985f5b15f43925a2d40d13aa912b3d))
    - Add 'pick' subcommand ([`9cd02f6`](https://github.com/AOSC-Dev/aoscpt/commit/9cd02f60d81e87a61f1e9ca343196d5ebdd4e30c))
    - Add oma install --dbg(--install-dbg) argument ([`2c4c9e3`](https://github.com/AOSC-Dev/aoscpt/commit/2c4c9e3a36c0bd17e7e76b9ffc5b4c5e0e7263f3))
    - Use search::show_pkgs to get pkg info, improve code style ([`d7a51c7`](https://github.com/AOSC-Dev/aoscpt/commit/d7a51c71cc16ccbb97399d3ddb4f710d38298166))
    - Improve upgradable ui style ([`c03d4cc`](https://github.com/AOSC-Dev/aoscpt/commit/c03d4cc04ea8f7ca4d969445e5a8e89b35c8cb20))
    - Add error output if contents not exist ([`7d66d77`](https://github.com/AOSC-Dev/aoscpt/commit/7d66d7775f1c56ccc4bcd95407df8a232ff5212f))
    - Improve output result ([`b562788`](https://github.com/AOSC-Dev/aoscpt/commit/b562788198b26ab5ba7cda10bd5a4c6a21819a9e))
    - Fix list-files package display ([`8e63ac6`](https://github.com/AOSC-Dev/aoscpt/commit/8e63ac6269959f0f97502d06a476e1b7797a391d))
    - Use stderr to output info/warn/debug/dueto ... ([`37f4ff1`](https://github.com/AOSC-Dev/aoscpt/commit/37f4ff109457765216c7c59c042b108976f149fe))
    - Use rust-apt https://gitlab.com/volian/rust-apt/ newest git ([`a136e82`](https://github.com/AOSC-Dev/aoscpt/commit/a136e82a8d0b386a39ad8d396c36f6afb0122303))
    - Abstract is_root function ([`80ac7ac`](https://github.com/AOSC-Dev/aoscpt/commit/80ac7ac85368afbd259be7539f11543b47045d41))
    - Revert update_db feature ... ([`73cde89`](https://github.com/AOSC-Dev/aoscpt/commit/73cde8941210379abf7f2f1b572cfd064ddf8bb0))
    - Remove dpkg ctrlc handler ... ([`82ecc5f`](https://github.com/AOSC-Dev/aoscpt/commit/82ecc5f8ca5e8de1719cb6a78583a2803524c5c2))
    - Improve lock/unkock logic from szclsya/sasm ([`17e7c5b`](https://github.com/AOSC-Dev/aoscpt/commit/17e7c5b1c635c3708284e8f2d5027cf2350d8dd4))
    - Lock ctrlc in dpkg install ([`1dbc932`](https://github.com/AOSC-Dev/aoscpt/commit/1dbc93277c75f65ccf19cb40b939c4669ca40bfd))
    - Action, main: add lock_oma and unlock_oma function to lock/unlock oma in install/remove/upgrade ([`840ac52`](https://github.com/AOSC-Dev/aoscpt/commit/840ac5234a1fa686e1f3f12bbfaa6033ff37e693))
    - If user run oma provides/list-files, force run update_db ([`ff39098`](https://github.com/AOSC-Dev/aoscpt/commit/ff390988defe99aefdd68c753512ee4b9e206905))
    - If local contents file is empty, run update db ([`5bde010`](https://github.com/AOSC-Dev/aoscpt/commit/5bde010f0cbe0e3bec4c9ebcd95b62a48c096a3f))
    - Use regex::escape to replace my escape step ([`37a5b65`](https://github.com/AOSC-Dev/aoscpt/commit/37a5b651de67bceda00950201000895f7951c96e))
    - Add dependencies ([`d02d073`](https://github.com/AOSC-Dev/aoscpt/commit/d02d073945910501d8d4e7dd4b48e43a39d0db1d))
    - Add Japanese spelling for Omakase ([`be1d647`](https://github.com/AOSC-Dev/aoscpt/commit/be1d647e135f9c754ecc98f84a86a0652b95a7d6))
    - Add a definition for Omakase ([`5f62f4b`](https://github.com/AOSC-Dev/aoscpt/commit/5f62f4b921c4a1e649420eece1c4a1e4d10fe5a3))
    - Update ([`006b060`](https://github.com/AOSC-Dev/aoscpt/commit/006b0602ca42b51edafc48c7b0e2451b3950782e))
    - Rename to oma, fix grammar ([`a1d75e1`](https://github.com/AOSC-Dev/aoscpt/commit/a1d75e183c26717ab98fbadf2430050fa89dff32))
    - Bump version to 0.1.0-alpha.1 ([`0a93298`](https://github.com/AOSC-Dev/aoscpt/commit/0a93298be9d5ec3440d614784bd294ee3bf96faf))
    - Use cargo clippy ([`abbb1fc`](https://github.com/AOSC-Dev/aoscpt/commit/abbb1fc8cc537c1b65e64ec05d09904d754e5f96))
    - Fix regex security issue ([`e5c38a7`](https://github.com/AOSC-Dev/aoscpt/commit/e5c38a7efcf8573126f745b590ce94c0011932ba))
    - Fix-broken add packages_download step ([`6951391`](https://github.com/AOSC-Dev/aoscpt/commit/6951391f9bfda969f32931c400edf870d4563fa4))
    - Rename list-file to list-files ([`de2dc71`](https://github.com/AOSC-Dev/aoscpt/commit/de2dc71d12498195cf3b557c061cb241f0233238))
    - Fix-broken command add pending operations page ([`465afa7`](https://github.com/AOSC-Dev/aoscpt/commit/465afa7355bbe4c665796df5ae39f906b5997954))
    - Add fix-broken command ([`e983828`](https://github.com/AOSC-Dev/aoscpt/commit/e9838282ae03398f1c14aaf880b11dfb5a421cf7))
    - Improve error return again ([`c6d4f2d`](https://github.com/AOSC-Dev/aoscpt/commit/c6d4f2dfed897c077a3c0ae2aea93507f44b928e))
    - Improve error return ([`9d30ed5`](https://github.com/AOSC-Dev/aoscpt/commit/9d30ed53fccaff7fba050179e319ef9f180d7ad6))
    - Fix rg error return ([`e00ea67`](https://github.com/AOSC-Dev/aoscpt/commit/e00ea67ad6320937bc97914fca561c8d2af4f0b4))
    - Rename search-file command to provides ([`a047d80`](https://github.com/AOSC-Dev/aoscpt/commit/a047d802569d054c1b65af655fdd6734c0d8b266))
    - Move root check to need root function ([`bc79fe3`](https://github.com/AOSC-Dev/aoscpt/commit/bc79fe397873c860876dfd28b185eb9ed879e756))
    - Download command only download package ifself ([`80a3127`](https://github.com/AOSC-Dev/aoscpt/commit/80a3127af00e1d240855969e5a1653555d6dbab8))
    - Add 'download' command ([`85b5aff`](https://github.com/AOSC-Dev/aoscpt/commit/85b5aff00be3705bf658a09a45c31d24d4a3837b))
    - Move update alias to refresh command ([`5a4eefb`](https://github.com/AOSC-Dev/aoscpt/commit/5a4eefb38f61b51ef1dd026f0c129b7cd0066020))
    - Fix oma_style_pb in terminal length < 100 run ([`dc4cf61`](https://github.com/AOSC-Dev/aoscpt/commit/dc4cf61e51354b2f91a8f0b30f297a7e6b199f92))
    - Use ripgrep cli to read contents ... ([`892ce4c`](https://github.com/AOSC-Dev/aoscpt/commit/892ce4c69e7778fdff6c6905e2eb42b5d0b10177))
    - Improve code style again ([`7861f8b`](https://github.com/AOSC-Dev/aoscpt/commit/7861f8b4f2465d928f072527141893e6cafd59b2))
    - Improve code style ([`4a50091`](https://github.com/AOSC-Dev/aoscpt/commit/4a5009138cbe7805b2797f85c305a8b87ef236ac))
    - Improve output result ([`84c4591`](https://github.com/AOSC-Dev/aoscpt/commit/84c4591a454b6e11c503c2cf473df0b0e9f3edf5))
    - Improve contents logic ([`6270f27`](https://github.com/AOSC-Dev/aoscpt/commit/6270f2758dda2c8e1325ec1798bc8147089bf35f))
    - Done, but so slow ([`935a2f1`](https://github.com/AOSC-Dev/aoscpt/commit/935a2f19a9129a7ee5f499e7f62c3895c158e1ef))
    - Fix upgradable output ([`d4d989b`](https://github.com/AOSC-Dev/aoscpt/commit/d4d989b374705cda90987ce11f2ea80649b5bafa))
    - Fix a typo ([`f66528d`](https://github.com/AOSC-Dev/aoscpt/commit/f66528dbf97dda74598984d4ae3532865b55223e))
    - Rust-apt use my fork to fix search/show panic ... ([`1d14a40`](https://github.com/AOSC-Dev/aoscpt/commit/1d14a40f141bd0d6c37c9edfeb024e2c4cc307b0))
    - No need to use indexmap ([`8977689`](https://github.com/AOSC-Dev/aoscpt/commit/897768954112b2ef36a9e841550a4cd3598b6b0b))
    - Fix local install again ... ([`9ed8f89`](https://github.com/AOSC-Dev/aoscpt/commit/9ed8f895bb23713bc637d39fa5946666c7ee1614))
    - Fix install with branch and version ([`a606e36`](https://github.com/AOSC-Dev/aoscpt/commit/a606e361c9e2f6fefe977eef5ce51b45dccc9efc))
    - Improve search ui style ([`300f311`](https://github.com/AOSC-Dev/aoscpt/commit/300f311b293a7dddb481673c3d112af4a4f84b78))
    - Add search feature ([`b0142fb`](https://github.com/AOSC-Dev/aoscpt/commit/b0142fb66b327fe1f631d5c216d978eca0f0bb96))
    - Remove useless display version info ([`0046768`](https://github.com/AOSC-Dev/aoscpt/commit/00467685a532424d82cb4c0de6262ba3e98c3f2d))
    - Add oma show command ([`71b48c2`](https://github.com/AOSC-Dev/aoscpt/commit/71b48c2015f481f452f0953e0896402b3895ad78))
    - Cargo fmt ([`8147605`](https://github.com/AOSC-Dev/aoscpt/commit/81476054159b3b9cc935393b642b02671eb8b192))
    - Fix libapt get url ([`c2bd3eb`](https://github.com/AOSC-Dev/aoscpt/commit/c2bd3eb3ccc2fc43bba42a77602f4200bc3005d6))
    - Multi thread download InRelease files ([`a92aab4`](https://github.com/AOSC-Dev/aoscpt/commit/a92aab480c1c05328acae9fe55016f0aff7c34dc))
    - If local install error display file path ([`3cb03aa`](https://github.com/AOSC-Dev/aoscpt/commit/3cb03aae0e36238b157b12bb088abae37e66c016))
    - Install_handle add comment ([`1b683a9`](https://github.com/AOSC-Dev/aoscpt/commit/1b683a9982aaa0bfb1120905fcdfa155879cd981))
    - Fix local install .deb package ([`92cb689`](https://github.com/AOSC-Dev/aoscpt/commit/92cb689dc9159b74324c38fdad99aae72f38e9ad))
    - Add oma info and root check ... ([`2a11445`](https://github.com/AOSC-Dev/aoscpt/commit/2a114455a39402a5a77149eba747a07aff0c09de))
    - Update and set rust-apt to crate version ([`59d47a7`](https://github.com/AOSC-Dev/aoscpt/commit/59d47a77f29e8cbd3854333993124ce23839dca4))
    - Correct a typo in download.rs ([`139cdf1`](https://github.com/AOSC-Dev/aoscpt/commit/139cdf14ef7871c765314de927f1ad50c405ea1d))
    - Use info to tell user what package is installed ([`11296f3`](https://github.com/AOSC-Dev/aoscpt/commit/11296f38e6b30470f7339e7eb5e0c36e0d6b19c6))
    - Improve install tips ... ([`a5eb1f1`](https://github.com/AOSC-Dev/aoscpt/commit/a5eb1f11810f18f05d339d54165dadfdab422eba))
    - Use cargo clippy ([`54e6e7f`](https://github.com/AOSC-Dev/aoscpt/commit/54e6e7f373f9c3e966be16dbc94563ad260d948c))
    - Abstract some step to try_download ([`0a8231e`](https://github.com/AOSC-Dev/aoscpt/commit/0a8231e6c65fdf3f5dbabe1f47b45af307ab3f94))
    - Code clean up again ([`db0a78a`](https://github.com/AOSC-Dev/aoscpt/commit/db0a78a4d99961e80bc73ae7f0b9afe1d644f842))
    - Clean again ([`5789940`](https://github.com/AOSC-Dev/aoscpt/commit/57899403ca491166c9a695ad50a74fca24fa0358))
    - Use cargo clippy ([`4c9930b`](https://github.com/AOSC-Dev/aoscpt/commit/4c9930b49dbcc0c88868bf9c33f959d29b4cb6a5))
    - Code all clean up ([`023080f`](https://github.com/AOSC-Dev/aoscpt/commit/023080f6ba870a461a07ef70101d969efe342d2b))
    - Improve download logic ... ([`4f4b0af`](https://github.com/AOSC-Dev/aoscpt/commit/4f4b0af7d0fc22aa64d54cc4eff26f96470ef3ea))
    - Add install .deb from local ([`0eb592c`](https://github.com/AOSC-Dev/aoscpt/commit/0eb592c20f4e29e96db3559ce1bb9e42256951e0))
    - Improve display package version logic ([`e68970b`](https://github.com/AOSC-Dev/aoscpt/commit/e68970b2b3002988b462d61ef60b98402da1f47a))
    - Improve download message ([`63477bf`](https://github.com/AOSC-Dev/aoscpt/commit/63477bf49058f7455bd54e0d840bf915a36531c8))
    - Fix color in non-global bar ([`14a2d05`](https://github.com/AOSC-Dev/aoscpt/commit/14a2d050f193ba0533969eabd282259dfe2b111a))
    - Fix global bar number color ([`02b919b`](https://github.com/AOSC-Dev/aoscpt/commit/02b919bb33d375a51b27f360e7681860382d1de9))
    - If error exit code 1 ([`45d3de8`](https://github.com/AOSC-Dev/aoscpt/commit/45d3de844455d2a09adff33f7dc7b35fde0fa919))
    - Improve error handle ([`6d0168a`](https://github.com/AOSC-Dev/aoscpt/commit/6d0168a65d187f4d39ca7b95fd7991107a74fa63))
    - Add a comment ([`86066fe`](https://github.com/AOSC-Dev/aoscpt/commit/86066fe7938332c1170d7786364e42b38827772c))
    - Handle file:// or cdrom:// mirror ([`2cdb7c8`](https://github.com/AOSC-Dev/aoscpt/commit/2cdb7c8ec64128caef2854f6f5c8734605514539))
    - Update rust-apt to new git commit ([`154ccb7`](https://github.com/AOSC-Dev/aoscpt/commit/154ccb7ca80060e68702beeb302e1b6a0cb02ffc))
    - Improve global progress bar ([`c65b9c5`](https://github.com/AOSC-Dev/aoscpt/commit/c65b9c58ea3f92cc11f0b4bce10d6e2da8cfc635))
    - Action, pager: improve omakase ui ([`7b606f7`](https://github.com/AOSC-Dev/aoscpt/commit/7b606f7bdde90a7e41d28c1ec2242cc1c26825c8))
    - Use more rust-apt library ([`320ec89`](https://github.com/AOSC-Dev/aoscpt/commit/320ec8904db17c343071b3eb6e3a09eec99cf5ed))
    - Use rust-apt to read all new and old pkg database ([`4b2b92d`](https://github.com/AOSC-Dev/aoscpt/commit/4b2b92dfe5251161d5fc09f4222ca4df61f501de))
    - Improve display result ([`661818a`](https://github.com/AOSC-Dev/aoscpt/commit/661818a80e93370d07033ee80536db1b4062c2bd))
    - Fix remove result wrong issue ... ([`5717149`](https://github.com/AOSC-Dev/aoscpt/commit/5717149aa40319896414a0dd24057274fe959c4c))
    - Remove useless function ([`9967e87`](https://github.com/AOSC-Dev/aoscpt/commit/9967e879e2183efc04bba46a6665c97c601ee6b3))
    - Fix cargo clippy ([`b0f5ac9`](https://github.com/AOSC-Dev/aoscpt/commit/b0f5ac9cd0f5e692efb019f07142308b95120201))
    - Use cargo clippy ([`86d6364`](https://github.com/AOSC-Dev/aoscpt/commit/86d6364ff64947a1b0d63c2e035c3c0d93225858))
    - Oma install support glob ... ([`e353223`](https://github.com/AOSC-Dev/aoscpt/commit/e353223aaf2dbb93fad9c1d5cbe175aef832b0f5))
    - Fix like oma upgrade fish=3.5.1-1 ([`ea26ae5`](https://github.com/AOSC-Dev/aoscpt/commit/ea26ae54b9f37eb55aa78bf68a977e8db538d74a))
    - Fix downgrade color ([`b4deefd`](https://github.com/AOSC-Dev/aoscpt/commit/b4deefd38d75393ce0d8c2c1cdc3688c19555347))
    - Protect mark_install with oma install PACKAGE/BRANCH ([`bbe0986`](https://github.com/AOSC-Dev/aoscpt/commit/bbe0986c66bee67665241983b6df83ea722cf512))
    - Support like oma upgrade meowdict ... ([`6a654b0`](https://github.com/AOSC-Dev/aoscpt/commit/6a654b01b89bc41e1e6b248a22101a6b9dc0fa47))
    - Fix display select package version ... ([`65345aa`](https://github.com/AOSC-Dev/aoscpt/commit/65345aad43e65bd64a4d2fd9c28d2e6d325bd323))
    - Fix package download with version ([`bd4cc8c`](https://github.com/AOSC-Dev/aoscpt/commit/bd4cc8c567a3dec4510d10d64c2f30a5c7ff9feb))
    - Use libapt-pkg to check download version ([`e551c8c`](https://github.com/AOSC-Dev/aoscpt/commit/e551c8c4df264f6b03bb1a1344c6e84abb2d8bf3))
    - Pager add download size and disk_size display ([`acc9b36`](https://github.com/AOSC-Dev/aoscpt/commit/acc9b36ada76d564baa40747299076cf4d15fcb6))
    - Fix packages_download ([`03a7f62`](https://github.com/AOSC-Dev/aoscpt/commit/03a7f6256ab6c5efd1baa90096c662c06581b8a8))
    - Add result display ... ([`ef3468e`](https://github.com/AOSC-Dev/aoscpt/commit/ef3468ec9690712125a61f4a7cf4151e350bc408))
    - Fix checksum eat memory issue ... ([`620abec`](https://github.com/AOSC-Dev/aoscpt/commit/620abec6dfe41d854a2fbf83eb7ed549a3ef72b6))
    - Add more ouput ([`d7a47f2`](https://github.com/AOSC-Dev/aoscpt/commit/d7a47f22e374472277a67ab85801b010e6073de9))
    - Add more info output ([`3eaffa1`](https://github.com/AOSC-Dev/aoscpt/commit/3eaffa1a2921586bb1f21cd47c140d12747a6972))
    - Pb display branch ([`26c7262`](https://github.com/AOSC-Dev/aoscpt/commit/26c7262b81ce8f9275e02c1bf95d628f3429fa0c))
    - Improve pb style ([`56ba0ef`](https://github.com/AOSC-Dev/aoscpt/commit/56ba0efa8ca337de4a1ac31a6dc7153524109857))
    - Add fetch database multiprogress ([`9d43bae`](https://github.com/AOSC-Dev/aoscpt/commit/9d43baed1cd41b1fdf1e1362cf5963017952af5d))
    - Set update feature subcommand name as upgrade ... ([`c48a9f9`](https://github.com/AOSC-Dev/aoscpt/commit/c48a9f970768364dc4c2ec890dde1056bb23efdc))
    - Add global progress bar to global download packages progress ([`c804d58`](https://github.com/AOSC-Dev/aoscpt/commit/c804d58436cd4f495b6d75dcee8419183d931b83))
    - Progressbar add count and len ([`ab1ff4f`](https://github.com/AOSC-Dev/aoscpt/commit/ab1ff4ff44a5aedebbfff8f3cecfe014399a49e1))
    - Set name as oma (Oh My Ailurus) ([`f35200f`](https://github.com/AOSC-Dev/aoscpt/commit/f35200ff50a6af7365296187c1cf6ee2b03f2f95))
    - Handle pb message if text width > 48 ([`896a416`](https://github.com/AOSC-Dev/aoscpt/commit/896a41658025f2f9d939091ede7418b6830288fb))
    - Fix download filename ([`abda763`](https://github.com/AOSC-Dev/aoscpt/commit/abda7634e77560f2fc9f761013e16303b4dcc725))
    - Add download thread limit ([`9d18cd4`](https://github.com/AOSC-Dev/aoscpt/commit/9d18cd4d6321dd787f28d7a9e72d197f1a5156fa))
    - Use MultiProgress to display download progress ([`904685f`](https://github.com/AOSC-Dev/aoscpt/commit/904685f69c9ced1f31c5cb258f355f5c11f5034d))
    - Add error check ([`ed996da`](https://github.com/AOSC-Dev/aoscpt/commit/ed996daa4a0ad1d3a72ac7b72494bf4997e6e8b6))
    - Fix progress bar file size ([`8de611f`](https://github.com/AOSC-Dev/aoscpt/commit/8de611fbc1e242c74a53de2229e9adb2b6689ce6))
    - Use async to partial download ([`4d75c7b`](https://github.com/AOSC-Dev/aoscpt/commit/4d75c7b4c7ebbc19f1bb0fe2cfe395ead71aa5b0))
    - Add refresh to only update package database ([`0e295e8`](https://github.com/AOSC-Dev/aoscpt/commit/0e295e8b834603a8bf0b43fa94c8470e8f2a8248))
    - Improve download code logic ([`53b3010`](https://github.com/AOSC-Dev/aoscpt/commit/53b3010439a7b55051f1b30c27700d247a9ffc2f))
    - Fix if /var/lib/apt doesn't not exist ([`8757a02`](https://github.com/AOSC-Dev/aoscpt/commit/8757a027ec7fb2a6d0fdc3a1a55ed8ab588ebb23))
    - Learn omakase ([`90cb592`](https://github.com/AOSC-Dev/aoscpt/commit/90cb592cc07cd1e1b2f0fcc76952d8735255ae22))
    - Fix a bug, if arch = xxxnoarch ([`49fc96d`](https://github.com/AOSC-Dev/aoscpt/commit/49fc96d6cfa29423f14945ad21f744d188705b46))
    - Fix filename to compatible apt download ([`0599374`](https://github.com/AOSC-Dev/aoscpt/commit/05993741172c41d9a78e52b9a6beb1df7ef6bc31))
    - Add exit code ([`b23e90d`](https://github.com/AOSC-Dev/aoscpt/commit/b23e90dd92db4c481c89bec5a836985b3eb75ea9))
    - Use cargo clippy ([`4896e4a`](https://github.com/AOSC-Dev/aoscpt/commit/4896e4a92b9c05c48524849595bae22cfcba4cf4))
    - Improve retry mechanism ([`0c9ecc6`](https://github.com/AOSC-Dev/aoscpt/commit/0c9ecc678d9819f23cfd6209b4d33ad0c83ea38e))
    - Download, formatter, update: add more comment ([`b826165`](https://github.com/AOSC-Dev/aoscpt/commit/b826165bba6b6f61b658d3bf2296a62396ee0c08))
    - Run cargo clippy ([`e46d53e`](https://github.com/AOSC-Dev/aoscpt/commit/e46d53e87b45b40584e2f4523a7408b3d10758f1))
    - Fix autoremove ([`34330ec`](https://github.com/AOSC-Dev/aoscpt/commit/34330ecc236e2a85b735bbef2794ee9bdd4a126b))
    - Use clap to handle subcommand ([`4426499`](https://github.com/AOSC-Dev/aoscpt/commit/44264992a753feba5ab80e95aee6a5a89c861ca6))
    - Improve install/update feature ([`aadf76b`](https://github.com/AOSC-Dev/aoscpt/commit/aadf76bf87501f678751f9fbcff38403800e8592))
    - Add remove feature ([`19ee335`](https://github.com/AOSC-Dev/aoscpt/commit/19ee335b14a5f4c0cef728ad7a704b4bfcbc4f1f))
    - Abstract apt install step to apt_install function ([`4537a17`](https://github.com/AOSC-Dev/aoscpt/commit/4537a17c3b3cc85d826ff602a044873715afbf5f))
    - Support apt install fish/stable ([`8d58616`](https://github.com/AOSC-Dev/aoscpt/commit/8d586168561a8b21e0bc7c5771c8f89212a66269))
    - Fix comment typo ([`cc15fae`](https://github.com/AOSC-Dev/aoscpt/commit/cc15fae459ad8cff3505abe20bb66dd7029d3444))
    - Update and install done ([`2be8ef9`](https://github.com/AOSC-Dev/aoscpt/commit/2be8ef9beec651c6d733961c7ae3fcffbe653f45))
    - Use rust-apt to calculate dep ([`79db780`](https://github.com/AOSC-Dev/aoscpt/commit/79db780e333d718244b202f9f1e4e53479d89d80))
    - Use debcontrol to replace 8dparser ([`be660cf`](https://github.com/AOSC-Dev/aoscpt/commit/be660cf3eb89fe2339fc753d846539a3df168604))
    - New, this is User Action Control ([`749eebf`](https://github.com/AOSC-Dev/aoscpt/commit/749eebf42a2f0727f9e9ed765fa2048df7b35313))
    - Fill of remove and purge feature ([`75ad9a6`](https://github.com/AOSC-Dev/aoscpt/commit/75ad9a6911e3e8b2115b566cc20224053fae9e3d))
    - Improve abstraction ([`d7e0541`](https://github.com/AOSC-Dev/aoscpt/commit/d7e0541c1eefda41fca43f1178eb1ab345cb2203))
    - Dpkg_executer: retry 3 times to workround dpkg trigger cycle ([`b6e69ed`](https://github.com/AOSC-Dev/aoscpt/commit/b6e69ed25af683d624abe88268a5dc7157578d4c))
    - All done ([`a2b921e`](https://github.com/AOSC-Dev/aoscpt/commit/a2b921e76ff97d598cc47f9daa3f2c3ce7d15df8))
    - Download, update: all done ([`a7e5f4d`](https://github.com/AOSC-Dev/aoscpt/commit/a7e5f4de7774a5ee9dcdca67f81c3557cc3ec650))
    - Update, blackbox, verify: add more comment ([`701e8d9`](https://github.com/AOSC-Dev/aoscpt/commit/701e8d991676374dc5a04b9d5059d713e9c66ee0))
    - Add debug info ([`94358c8`](https://github.com/AOSC-Dev/aoscpt/commit/94358c81cdefd194473ec751dd321bd164e9c140))
    - Add apt -s info parse ([`fa7331b`](https://github.com/AOSC-Dev/aoscpt/commit/fa7331b6b86ac3287693c615eaf599ef5130d0de))
    - Add AptAction::Purge ([`663239e`](https://github.com/AOSC-Dev/aoscpt/commit/663239ecbe758bbb68beea0d8ff4b9aa6a04763c))
    - Add apt_calc function ([`a7e0b0c`](https://github.com/AOSC-Dev/aoscpt/commit/a7e0b0cb4acc3fca8aa188f97d364d5b2f4d17e0))
    - UpdatePackage add some size field ([`152e8fc`](https://github.com/AOSC-Dev/aoscpt/commit/152e8fc9b5c3dfc45c77e97beeca7c760694009b))
    - UpdatePackage add filename and from field; fix var name mistake ([`3b40d16`](https://github.com/AOSC-Dev/aoscpt/commit/3b40d166bbc1d8cb3ebbfeaa0a0b0853c5f6df66))
    - Handle if apt Installed-Size and dpkg mismatch ([`dcdd435`](https://github.com/AOSC-Dev/aoscpt/commit/dcdd4357bb88d51d234b76dbeefd87de52f00f7d))
    - All done ([`786d02b`](https://github.com/AOSC-Dev/aoscpt/commit/786d02b6569651433d99be7a84a9d5c5b1869d80))
    - Fill of download package list and contents ([`1a5c568`](https://github.com/AOSC-Dev/aoscpt/commit/1a5c5688b2c746141d848c73d3612f544468f620))
    - Use vector to put ChecksumItem ([`467a501`](https://github.com/AOSC-Dev/aoscpt/commit/467a501d425d404259a7d2a8c9023b0d61beeeae))
    - Init ([`b65db57`](https://github.com/AOSC-Dev/aoscpt/commit/b65db576b86e7ce106119d600cfcfe52260f838b))
    - Initial commit ([`b7652ba`](https://github.com/AOSC-Dev/aoscpt/commit/b7652ba83f650bdc87d19e273566ee6bd88aa78d))
</details>


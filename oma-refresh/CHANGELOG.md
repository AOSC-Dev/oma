# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.4.0 (2023-09-06)

### New Features

 - <csr-id-a0750502605cabb6d7385f1cbc96edf639324cb5/> Add DownloadEvent::AllDone to allow control global progress bar finish and clear
 - <csr-id-13018326745688027422575eb5a364a050c4c691/> Add --no-progress option to no output progress bar to terminal

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-topics v0.1.2, oma-refresh v0.4.0 ([`14edf42`](https://github.com/AOSC-Dev/oma/commit/14edf42022306405c9e4583b3445d3fd573a100e))
    - Release oma-fetch v0.3.0, safety bump 2 crates ([`0959dfb`](https://github.com/AOSC-Dev/oma/commit/0959dfb5414f46c96d7b7aa39c485bdc1d3862de))
    - Add DownloadEvent::AllDone to allow control global progress bar finish and clear ([`a075050`](https://github.com/AOSC-Dev/oma/commit/a0750502605cabb6d7385f1cbc96edf639324cb5))
    - Add --no-progress option to no output progress bar to terminal ([`1301832`](https://github.com/AOSC-Dev/oma/commit/13018326745688027422575eb5a364a050c4c691))
</details>

## v0.3.0 (2023-09-05)

<csr-id-11fd26ec2732fe5be0137601fe3388a1f9aaf014/>
<csr-id-adffcfbc7c19c8e559ba45f991cb4af55f1c8224/>
<csr-id-57003169329e01d60172d3531e7f3817bacf46da/>
<csr-id-922fb8aa093a6050c4fdc848f2e5fab369db6095/>
<csr-id-0f2613cb0419e58d10a6bf453d4e4417b02f6e4a/>
<csr-id-57fcaa531bc827a8661cf2a4f0f8a50c39289277/>
<csr-id-0e14c25a9f5ad34da79df93cd3e686e81323f320/>
<csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/>
<csr-id-f875de43cb615ab3f620e5e1c6989b3f07c651be/>
<csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/>
<csr-id-1943b764ee60248d6c02f820e50cdc1e5d73716b/>
<csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/>
<csr-id-2768dc5e3070661cc797121575c25ba88819d8a9/>
<csr-id-1a3d60e8665faf452a217a478bf0b1c7ce3e445b/>
<csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/>
<csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/>
<csr-id-5622e3699691081f0de4466379c14bc539e69c11/>
<csr-id-20818083ca01c6209cd28d5279637d7e21422192/>
<csr-id-a3c910b6cf9ef432f2b93e38adb61fd6b021d819/>
<csr-id-88efbe1e674c3a3030144ad3b0690d1e2095cdaf/>
<csr-id-1e637a4c0b535d095c8f35229a8ce910c3a163a6/>
<csr-id-0e32ceead5727a79c2841c5d137fd32a8cd88753/>
<csr-id-65fa216e325fe96f964a31c47d500e3197c9a269/>
<csr-id-20ee30139b0da28db1d422d4605cbe3582a71e15/>
<csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/>
<csr-id-b8b68685187bf1740c91372b9aa73bb777e3d134/>
<csr-id-86d65eb054576ec4e2fea52d3722beb7dc8c0c32/>

### Chore

 - <csr-id-11fd26ec2732fe5be0137601fe3388a1f9aaf014/> Update nix to 0.27
 - <csr-id-adffcfbc7c19c8e559ba45f991cb4af55f1c8224/> Switch to chrono
 - <csr-id-57003169329e01d60172d3531e7f3817bacf46da/> Adapt tokio enabled feature
 - <csr-id-922fb8aa093a6050c4fdc848f2e5fab369db6095/> Adjust some deps
 - <csr-id-0f2613cb0419e58d10a6bf453d4e4417b02f6e4a/> Use oma-debcontrol crate (own fork)
 - <csr-id-57fcaa531bc827a8661cf2a4f0f8a50c39289277/> Inquire -> oma-inquire
 - <csr-id-0e14c25a9f5ad34da79df93cd3e686e81323f320/> Drop useless dep
 - <csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/> No need to use tracing
 - <csr-id-f875de43cb615ab3f620e5e1c6989b3f07c651be/> Fmt example

### Documentation

 - <csr-id-54bc679fe098faceea2ed461f5da6178b34330f0/> Add changelog

### Chore

 - <csr-id-b8b68685187bf1740c91372b9aa73bb777e3d134/> Fill some dep version
 - <csr-id-86d65eb054576ec4e2fea52d3722beb7dc8c0c32/> Add license and desc

### New Features

 - <csr-id-9665cd4b3e50ca8fbe18c388bd3c75f6c4b81b2e/> Add topic closing message
 - <csr-id-94687df792f92c1b717c81ff31b8e803aa5fb125/> Do not fetch database with the same content but in a different compression format
 - <csr-id-69a17fe9bbc77374992e617a62db681bb7a1bca6/> Use feature to select abstract code
 - <csr-id-870fcaeeafdf83a4e2e54d07f81a59e38c05ec9b/> Refresh done no need to manual fsync
 - <csr-id-578b5e39890ec6a53b378c56201b0e179107f451/> Add mark_version_status function
 - <csr-id-67c9c44809f1ae091913d851fc2e8b18163eb037/> Download compress file after extract
 - <csr-id-df69c9714ffb218ba8963d39ef63bd5cedecf015/> Checksum pure database
 - <csr-id-bf04133b4335ac1de687634a393bf5f2685d9e5f/> Add translate
 - <csr-id-3ee53e62af52f374b32cbbf86e60a591547ca17a/> After the database refresh is complete fsync
 - <csr-id-5cedd38dc69b89403b8f13aa8b68a6360481991b/> Init

### Bug Fixes

 - <csr-id-7a41dbe55da4336620a5b3ea0606f2144bff0c50/> Fix mips64r6el contents fetch ...
   ... also improve refresh logic
 - <csr-id-66d3fd158891d2c061a3133b39bd179077c10d72/> Fix local mirror host name get
 - <csr-id-f4b96b0e5e5f944e74528b857402bb8e5de36030/> Fix flat-repo fetch
 - <csr-id-6ff39b47d20f24e194187e1c0a35f3f4f615d410/> Adapt new oma-fetch api
 - <csr-id-f86961d4ad183a69974186c7a9a8fd59d4e63d84/> Do not always decompress contents every refresh
 - <csr-id-5732aeab8067c66265b1f0c9893fc216a2a1c0a3/> Do not fetch repeatedly source
 - <csr-id-2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5/> Clear decompress progress bar

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt

### Refactor

 - <csr-id-1943b764ee60248d6c02f820e50cdc1e5d73716b/> Adapt new oma-fetch api
 - <csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/> Re-abstract code
 - <csr-id-2768dc5e3070661cc797121575c25ba88819d8a9/> Remove useless code
 - <csr-id-1a3d60e8665faf452a217a478bf0b1c7ce3e445b/> Inner reqwest::Client
 - <csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/> Remove useless file; lint
 - <csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/> Use builder api design
 - <csr-id-5622e3699691081f0de4466379c14bc539e69c11/> Use async
 - <csr-id-20818083ca01c6209cd28d5279637d7e21422192/> Add some error translate
 - <csr-id-a3c910b6cf9ef432f2b93e38adb61fd6b021d819/> Fill of error translate todo
 - <csr-id-88efbe1e674c3a3030144ad3b0690d1e2095cdaf/> Done 1
 - <csr-id-1e637a4c0b535d095c8f35229a8ce910c3a163a6/> Done for decompress

### Style

 - <csr-id-0e32ceead5727a79c2841c5d137fd32a8cd88753/> Use cargo-fmt to format code
 - <csr-id-65fa216e325fe96f964a31c47d500e3197c9a269/> Lint code style
 - <csr-id-20ee30139b0da28db1d422d4605cbe3582a71e15/> Use cargo-fmt and cargo-clippy to lint code
 - <csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 61 commits contributed to the release over the course of 13 calendar days.
 - 45 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fill some dep version ([`b8b6868`](https://github.com/AOSC-Dev/oma/commit/b8b68685187bf1740c91372b9aa73bb777e3d134))
    - Add license and desc ([`86d65eb`](https://github.com/AOSC-Dev/oma/commit/86d65eb054576ec4e2fea52d3722beb7dc8c0c32))
    - Add changelog ([`54bc679`](https://github.com/AOSC-Dev/oma/commit/54bc679fe098faceea2ed461f5da6178b34330f0))
    - Release oma-console v0.1.2, oma-topics v0.1.1, oma-refresh v0.3.0 ([`5f4e6d8`](https://github.com/AOSC-Dev/oma/commit/5f4e6d8262f42724c8f796fc0b6c560a39d3fd5f))
    - Add topic closing message ([`9665cd4`](https://github.com/AOSC-Dev/oma/commit/9665cd4b3e50ca8fbe18c388bd3c75f6c4b81b2e))
    - Release oma-fetch v0.2.0, safety bump 2 crates ([`3d643f9`](https://github.com/AOSC-Dev/oma/commit/3d643f98588d93c60a094808b794624e78d464b7))
    - Adapt new oma-fetch api ([`1943b76`](https://github.com/AOSC-Dev/oma/commit/1943b764ee60248d6c02f820e50cdc1e5d73716b))
    - Bump oma-utils v0.1.5 ([`f671881`](https://github.com/AOSC-Dev/oma/commit/f67188176dfaa546bcfec4512c00509a60c86f98))
    - Fix mips64r6el contents fetch ... ([`7a41be`](https://github.com/AOSC-Dev/oma/commit/7a41dbe55da4336620a5b3ea0606f2144bff0c50))
    - Use cargo-fmt to format code ([`0e32cee`](https://github.com/AOSC-Dev/oma/commit/0e32ceead5727a79c2841c5d137fd32a8cd88753))
    - Lint code style ([`65fa216`](https://github.com/AOSC-Dev/oma/commit/65fa216e325fe96f964a31c47d500e3197c9a269))
    - Do not fetch database with the same content but in a different compression format ([`94687df`](https://github.com/AOSC-Dev/oma/commit/94687df792f92c1b717c81ff31b8e803aa5fb125))
    - Update nix to 0.27 ([`11fd26e`](https://github.com/AOSC-Dev/oma/commit/11fd26ec2732fe5be0137601fe3388a1f9aaf014))
    - Use cargo-fmt and cargo-clippy to lint code ([`20ee301`](https://github.com/AOSC-Dev/oma/commit/20ee30139b0da28db1d422d4605cbe3582a71e15))
    - Feat(oma-refresh: improve date parse error handle ([`ac889c4`](https://github.com/AOSC-Dev/oma/commit/ac889c4e4e5b0f71b5e5b439f68bc3bffcc5ebd4))
    - Switch to chrono ([`adffcfb`](https://github.com/AOSC-Dev/oma/commit/adffcfbc7c19c8e559ba45f991cb4af55f1c8224))
    - Adapt tokio enabled feature ([`5700316`](https://github.com/AOSC-Dev/oma/commit/57003169329e01d60172d3531e7f3817bacf46da))
    - Adjust some deps ([`922fb8a`](https://github.com/AOSC-Dev/oma/commit/922fb8aa093a6050c4fdc848f2e5fab369db6095))
    - Bump oma-console v0.1.1, oma-fetch v0.1.2, oma-utils v0.1.4, oma-pm v0.2.1 ([`64f5d1b`](https://github.com/AOSC-Dev/oma/commit/64f5d1bf4f93b7b3b1f5a00134e232409458e5e3))
    - Fix local mirror host name get ([`66d3fd1`](https://github.com/AOSC-Dev/oma/commit/66d3fd158891d2c061a3133b39bd179077c10d72))
    - Bump oma-utils v0.1.3 ([`206806f`](https://github.com/AOSC-Dev/oma/commit/206806f036ed7f127955c14499c742c7864848f9))
    - Bump oma-utils v0.1.2 ([`27954dc`](https://github.com/AOSC-Dev/oma/commit/27954dc8346d57431f4d4f4cbf695841027eb440))
    - Use feature to select abstract code ([`69a17fe`](https://github.com/AOSC-Dev/oma/commit/69a17fe9bbc77374992e617a62db681bb7a1bca6))
    - Refresh done no need to manual fsync ([`870fcae`](https://github.com/AOSC-Dev/oma/commit/870fcaeeafdf83a4e2e54d07f81a59e38c05ec9b))
    - Use oma-debcontrol crate (own fork) ([`0f2613c`](https://github.com/AOSC-Dev/oma/commit/0f2613cb0419e58d10a6bf453d4e4417b02f6e4a))
    - Fmt ([`9bb6e19`](https://github.com/AOSC-Dev/oma/commit/9bb6e19a703bc76515a7fa70c19aaafef38c7d7b))
    - Release oma-console v0.1.0 ([`aaf51bc`](https://github.com/AOSC-Dev/oma/commit/aaf51bc802b8e2c464c68c47efb6ffa6c0f075c2))
    - Re-abstract code ([`21864b9`](https://github.com/AOSC-Dev/oma/commit/21864b9135312ce096ccfed57dc240fffd28fda1))
    - Fix flat-repo fetch ([`f4b96b0`](https://github.com/AOSC-Dev/oma/commit/f4b96b0e5e5f944e74528b857402bb8e5de36030))
    - Add mark_version_status function ([`578b5e3`](https://github.com/AOSC-Dev/oma/commit/578b5e39890ec6a53b378c56201b0e179107f451))
    - Remove useless code ([`2768dc5`](https://github.com/AOSC-Dev/oma/commit/2768dc5e3070661cc797121575c25ba88819d8a9))
    - Inner reqwest::Client ([`1a3d60e`](https://github.com/AOSC-Dev/oma/commit/1a3d60e8665faf452a217a478bf0b1c7ce3e445b))
    - Inquire -> oma-inquire ([`57fcaa5`](https://github.com/AOSC-Dev/oma/commit/57fcaa531bc827a8661cf2a4f0f8a50c39289277))
    - Drop useless dep ([`0e14c25`](https://github.com/AOSC-Dev/oma/commit/0e14c25a9f5ad34da79df93cd3e686e81323f320))
    - Remove useless file; lint ([`336b02c`](https://github.com/AOSC-Dev/oma/commit/336b02cd7f1e950d028724c11d2318bed0495ddc))
    - Download compress file after extract ([`67c9c44`](https://github.com/AOSC-Dev/oma/commit/67c9c44809f1ae091913d851fc2e8b18163eb037))
    - Checksum pure database ([`df69c97`](https://github.com/AOSC-Dev/oma/commit/df69c9714ffb218ba8963d39ef63bd5cedecf015))
    - Use builder api design ([`b097de9`](https://github.com/AOSC-Dev/oma/commit/b097de9165dc0f1a8d970b750c84d6f5fc8ead81))
    - Use async ([`5622e36`](https://github.com/AOSC-Dev/oma/commit/5622e3699691081f0de4466379c14bc539e69c11))
    - Add some error translate ([`2081808`](https://github.com/AOSC-Dev/oma/commit/20818083ca01c6209cd28d5279637d7e21422192))
    - Fill of error translate todo ([`a3c910b`](https://github.com/AOSC-Dev/oma/commit/a3c910b6cf9ef432f2b93e38adb61fd6b021d819))
    - Cargo fmt ([`b0f6954`](https://github.com/AOSC-Dev/oma/commit/b0f69541f4d8baa5abb92d1db2e73fe6dc4c71f5))
    - No need to use tracing ([`0ca5be7`](https://github.com/AOSC-Dev/oma/commit/0ca5be73a7ddb70e3a07b63ef21f2f873e420832))
    - Adapt new oma-fetch api ([`6ff39b4`](https://github.com/AOSC-Dev/oma/commit/6ff39b47d20f24e194187e1c0a35f3f4f615d410))
    - Add translate ([`bf04133`](https://github.com/AOSC-Dev/oma/commit/bf04133b4335ac1de687634a393bf5f2685d9e5f))
    - After the database refresh is complete fsync ([`3ee53e6`](https://github.com/AOSC-Dev/oma/commit/3ee53e62af52f374b32cbbf86e60a591547ca17a))
    - Do not always decompress contents every refresh ([`f86961d`](https://github.com/AOSC-Dev/oma/commit/f86961d4ad183a69974186c7a9a8fd59d4e63d84))
    - Do not fetch repeatedly source ([`5732aea`](https://github.com/AOSC-Dev/oma/commit/5732aeab8067c66265b1f0c9893fc216a2a1c0a3))
    - Run cargo clippy and cargo fmt to lint code ([`9de51fa`](https://github.com/AOSC-Dev/oma/commit/9de51fa2cf2993c10acfd05d3cda133e6140ac44))
    - Fmt example ([`f875de4`](https://github.com/AOSC-Dev/oma/commit/f875de43cb615ab3f620e5e1c6989b3f07c651be))
    - Clear decompress progress bar ([`2f40bc8`](https://github.com/AOSC-Dev/oma/commit/2f40bc8d2709ffc8d1cfec391ef5eab6a42c1dd5))
    - Done 1 ([`88efbe1`](https://github.com/AOSC-Dev/oma/commit/88efbe1e674c3a3030144ad3b0690d1e2095cdaf))
    - Done for decompress ([`1e637a4`](https://github.com/AOSC-Dev/oma/commit/1e637a4c0b535d095c8f35229a8ce910c3a163a6))
    - 7 ([`211c9f0`](https://github.com/AOSC-Dev/oma/commit/211c9f036108f74e0c331e6228e54482350ebeb9))
    - 6 ([`9b195b0`](https://github.com/AOSC-Dev/oma/commit/9b195b04f2f7e224f096aa6c04aaba56c55b1698))
    - 5 ([`7c5a418`](https://github.com/AOSC-Dev/oma/commit/7c5a418058df9cc95d556323abaed84e34156116))
    - Some changes(4) ([`6450e2d`](https://github.com/AOSC-Dev/oma/commit/6450e2d2a7588d958be39cbecb375872422277f2))
    - Some changes(3) ([`80ea9eb`](https://github.com/AOSC-Dev/oma/commit/80ea9ebb86e4f88ea3a1d283f2686d1d38c2c73c))
    - Some changes(2) ([`12407e3`](https://github.com/AOSC-Dev/oma/commit/12407e3c9bf79f1b47a8b3a942f1979a771d72f1))
    - Some change ([`5d16784`](https://github.com/AOSC-Dev/oma/commit/5d16784215b2c47059c335e5f03c94ffaaf63693))
    - Init ([`5cedd38`](https://github.com/AOSC-Dev/oma/commit/5cedd38dc69b89403b8f13aa8b68a6360481991b))
</details>

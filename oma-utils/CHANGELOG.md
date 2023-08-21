# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.1 (2023-08-21)

### Chore

 - <csr-id-64280ae41d3df6a11e5806153a6cb0057f0875fe/> add changelog
 - <csr-id-882ef91ff21a1376be3daecfd54359e89f6c35be/> add desc and LICENSE (MIT) and comment

### New Features

 - <csr-id-d527b6b04616b9e46714338856b5e47fea9befd8/> add take walk lock and check battery feature
 - <csr-id-c0cd36b57e8169497e6744065078c3c245573ec6/> oma mark check root
 - <csr-id-bc470fdee31c413e32f5f9c1abb320297da1d987/> add mark_version_status function

### Other

 - <csr-id-42a30f3c99799b933d4ae663c543376d9644c634/> fmt

### Refactor

 - <csr-id-d900e4a30d02215f43d026a998b0a7bd95bbc099/> re-abstract code
 - <csr-id-0ed23241a26d9fa82deca4c49ee676b905950f74/> oma mark is back
 - <csr-id-201ff85c8c933370416f7bd8fd2100b86f10e40f/> can set pkgs as argument in mark_version_status function
 - <csr-id-9388436c646d65eb5527b6c6a1f3f39923aadeee/> install/remove/upgrade/refresh done
 - <csr-id-ecb46d44b356e994225e00c5cc16439198fd4ff3/> pkg.rs => oma-pm

### Style

 - <csr-id-bb833287d6d439c622e737148d609c1b848e5efa/> run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release over the course of 3 calendar days.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Fmt ([`42a30f3`](https://github.com/AOSC-Dev/oma/commit/42a30f3c99799b933d4ae663c543376d9644c634))
    - Release oma-utils v0.1.0 ([`d18eb0e`](https://github.com/AOSC-Dev/oma/commit/d18eb0efe81bdb7d5c7d2b3d64ab05c037f327df))
    - Add changelog ([`64280ae`](https://github.com/AOSC-Dev/oma/commit/64280ae41d3df6a11e5806153a6cb0057f0875fe))
    - Add desc and LICENSE (MIT) and comment ([`882ef91`](https://github.com/AOSC-Dev/oma/commit/882ef91ff21a1376be3daecfd54359e89f6c35be))
    - Re-abstract code ([`d900e4a`](https://github.com/AOSC-Dev/oma/commit/d900e4a30d02215f43d026a998b0a7bd95bbc099))
    - Add take walk lock and check battery feature ([`d527b6b`](https://github.com/AOSC-Dev/oma/commit/d527b6b04616b9e46714338856b5e47fea9befd8))
    - Oma mark check root ([`c0cd36b`](https://github.com/AOSC-Dev/oma/commit/c0cd36b57e8169497e6744065078c3c245573ec6))
    - Oma mark is back ([`0ed2324`](https://github.com/AOSC-Dev/oma/commit/0ed23241a26d9fa82deca4c49ee676b905950f74))
    - Can set pkgs as argument in mark_version_status function ([`201ff85`](https://github.com/AOSC-Dev/oma/commit/201ff85c8c933370416f7bd8fd2100b86f10e40f))
    - Add mark_version_status function ([`bc470fd`](https://github.com/AOSC-Dev/oma/commit/bc470fdee31c413e32f5f9c1abb320297da1d987))
    - Cargo fmt ([`75b6c86`](https://github.com/AOSC-Dev/oma/commit/75b6c866b398d90ee55655e29c436303673b8a52))
    - Install/remove/upgrade/refresh done ([`9388436`](https://github.com/AOSC-Dev/oma/commit/9388436c646d65eb5527b6c6a1f3f39923aadeee))
    - Run cargo clippy and cargo fmt to lint code ([`bb83328`](https://github.com/AOSC-Dev/oma/commit/bb833287d6d439c622e737148d609c1b848e5efa))
    - Pkg.rs => oma-pm ([`ecb46d4`](https://github.com/AOSC-Dev/oma/commit/ecb46d44b356e994225e00c5cc16439198fd4ff3))
    - 6 ([`4b4d394`](https://github.com/AOSC-Dev/oma/commit/4b4d394642e2df41382b608ab4784793727a79bd))
</details>

## v0.1.0 (2023-08-17)

<csr-id-0b0c1dbdf1faa21f01a54f889a65b984d74b4059/>
<csr-id-30a708a8419dd4d07d833a56466dffb7f290fda8/>
<csr-id-717bece8a874dede7a8ac58fc56f41daaf3daa48/>
<csr-id-2c4554b6a9988e55e0d1bf41b05e4e24b82899f7/>
<csr-id-c1e161f60650ed8feb562838ed9ecb5ecdadfe05/>
<csr-id-a4207f7a57e8561f1aa58e4af66057227b2c00e2/>
<csr-id-ee45498f402ccc6a686c44b1b4f887301e9801e1/>
<csr-id-0501e3ed5b24636e9c155a8781e7e7004cd8316c/>

### Chore

 - <csr-id-0b0c1dbdf1faa21f01a54f889a65b984d74b4059/> add desc and LICENSE (MIT) and comment

### Chore

 - <csr-id-0501e3ed5b24636e9c155a8781e7e7004cd8316c/> add changelog

### New Features

 - <csr-id-d6c45b2360f26a00bfaec6c60521d274f03ee729/> add take walk lock and check battery feature
 - <csr-id-bc5112669b5ed735b03040843b359647eb9063ed/> oma mark check root
 - <csr-id-13a65de5404dac6f0820733553792a86fd949511/> add mark_version_status function

### Refactor

 - <csr-id-30a708a8419dd4d07d833a56466dffb7f290fda8/> re-abstract code
 - <csr-id-717bece8a874dede7a8ac58fc56f41daaf3daa48/> oma mark is back
 - <csr-id-2c4554b6a9988e55e0d1bf41b05e4e24b82899f7/> can set pkgs as argument in mark_version_status function
 - <csr-id-c1e161f60650ed8feb562838ed9ecb5ecdadfe05/> install/remove/upgrade/refresh done
 - <csr-id-a4207f7a57e8561f1aa58e4af66057227b2c00e2/> pkg.rs => oma-pm

### Style

 - <csr-id-ee45498f402ccc6a686c44b1b4f887301e9801e1/> run cargo clippy and cargo fmt to lint code


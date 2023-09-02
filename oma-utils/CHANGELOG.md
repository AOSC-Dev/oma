# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.5 (2023-09-02)

### New Features

 - <csr-id-232b98246297a42b6294f2c39dc6d06b58ebbb32/> Do not ring if not is_terminal

### Refactor

 - <csr-id-25554c2835d2b2ce50815ce2aa3e8b3cd40071b3/> Move oma-pm url_no_escape function to oma-utils

### Style

 - <csr-id-177be7637ea9e6dce7a988d1b20553cb072ac33d/> Use cargo-fmt to format code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 4 calendar days.
 - 7 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Use cargo-fmt to format code ([`177be76`](https://github.com/AOSC-Dev/oma/commit/177be7637ea9e6dce7a988d1b20553cb072ac33d))
    - Move oma-pm url_no_escape function to oma-utils ([`25554c2`](https://github.com/AOSC-Dev/oma/commit/25554c2835d2b2ce50815ce2aa3e8b3cd40071b3))
    - Do not ring if not is_terminal ([`232b982`](https://github.com/AOSC-Dev/oma/commit/232b98246297a42b6294f2c39dc6d06b58ebbb32))
</details>

## v0.1.4 (2023-08-26)

<csr-id-08bafaf3f46c347f8f95ef2e0dbd420e7ee3e197/>
<csr-id-5f8881a5f16b1798323ec1bd558c1c8abb7b44d1/>
<csr-id-02d849fe98571c85ca78c5b6c1df71ef5077deb4/>
<csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/>
<csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/>
<csr-id-004cf53213308152b780115f50ec55589e08d3ae/>
<csr-id-87f2218bd28559b2483a515b892043d65df8f576/>
<csr-id-d921ccbec06258c1f30815b0685302376ecbd343/>
<csr-id-e0208cd2160358e8125577f990df090f02dc9528/>
<csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/>

### Chore

 - <csr-id-08bafaf3f46c347f8f95ef2e0dbd420e7ee3e197/> 0.1.3
 - <csr-id-5f8881a5f16b1798323ec1bd558c1c8abb7b44d1/> Add changelog
 - <csr-id-02d849fe98571c85ca78c5b6c1df71ef5077deb4/> Add desc and LICENSE (MIT) and comment

### New Features

 - <csr-id-69a17fe9bbc77374992e617a62db681bb7a1bca6/> Use feature to select abstract code
 - <csr-id-5afbe32511508a14055d780724c8bd71db2fcb18/> Add take walk lock and check battery feature
 - <csr-id-efe79f32b2923d4ebfa836349e7b5b041b953e77/> Oma mark check root
 - <csr-id-578b5e39890ec6a53b378c56201b0e179107f451/> Add mark_version_status function

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt

### Refactor

 - <csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/> Re-abstract code
 - <csr-id-004cf53213308152b780115f50ec55589e08d3ae/> Oma mark is back
 - <csr-id-87f2218bd28559b2483a515b892043d65df8f576/> Can set pkgs as argument in mark_version_status function
 - <csr-id-d921ccbec06258c1f30815b0685302376ecbd343/> Install/remove/upgrade/refresh done
 - <csr-id-e0208cd2160358e8125577f990df090f02dc9528/> Pkg.rs => oma-pm

### Style

 - <csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/> Run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release over the course of 4 calendar days.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma-console v0.1.1, oma-fetch v0.1.2, oma-utils v0.1.4, oma-pm v0.2.1 ([`64f5d1b`](https://github.com/AOSC-Dev/oma/commit/64f5d1bf4f93b7b3b1f5a00134e232409458e5e3))
    - Bump oma-utils v0.1.3 ([`206806f`](https://github.com/AOSC-Dev/oma/commit/206806f036ed7f127955c14499c742c7864848f9))
    - 0.1.3 ([`08bafaf`](https://github.com/AOSC-Dev/oma/commit/08bafaf3f46c347f8f95ef2e0dbd420e7ee3e197))
    - Bump oma-utils v0.1.2 ([`27954dc`](https://github.com/AOSC-Dev/oma/commit/27954dc8346d57431f4d4f4cbf695841027eb440))
    - Use feature to select abstract code ([`69a17fe`](https://github.com/AOSC-Dev/oma/commit/69a17fe9bbc77374992e617a62db681bb7a1bca6))
    - Bump oma-fetch v0.1.1, oma-utils v0.1.1, oma-pm v0.2.0 ([`51b4ab2`](https://github.com/AOSC-Dev/oma/commit/51b4ab259c5fe014493c78e04f5c6671f56d95e8))
    - Fmt ([`9bb6e19`](https://github.com/AOSC-Dev/oma/commit/9bb6e19a703bc76515a7fa70c19aaafef38c7d7b))
    - Release oma-utils v0.1.0 ([`743a1a2`](https://github.com/AOSC-Dev/oma/commit/743a1a26cb12a97ad7d4eeb63b21c1df6d4f4afd))
    - Add changelog ([`5f8881a`](https://github.com/AOSC-Dev/oma/commit/5f8881a5f16b1798323ec1bd558c1c8abb7b44d1))
    - Add desc and LICENSE (MIT) and comment ([`02d849f`](https://github.com/AOSC-Dev/oma/commit/02d849fe98571c85ca78c5b6c1df71ef5077deb4))
    - Re-abstract code ([`21864b9`](https://github.com/AOSC-Dev/oma/commit/21864b9135312ce096ccfed57dc240fffd28fda1))
    - Add take walk lock and check battery feature ([`5afbe32`](https://github.com/AOSC-Dev/oma/commit/5afbe32511508a14055d780724c8bd71db2fcb18))
    - Oma mark check root ([`efe79f3`](https://github.com/AOSC-Dev/oma/commit/efe79f32b2923d4ebfa836349e7b5b041b953e77))
    - Oma mark is back ([`004cf53`](https://github.com/AOSC-Dev/oma/commit/004cf53213308152b780115f50ec55589e08d3ae))
    - Can set pkgs as argument in mark_version_status function ([`87f2218`](https://github.com/AOSC-Dev/oma/commit/87f2218bd28559b2483a515b892043d65df8f576))
    - Add mark_version_status function ([`578b5e3`](https://github.com/AOSC-Dev/oma/commit/578b5e39890ec6a53b378c56201b0e179107f451))
    - Cargo fmt ([`b0f6954`](https://github.com/AOSC-Dev/oma/commit/b0f69541f4d8baa5abb92d1db2e73fe6dc4c71f5))
    - Install/remove/upgrade/refresh done ([`d921ccb`](https://github.com/AOSC-Dev/oma/commit/d921ccbec06258c1f30815b0685302376ecbd343))
    - Run cargo clippy and cargo fmt to lint code ([`9de51fa`](https://github.com/AOSC-Dev/oma/commit/9de51fa2cf2993c10acfd05d3cda133e6140ac44))
    - Pkg.rs => oma-pm ([`e0208cd`](https://github.com/AOSC-Dev/oma/commit/e0208cd2160358e8125577f990df090f02dc9528))
    - 6 ([`9b195b0`](https://github.com/AOSC-Dev/oma/commit/9b195b04f2f7e224f096aa6c04aaba56c55b1698))
</details>

## v0.1.3 (2023-08-22)

<csr-id-3af47b057182b1311d96e1fe6825ad32bbd0e23b/>

### Chore

 - <csr-id-3af47b057182b1311d96e1fe6825ad32bbd0e23b/> 0.1.3

## v0.1.2 (2023-08-22)

### New Features

 - <csr-id-ec61dda03a3ad18f3b9b34db398b39c550e0abbf/> Use feature to select abstract code

## v0.1.1 (2023-08-21)

<csr-id-64280ae41d3df6a11e5806153a6cb0057f0875fe/>
<csr-id-882ef91ff21a1376be3daecfd54359e89f6c35be/>
<csr-id-42a30f3c99799b933d4ae663c543376d9644c634/>
<csr-id-d900e4a30d02215f43d026a998b0a7bd95bbc099/>
<csr-id-0ed23241a26d9fa82deca4c49ee676b905950f74/>
<csr-id-201ff85c8c933370416f7bd8fd2100b86f10e40f/>
<csr-id-9388436c646d65eb5527b6c6a1f3f39923aadeee/>
<csr-id-ecb46d44b356e994225e00c5cc16439198fd4ff3/>
<csr-id-bb833287d6d439c622e737148d609c1b848e5efa/>

### Chore

 - <csr-id-64280ae41d3df6a11e5806153a6cb0057f0875fe/> add changelog
 - <csr-id-882ef91ff21a1376be3daecfd54359e89f6c35be/> add desc and LICENSE (MIT) and comment

### New Features

 - <csr-id-d527b6b04616b9e46714338856b5e47fea9befd8/> Add take walk lock and check battery feature
 - <csr-id-c0cd36b57e8169497e6744065078c3c245573ec6/> Oma mark check root
 - <csr-id-bc470fdee31c413e32f5f9c1abb320297da1d987/> Add mark_version_status function

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

 - <csr-id-d6c45b2360f26a00bfaec6c60521d274f03ee729/> Add take walk lock and check battery feature
 - <csr-id-bc5112669b5ed735b03040843b359647eb9063ed/> Oma mark check root
 - <csr-id-13a65de5404dac6f0820733553792a86fd949511/> Add mark_version_status function

### Refactor

 - <csr-id-30a708a8419dd4d07d833a56466dffb7f290fda8/> re-abstract code
 - <csr-id-717bece8a874dede7a8ac58fc56f41daaf3daa48/> oma mark is back
 - <csr-id-2c4554b6a9988e55e0d1bf41b05e4e24b82899f7/> can set pkgs as argument in mark_version_status function
 - <csr-id-c1e161f60650ed8feb562838ed9ecb5ecdadfe05/> install/remove/upgrade/refresh done
 - <csr-id-a4207f7a57e8561f1aa58e4af66057227b2c00e2/> pkg.rs => oma-pm

### Style

 - <csr-id-ee45498f402ccc6a686c44b1b4f887301e9801e1/> run cargo clippy and cargo fmt to lint code


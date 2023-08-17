# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2023-08-17)

<csr-id-3c7c29e6e5da4cc1b4e10006aa9cac2b2008d43a/>
<csr-id-eb52b648a8b51af5bdf1cd39dd3045c49267f399/>
<csr-id-119cc9f79cb3e0a2c1e5623614915c6e7c0b8769/>
<csr-id-9e6f244eaf4e52c13107c2dc6b42432982b5eb37/>
<csr-id-999ff58a1a4d6d5ceecb8563018a21b0002c90ae/>
<csr-id-61b0cf19043ce4ee0a50fa2ee1584248a03d30bf/>
<csr-id-b210c488cd00656131cf77ef7f98a5aef0999e73/>
<csr-id-3f0e43a4b964113b261b5688b612c71f6c87b7b1/>
<csr-id-5170f79611bb4b36baa7a179167de5cd3b141a2e/>
<csr-id-255c03b6d49e612578cd75f7e8c92aba273a2308/>
<csr-id-7dba4beee8cf26d469b01dbbe5b61b06d73622b2/>
<csr-id-ee45498f402ccc6a686c44b1b4f887301e9801e1/>

### Chore

 - <csr-id-3c7c29e6e5da4cc1b4e10006aa9cac2b2008d43a/> fill of comment
 - <csr-id-eb52b648a8b51af5bdf1cd39dd3045c49267f399/> no need to use tracing

### Chore

 - <csr-id-ec5d5d35534f5200143f6d819ca5d2ed989fd21c/> add desc
 - <csr-id-294cd1b853c63d48ab1fcb33db95ea3838ab47dd/> add changelog

### New Features

 - <csr-id-e8f56b5f1634556fd269d2b598d37f12eb1dfab7/> display done message if env is not atty
 - <csr-id-0a5a509cdd4d46e1848bbfae989f3dc752bf7e80/> use DEBUG globar var to store yes/no display debug message
 - <csr-id-e92a420653a852ebd2d26d2cbf91dd2f7cded154/> fill of remove() function
 - <csr-id-b50cfc0a5337053c496876de84eaf00f221884ed/> init

### Bug Fixes

 - <csr-id-2037757c6ebde5a94f85f4b1802674ac3c10d05f/> fix logger marco multi use

### Refactor

 - <csr-id-119cc9f79cb3e0a2c1e5623614915c6e7c0b8769/> remove useless file; lint
 - <csr-id-9e6f244eaf4e52c13107c2dc6b42432982b5eb37/> fill of error translate (50%)
 - <csr-id-999ff58a1a4d6d5ceecb8563018a21b0002c90ae/> improve debug marco
 - <csr-id-61b0cf19043ce4ee0a50fa2ee1584248a03d30bf/> add progressbar style
   - Also add oma-fetcher ProgressStyle
 - <csr-id-b210c488cd00656131cf77ef7f98a5aef0999e73/> add todo
 - <csr-id-3f0e43a4b964113b261b5688b612c71f6c87b7b1/> do not const Writer::default as WRITER
 - <csr-id-5170f79611bb4b36baa7a179167de5cd3b141a2e/> add oma-topics crate
 - <csr-id-255c03b6d49e612578cd75f7e8c92aba273a2308/> abstract tips and has_x11
 - <csr-id-7dba4beee8cf26d469b01dbbe5b61b06d73622b2/> add crate oma-console

### Style

 - <csr-id-ee45498f402ccc6a686c44b1b4f887301e9801e1/> run cargo clippy and cargo fmt to lint code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 10 calendar days.
 - 19 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add desc ([`ec5d5d3`](https://github.com/AOSC-Dev/oma/commit/ec5d5d35534f5200143f6d819ca5d2ed989fd21c))
    - Add changelog ([`294cd1b`](https://github.com/AOSC-Dev/oma/commit/294cd1b853c63d48ab1fcb33db95ea3838ab47dd))
    - Fill of comment ([`3c7c29e`](https://github.com/AOSC-Dev/oma/commit/3c7c29e6e5da4cc1b4e10006aa9cac2b2008d43a))
    - Display done message if env is not atty ([`e8f56b5`](https://github.com/AOSC-Dev/oma/commit/e8f56b5f1634556fd269d2b598d37f12eb1dfab7))
    - Remove useless file; lint ([`119cc9f`](https://github.com/AOSC-Dev/oma/commit/119cc9f79cb3e0a2c1e5623614915c6e7c0b8769))
    - Fill of error translate (50%) ([`9e6f244`](https://github.com/AOSC-Dev/oma/commit/9e6f244eaf4e52c13107c2dc6b42432982b5eb37))
    - No need to use tracing ([`eb52b64`](https://github.com/AOSC-Dev/oma/commit/eb52b648a8b51af5bdf1cd39dd3045c49267f399))
    - Use DEBUG globar var to store yes/no display debug message ([`0a5a509`](https://github.com/AOSC-Dev/oma/commit/0a5a509cdd4d46e1848bbfae989f3dc752bf7e80))
    - Fix logger marco multi use ([`2037757`](https://github.com/AOSC-Dev/oma/commit/2037757c6ebde5a94f85f4b1802674ac3c10d05f))
    - Improve debug marco ([`999ff58`](https://github.com/AOSC-Dev/oma/commit/999ff58a1a4d6d5ceecb8563018a21b0002c90ae))
    - Fill of remove() function ([`e92a420`](https://github.com/AOSC-Dev/oma/commit/e92a420653a852ebd2d26d2cbf91dd2f7cded154))
    - Run cargo clippy and cargo fmt to lint code ([`ee45498`](https://github.com/AOSC-Dev/oma/commit/ee45498f402ccc6a686c44b1b4f887301e9801e1))
    - Init ([`b50cfc0`](https://github.com/AOSC-Dev/oma/commit/b50cfc0a5337053c496876de84eaf00f221884ed))
    - Add progressbar style ([`61b0cf1`](https://github.com/AOSC-Dev/oma/commit/61b0cf19043ce4ee0a50fa2ee1584248a03d30bf))
    - Add todo ([`b210c48`](https://github.com/AOSC-Dev/oma/commit/b210c488cd00656131cf77ef7f98a5aef0999e73))
    - Do not const Writer::default as WRITER ([`3f0e43a`](https://github.com/AOSC-Dev/oma/commit/3f0e43a4b964113b261b5688b612c71f6c87b7b1))
    - Add oma-topics crate ([`5170f79`](https://github.com/AOSC-Dev/oma/commit/5170f79611bb4b36baa7a179167de5cd3b141a2e))
    - Abstract tips and has_x11 ([`255c03b`](https://github.com/AOSC-Dev/oma/commit/255c03b6d49e612578cd75f7e8c92aba273a2308))
    - Add crate oma-console ([`7dba4be`](https://github.com/AOSC-Dev/oma/commit/7dba4beee8cf26d469b01dbbe5b61b06d73622b2))
</details>


# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.4.1 (2023-09-06)

### Chore

 - <csr-id-fd0c5d0ca12294b58c7251dc69d4b9a80f3cee0b/> update indicium to 0.5.1
 - <csr-id-57003169329e01d60172d3531e7f3817bacf46da/> adapt tokio enabled feature
 - <csr-id-4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac/> adjust some deps (again)

### New Features

 - <csr-id-a0750502605cabb6d7385f1cbc96edf639324cb5/> add DownloadEvent::AllDone to allow control global progress bar finish and clear
 - <csr-id-eefd6a8ed8d528ff308ef76a1a2c7effd2a95561/> add no_progress option to control dpkg use-pty value
 - <csr-id-13018326745688027422575eb5a364a050c4c691/> add --no-progress option to no output progress bar to terminal
 - <csr-id-232b98246297a42b6294f2c39dc6d06b58ebbb32/> do not ring if not is_terminal
 - <csr-id-dea6e5e59573a480933d42bffd9a90cc13c19734/> if not is_terminal disable dpkg read database indicatif

### Bug Fixes

 - <csr-id-10ae79a09065b1c3f3c360662dddd4f71b5ed858/> do not download empty list
 - <csr-id-2144cd9d35cdcb817e7934f0fc5786c60427cd15/> fix real_pkg function if pkg has version and provides
 - <csr-id-b7ce7810f766d73fa18f7ecf8a051f58dbcf2027/> fix user remove package also display autoremove tag
 - <csr-id-f55963e9514bce0baf52c8295efa0725aa649fb0/> fix oma history ui string
 - <csr-id-e732e8ceea33787407f0124593f8d75a083f0572/> fix oma fix-broken with different results from apt install -f
 - <csr-id-d3195ea58fe14c24f8d82dd444a0d54689498285/> mark reinstall protect package mark
 - <csr-id-1167d0357c78071259390f0794f6f4a539f330a2/> fix oma install downgrade package mark install logic
 - <csr-id-ad3a2b78652acad3c0f1e9a7ffc78fbce9884da2/> allow multi value set at the same time
 - <csr-id-9def10a52588fbe6ccafa2a51b0abd6d6ebdf358/> mark_install use branch to compare source
 - <csr-id-9803474892095380871b76ec0c0512f520351803/> try to fix version source check
   - Also improve oma list tips output order
 - <csr-id-5c571ef867974dbc7ccc0fce464d434e8ae85d5b/> mark_delete after resolve deps to fix autoremove packages

### Refactor

 - <csr-id-8f2cb7c6f2bf4e118d0b5fe17105a4a2fd6164f5/> adapt oma-fetch new API
 - <csr-id-25554c2835d2b2ce50815ce2aa3e8b3cd40071b3/> move oma-pm url_no_escape function to oma-utils
 - <csr-id-79648fa18f3adf5e60bab608635093fb877771d9/> use version.is_downloadable to check package version is downloadable
 - <csr-id-9b3f923deb1da81abc0360577a0d743c34ff311d/> no need to clone some var in search function

### Style

 - <csr-id-1875106a3ac133a463bb1c251ba11b5b8b1429d6/> use cargo-fmt to format code
 - <csr-id-0b60dcd970bde752af1a81a04285b4e9577582fd/> improve code style
 - <csr-id-73ceb521eefe915b72e197783f9102dd3d78a3b6/> improve code style

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 32 commits contributed to the release over the course of 10 calendar days.
 - 10 days passed between releases.
 - 26 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release oma-fetch v0.3.1 ([`1e65ad3`](https://github.com/AOSC-Dev/oma/commit/1e65ad3641b396cb5c6e8675b431d4b176f9e314))
    - Do not download empty list ([`10ae79a`](https://github.com/AOSC-Dev/oma/commit/10ae79a09065b1c3f3c360662dddd4f71b5ed858))
    - Release oma-fetch v0.3.0, safety bump 2 crates ([`0959dfb`](https://github.com/AOSC-Dev/oma/commit/0959dfb5414f46c96d7b7aa39c485bdc1d3862de))
    - Add DownloadEvent::AllDone to allow control global progress bar finish and clear ([`a075050`](https://github.com/AOSC-Dev/oma/commit/a0750502605cabb6d7385f1cbc96edf639324cb5))
    - Add no_progress option to control dpkg use-pty value ([`eefd6a8`](https://github.com/AOSC-Dev/oma/commit/eefd6a8ed8d528ff308ef76a1a2c7effd2a95561))
    - Add --no-progress option to no output progress bar to terminal ([`1301832`](https://github.com/AOSC-Dev/oma/commit/13018326745688027422575eb5a364a050c4c691))
    - Release oma-console v0.1.2, oma-topics v0.1.1, oma-refresh v0.3.0 ([`5f4e6d8`](https://github.com/AOSC-Dev/oma/commit/5f4e6d8262f42724c8f796fc0b6c560a39d3fd5f))
    - Release oma-fetch v0.2.0, safety bump 2 crates ([`3d643f9`](https://github.com/AOSC-Dev/oma/commit/3d643f98588d93c60a094808b794624e78d464b7))
    - Use cargo-fmt to format code ([`1875106`](https://github.com/AOSC-Dev/oma/commit/1875106a3ac133a463bb1c251ba11b5b8b1429d6))
    - Adapt oma-fetch new API ([`8f2cb7c`](https://github.com/AOSC-Dev/oma/commit/8f2cb7c6f2bf4e118d0b5fe17105a4a2fd6164f5))
    - Update indicium to 0.5.1 ([`fd0c5d0`](https://github.com/AOSC-Dev/oma/commit/fd0c5d0ca12294b58c7251dc69d4b9a80f3cee0b))
    - Bump oma-fetch v0.1.3 ([`808db0b`](https://github.com/AOSC-Dev/oma/commit/808db0bef0e9b4c001d1c2e1a291bd2d7a4a1871))
    - Bump oma-utils v0.1.5 ([`f671881`](https://github.com/AOSC-Dev/oma/commit/f67188176dfaa546bcfec4512c00509a60c86f98))
    - Move oma-pm url_no_escape function to oma-utils ([`25554c2`](https://github.com/AOSC-Dev/oma/commit/25554c2835d2b2ce50815ce2aa3e8b3cd40071b3))
    - Fix real_pkg function if pkg has version and provides ([`2144cd9`](https://github.com/AOSC-Dev/oma/commit/2144cd9d35cdcb817e7934f0fc5786c60427cd15))
    - Fix user remove package also display autoremove tag ([`b7ce781`](https://github.com/AOSC-Dev/oma/commit/b7ce7810f766d73fa18f7ecf8a051f58dbcf2027))
    - Fix oma history ui string ([`f55963e`](https://github.com/AOSC-Dev/oma/commit/f55963e9514bce0baf52c8295efa0725aa649fb0))
    - Improve code style ([`0b60dcd`](https://github.com/AOSC-Dev/oma/commit/0b60dcd970bde752af1a81a04285b4e9577582fd))
    - Fix oma fix-broken with different results from apt install -f ([`e732e8c`](https://github.com/AOSC-Dev/oma/commit/e732e8ceea33787407f0124593f8d75a083f0572))
    - Improve code style ([`73ceb52`](https://github.com/AOSC-Dev/oma/commit/73ceb521eefe915b72e197783f9102dd3d78a3b6))
    - Mark reinstall protect package mark ([`d3195ea`](https://github.com/AOSC-Dev/oma/commit/d3195ea58fe14c24f8d82dd444a0d54689498285))
    - Fix oma install downgrade package mark install logic ([`1167d03`](https://github.com/AOSC-Dev/oma/commit/1167d0357c78071259390f0794f6f4a539f330a2))
    - Do not ring if not is_terminal ([`232b982`](https://github.com/AOSC-Dev/oma/commit/232b98246297a42b6294f2c39dc6d06b58ebbb32))
    - If not is_terminal disable dpkg read database indicatif ([`dea6e5e`](https://github.com/AOSC-Dev/oma/commit/dea6e5e59573a480933d42bffd9a90cc13c19734))
    - Allow multi value set at the same time ([`ad3a2b7`](https://github.com/AOSC-Dev/oma/commit/ad3a2b78652acad3c0f1e9a7ffc78fbce9884da2))
    - Use version.is_downloadable to check package version is downloadable ([`79648fa`](https://github.com/AOSC-Dev/oma/commit/79648fa18f3adf5e60bab608635093fb877771d9))
    - Mark_install use branch to compare source ([`9def10a`](https://github.com/AOSC-Dev/oma/commit/9def10a52588fbe6ccafa2a51b0abd6d6ebdf358))
    - Try to fix version source check ([`9803474`](https://github.com/AOSC-Dev/oma/commit/9803474892095380871b76ec0c0512f520351803))
    - Mark_delete after resolve deps to fix autoremove packages ([`5c571ef`](https://github.com/AOSC-Dev/oma/commit/5c571ef867974dbc7ccc0fce464d434e8ae85d5b))
    - No need to clone some var in search function ([`9b3f923`](https://github.com/AOSC-Dev/oma/commit/9b3f923deb1da81abc0360577a0d743c34ff311d))
    - Adapt tokio enabled feature ([`5700316`](https://github.com/AOSC-Dev/oma/commit/57003169329e01d60172d3531e7f3817bacf46da))
    - Adjust some deps (again) ([`4d25b67`](https://github.com/AOSC-Dev/oma/commit/4d25b67028aab447a042bf0d6cbe4fcd9a1a4eac))
</details>

## v0.2.1 (2023-08-26)

<csr-id-0da16ac5e46ec5152b99383e87b8473443e286ba/>
<csr-id-d97162225f4f6bd989ea877663462ecda0f2260d/>
<csr-id-b8cbd746f11741ba2d6a0b2fc08fc096d7a294f8/>
<csr-id-1735e90d53db25c588e79143f4fc98a742a1c99b/>
<csr-id-0e0b8eb6912b3b6eaf340f7974b12e1f0b4893b6/>
<csr-id-24c4d3b3335b1df0378b2706f74b2c10427d757e/>
<csr-id-aac287434f3c9590d4c056ae69e4637f51605fff/>
<csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/>
<csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/>
<csr-id-f315784d7e1081e906148c5d07c1abe40c033755/>
<csr-id-f7cf4a1db505958cccf77b4e0703336318dc8b2d/>
<csr-id-5aa02c250f7344f2e97300a0e072fe94d181cda9/>
<csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/>
<csr-id-cfb7d0509673b85ea770987f2660f323dc3621e2/>
<csr-id-004cf53213308152b780115f50ec55589e08d3ae/>
<csr-id-8fe6a3147820fbee5a99f3ac16a45b5148be4e7a/>
<csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/>
<csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/>
<csr-id-24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b/>
<csr-id-f14236ef10dc77d1b912c4f29d7e56d6181e1bee/>
<csr-id-07c07e77179c318eb3fcce1327238037c212c025/>
<csr-id-2b37d6c7bc57bbfbcaa8175bb6b23b796d7814f4/>
<csr-id-baf2b96e4a423cfe5e981b486c6975d36b577249/>
<csr-id-7560c558cbfc68ccb488bac29aa15477e74d9607/>
<csr-id-5be647ac17030fa8e5aaf3d2366dec454b9f4803/>
<csr-id-2f367e93d8bf5568b057d356335900059dd6ff08/>
<csr-id-4f9a0750c1917a6ea74857a22e60d4564e1b8184/>
<csr-id-744de142f4401e57eaf3e52ec57e467a3ae4157f/>
<csr-id-a5fddd1f052a61442a2e712521cfb6705f4492f9/>
<csr-id-916b1c26ea32039845263e8f39f3e08a4f7719d7/>
<csr-id-e2ca9f8ffb9c4e9d80d1017e2fe72aa6b9e22b70/>
<csr-id-f6eaf24e87ef942889624b277703381f528d3656/>
<csr-id-7b87edd6554a60316f645c538cab61bfcd1bea9c/>
<csr-id-2bca031a16a01b3474a3136dfaf305f6f6b5e3f7/>
<csr-id-03c395718e2b2b73a55767381ad5442c55f367fa/>
<csr-id-3f3c7ed693312735efd9714ee3a76a84029fb647/>
<csr-id-b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef/>
<csr-id-7839bd1f49759e5a937e587ab231b9fe384cd119/>
<csr-id-5210a35f685fe23ad53af7d0cf1ef5c9f3d3f9a2/>
<csr-id-2edbfb3b213d75e9920c314ea8ef831fc9126124/>
<csr-id-24a69b81d1eb4c3dd1efc62f305e97831537679e/>
<csr-id-4b1e6bf5e29755b80426d84f21cb4daed34c16a2/>
<csr-id-8dc2091c1e7da9237647ebaa61d83d87888b86d6/>
<csr-id-97e8bffac933a9c450f38e1626a25888e9a19a51/>
<csr-id-4e28e415ebaceb64cd85145f3a940516d50730e6/>
<csr-id-6f199d842ce0fe9c8493556e2988c6895cacd5c1/>
<csr-id-1871c05c869a7d759dbcee788173d2b4217a4481/>
<csr-id-e0208cd2160358e8125577f990df090f02dc9528/>
<csr-id-3c48d031ce68621ca49d3a47b9424f7269038b17/>
<csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/>
<csr-id-b0ff13ba140eee1634bca70b244f4f58cd63d9d4/>
<csr-id-37ba4023d6a93ddaacb1dab4a53e636bd66a81cf/>
<csr-id-8c5ee5fc38c441708aa338bf27a193708e583c39/>
<csr-id-047f7c728924d7b34ecb71e333713eb92da42573/>
<csr-id-2e6b1b2fee9912585bce067497ec5d597d6abfca/>
<csr-id-90d7308c28f0b89839e18969788ef89464b7c810/>
<csr-id-e8e83b6dfcfd669a62bc54f2a941a9932bbffcf5/>
<csr-id-1e01e6c459701d05052e5ed178792e647a46bb94/>
<csr-id-afd08b3d813a774bc2fbfc28af7b07142de23788/>

### Chore

 - <csr-id-0da16ac5e46ec5152b99383e87b8473443e286ba/> Update all deps and cargo clippy
 - <csr-id-d97162225f4f6bd989ea877663462ecda0f2260d/> Fix license
 - <csr-id-b8cbd746f11741ba2d6a0b2fc08fc096d7a294f8/> Add changelog
 - <csr-id-1735e90d53db25c588e79143f4fc98a742a1c99b/> Fill in comment
 - <csr-id-0e0b8eb6912b3b6eaf340f7974b12e1f0b4893b6/> Add desc and license
 - <csr-id-24c4d3b3335b1df0378b2706f74b2c10427d757e/> Switch to oma-apt (own rust-apt fork)
 - <csr-id-aac287434f3c9590d4c056ae69e4637f51605fff/> Remove useless dep
 - <csr-id-0ca5be73a7ddb70e3a07b63ef21f2f873e420832/> No need to use tracing

### New Features

<csr-id-69a17fe9bbc77374992e617a62db681bb7a1bca6/>
<csr-id-f81beb312f6fb7fb371f676ffefc647117f8de56/>
<csr-id-4677a5841c842b5cfee83c2b76e341d55209b219/>
<csr-id-a760fd334d1790dff050be56c895f2a76dee18b6/>
<csr-id-d4b01d927d5dc96c7047a51fd03097a3aa71caed/>
<csr-id-578b5e39890ec6a53b378c56201b0e179107f451/>
<csr-id-0dc6354c69cc885fadcc10d719179ac27dcebcad/>
<csr-id-1d9b5d15dc76e74672be7b0d610202ad1dc11fdb/>
<csr-id-53363a77097badf966faa33c927d2d8b8d0edd16/>
<csr-id-1a43ad7b9ebf2111c632b71e735e2289e4d39e12/>
<csr-id-530935f6d61e8a91681adcaa85a16614585fccd6/>
<csr-id-f0907c73484046dcc36b17f3040b6e86e1bd96bc/>
<csr-id-8cc1f724f1d7ed67355513323d8a1c1bf68d04d2/>
<csr-id-b99bf3ded4326eb148e19ef83d90497900b15580/>
<csr-id-ff1362eb4615acd0220afb5f5677249e411a5217/>
<csr-id-438b8fffe4c465b22981937bc0979546f797a62a/>

 - <csr-id-128c85cf11311743f76944d05cc481db268a11d4/> Use chrono to get and parse datetime
 - <csr-id-56529cf371b0239e81ad7a2d75d65cf75841b595/> Add oma history/undo date display
   - Also fix time offset get
- Also display progress from stderr not stdout

### Bug Fixes

 - <csr-id-5d78cfc42fed60d7c76671385e68d6c99f1d67f2/> Oma show if package not downloable display unknown
 - <csr-id-21976779fd3ed9359cf6f008cf8c92c60389637d/> Fix history text log Commandline display
 - <csr-id-5cf03f65bcd71735c80ae80cadc5ca8d2838617b/> Add oma-utils feature to fix compile
 - <csr-id-14da974da743488cc32a168e0e603730ffe15ea9/> Fix get selected pkgs logic
 - <csr-id-8ce739746cc9320263d33523b8d7508ca8958ec3/> Handle could not find any package for keyword
 - <csr-id-9078b9cf47f95a61445b317806e4c872503ec832/> Fix oma topics cancal select topic downgrade pkg
 - <csr-id-7d6db8af7ac642a2ec66e137e7ab482829ef8089/> Add loop count check in url_no_escape function
 - <csr-id-9e0f46a8a9909dcada3850b3005274f7311ce5d6/> Check mark pkg is installed
 - <csr-id-e6bfc1ba83cba016ede7e64e32b6e61d3154fc4e/> Try to fix unmet dep wrong error output
 - <csr-id-135d01e01badb15a2f1c4178213175a1f75b25be/> Try to fix some break item wrong result
 - <csr-id-885bc3f26553bf9540611dd442b672276b5494a6/> Oma upgrade add find breaks logic
 - <csr-id-98b501052ddccfb3aaf009ee22b98b8e727b9a44/> Use version.arch() replaced pkg.arch() to get package arch
 - <csr-id-482c3ab9c766d7b93ed83d38bd4aec92cde663de/> Fix u64 overflow to oma remove pkg to failed
 - <csr-id-448108a13054ba37c23a2e08774f1480e234868b/> --{no-,}install-{recommend,suggest}
 - <csr-id-4d0860c01cd043bae912074ffa3f19a022a1ab6f/> Oma show APT-Spirces display
 - <csr-id-cae33b5250e41fef43710d04a797e413e288cbd5/> Oma show multi package query
 - <csr-id-6f4d7854176377bdc2234ae58a5569549bd8482d/> Fix a typo
 - <csr-id-aa0763bec84cd98439e15516af6f9ca2f796d618/> Fix local package install
 - <csr-id-fb8f449648a1e2f939a77e238f37ec3f40e2b38d/> Fix ask is essential after will remove package

### Other

 - <csr-id-9bb6e19a703bc76515a7fa70c19aaafef38c7d7b/> Fmt
 - <csr-id-f315784d7e1081e906148c5d07c1abe40c033755/> Abstract resolve() function

### Refactor

 - <csr-id-f7cf4a1db505958cccf77b4e0703336318dc8b2d/> Oma read oma.toml config feature is back
 - <csr-id-5aa02c250f7344f2e97300a0e072fe94d181cda9/> Log history database is back
 - <csr-id-21864b9135312ce096ccfed57dc240fffd28fda1/> Re-abstract code
 - <csr-id-cfb7d0509673b85ea770987f2660f323dc3621e2/> Write history feature is back
 - <csr-id-004cf53213308152b780115f50ec55589e08d3ae/> Oma mark is back
 - <csr-id-8fe6a3147820fbee5a99f3ac16a45b5148be4e7a/> Improve oma topics downgrade logic
 - <csr-id-336b02cd7f1e950d028724c11d2318bed0495ddc/> Remove useless file; lint
 - <csr-id-b097de9165dc0f1a8d970b750c84d6f5fc8ead81/> Use builder api design
 - <csr-id-24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b/> Fill of error translate (50%)
 - <csr-id-f14236ef10dc77d1b912c4f29d7e56d6181e1bee/> Already-installed message is back
 - <csr-id-07c07e77179c318eb3fcce1327238037c212c025/> Some display info is back
 - <csr-id-2b37d6c7bc57bbfbcaa8175bb6b23b796d7814f4/> Dry-run mode is back
 - <csr-id-baf2b96e4a423cfe5e981b486c6975d36b577249/> Check disk size is back
 - <csr-id-7560c558cbfc68ccb488bac29aa15477e74d9607/> Do some todo
 - <csr-id-5be647ac17030fa8e5aaf3d2366dec454b9f4803/> Unmet dep ui is back
 - <csr-id-2f367e93d8bf5568b057d356335900059dd6ff08/> Move fix_broken function to command.rs
 - <csr-id-4f9a0750c1917a6ea74857a22e60d4564e1b8184/> Remove useless code
 - <csr-id-744de142f4401e57eaf3e52ec57e467a3ae4157f/> Move logic to command.rs
 - <csr-id-a5fddd1f052a61442a2e712521cfb6705f4492f9/> Oma pkgnames is back
 - <csr-id-916b1c26ea32039845263e8f39f3e08a4f7719d7/> Oma clean is back
 - <csr-id-e2ca9f8ffb9c4e9d80d1017e2fe72aa6b9e22b70/> Oma list is back
 - <csr-id-f6eaf24e87ef942889624b277703381f528d3656/> Oma pick is back
 - <csr-id-7b87edd6554a60316f645c538cab61bfcd1bea9c/> Oma fix-broken is back
 - <csr-id-2bca031a16a01b3474a3136dfaf305f6f6b5e3f7/> Oma search is back
 - <csr-id-03c395718e2b2b73a55767381ad5442c55f367fa/> Oma show is back!
 - <csr-id-3f3c7ed693312735efd9714ee3a76a84029fb647/> Refresh info is back
 - <csr-id-b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef/> Improve debug marco
 - <csr-id-7839bd1f49759e5a937e587ab231b9fe384cd119/> Oma remove after autoremove feature is back
 - <csr-id-5210a35f685fe23ad53af7d0cf1ef5c9f3d3f9a2/> Add remove/upgrade pending ui
 - <csr-id-2edbfb3b213d75e9920c314ea8ef831fc9126124/> Pending ui is back!
 - <csr-id-24a69b81d1eb4c3dd1efc62f305e97831537679e/> More args back
 - <csr-id-4b1e6bf5e29755b80426d84f21cb4daed34c16a2/> --install-dbg flag is back
 - <csr-id-8dc2091c1e7da9237647ebaa61d83d87888b86d6/> Improve api design
 - <csr-id-97e8bffac933a9c450f38e1626a25888e9a19a51/> Done OmaApt::commit function
 - <csr-id-4e28e415ebaceb64cd85145f3a940516d50730e6/> Done for operation_map
 - <csr-id-6f199d842ce0fe9c8493556e2988c6895cacd5c1/> Improve lifetime logic
 - <csr-id-1871c05c869a7d759dbcee788173d2b4217a4481/> Api adjust
 - <csr-id-e0208cd2160358e8125577f990df090f02dc9528/> Pkg.rs => oma-pm

### Style

 - <csr-id-3c48d031ce68621ca49d3a47b9424f7269038b17/> Run cargo clippy and cargo fmt to lint code
 - <csr-id-9de51fa2cf2993c10acfd05d3cda133e6140ac44/> Run cargo clippy and cargo fmt to lint code
 - <csr-id-b0ff13ba140eee1634bca70b244f4f58cd63d9d4/> Remove useless line

### Test

 - <csr-id-37ba4023d6a93ddaacb1dab4a53e636bd66a81cf/> Fix example
 - <csr-id-8c5ee5fc38c441708aa338bf27a193708e583c39/> Update example
 - <csr-id-047f7c728924d7b34ecb71e333713eb92da42573/> Example texlive -> vscodium to save your sweet time
 - <csr-id-2e6b1b2fee9912585bce067497ec5d597d6abfca/> Add download pkgs example
 - <csr-id-90d7308c28f0b89839e18969788ef89464b7c810/> Refactor again
 - <csr-id-e8e83b6dfcfd669a62bc54f2a941a9932bbffcf5/> Refactor
 - <csr-id-1e01e6c459701d05052e5ed178792e647a46bb94/> Add example
 - <csr-id-afd08b3d813a774bc2fbfc28af7b07142de23788/> Add test_branch_search

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 110 commits contributed to the release over the course of 4 calendar days.
 - 96 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Bump oma-console v0.1.1, oma-fetch v0.1.2, oma-utils v0.1.4, oma-pm v0.2.1 ([`64f5d1b`](https://github.com/AOSC-Dev/oma/commit/64f5d1bf4f93b7b3b1f5a00134e232409458e5e3))
    - Oma read oma.toml config feature is back ([`f7cf4a1`](https://github.com/AOSC-Dev/oma/commit/f7cf4a1db505958cccf77b4e0703336318dc8b2d))
    - Update all deps and cargo clippy ([`0da16ac`](https://github.com/AOSC-Dev/oma/commit/0da16ac5e46ec5152b99383e87b8473443e286ba))
    - Use chrono to get and parse datetime ([`128c85c`](https://github.com/AOSC-Dev/oma/commit/128c85cf11311743f76944d05cc481db268a11d4))
    - Fix example ([`37ba402`](https://github.com/AOSC-Dev/oma/commit/37ba4023d6a93ddaacb1dab4a53e636bd66a81cf))
    - Add oma history/undo date display ([`56529cf`](https://github.com/AOSC-Dev/oma/commit/56529cf371b0239e81ad7a2d75d65cf75841b595))
    - Oma show if package not downloable display unknown ([`5d78cfc`](https://github.com/AOSC-Dev/oma/commit/5d78cfc42fed60d7c76671385e68d6c99f1d67f2))
    - Fix history text log Commandline display ([`2197677`](https://github.com/AOSC-Dev/oma/commit/21976779fd3ed9359cf6f008cf8c92c60389637d))
    - Add oma-utils feature to fix compile ([`5cf03f6`](https://github.com/AOSC-Dev/oma/commit/5cf03f65bcd71735c80ae80cadc5ca8d2838617b))
    - Fix get selected pkgs logic ([`14da974`](https://github.com/AOSC-Dev/oma/commit/14da974da743488cc32a168e0e603730ffe15ea9))
    - Bump oma-utils v0.1.3 ([`206806f`](https://github.com/AOSC-Dev/oma/commit/206806f036ed7f127955c14499c742c7864848f9))
    - Bump oma-utils v0.1.2 ([`27954dc`](https://github.com/AOSC-Dev/oma/commit/27954dc8346d57431f4d4f4cbf695841027eb440))
    - Use feature to select abstract code ([`69a17fe`](https://github.com/AOSC-Dev/oma/commit/69a17fe9bbc77374992e617a62db681bb7a1bca6))
    - Run cargo clippy and cargo fmt to lint code ([`3c48d03`](https://github.com/AOSC-Dev/oma/commit/3c48d031ce68621ca49d3a47b9424f7269038b17))
    - Log history database is back ([`5aa02c2`](https://github.com/AOSC-Dev/oma/commit/5aa02c250f7344f2e97300a0e072fe94d181cda9))
    - Bump oma-fetch v0.1.1, oma-utils v0.1.1, oma-pm v0.2.0 ([`51b4ab2`](https://github.com/AOSC-Dev/oma/commit/51b4ab259c5fe014493c78e04f5c6671f56d95e8))
    - Handle could not find any package for keyword ([`8ce7397`](https://github.com/AOSC-Dev/oma/commit/8ce739746cc9320263d33523b8d7508ca8958ec3))
    - Fmt ([`9bb6e19`](https://github.com/AOSC-Dev/oma/commit/9bb6e19a703bc76515a7fa70c19aaafef38c7d7b))
    - Release oma-pm v0.1.0 ([`bb54624`](https://github.com/AOSC-Dev/oma/commit/bb5462441819fde9ac208bcb6d4448894b614de5))
    - Fix license ([`d971622`](https://github.com/AOSC-Dev/oma/commit/d97162225f4f6bd989ea877663462ecda0f2260d))
    - Release oma-pm v0.1.0 ([`fa1990c`](https://github.com/AOSC-Dev/oma/commit/fa1990c27dad37d58168674c3feab7553125a9ad))
    - Add changelog ([`b8cbd74`](https://github.com/AOSC-Dev/oma/commit/b8cbd746f11741ba2d6a0b2fc08fc096d7a294f8))
    - Fill in comment ([`1735e90`](https://github.com/AOSC-Dev/oma/commit/1735e90d53db25c588e79143f4fc98a742a1c99b))
    - Add desc and license ([`0e0b8eb`](https://github.com/AOSC-Dev/oma/commit/0e0b8eb6912b3b6eaf340f7974b12e1f0b4893b6))
    - Switch to oma-apt (own rust-apt fork) ([`24c4d3b`](https://github.com/AOSC-Dev/oma/commit/24c4d3b3335b1df0378b2706f74b2c10427d757e))
    - Fix oma topics cancal select topic downgrade pkg ([`9078b9c`](https://github.com/AOSC-Dev/oma/commit/9078b9cf47f95a61445b317806e4c872503ec832))
    - Re-abstract code ([`21864b9`](https://github.com/AOSC-Dev/oma/commit/21864b9135312ce096ccfed57dc240fffd28fda1))
    - Table display remove size delta message ([`f81beb3`](https://github.com/AOSC-Dev/oma/commit/f81beb312f6fb7fb371f676ffefc647117f8de56))
    - Do not display apt progress if not is terminal ([`4677a58`](https://github.com/AOSC-Dev/oma/commit/4677a5841c842b5cfee83c2b76e341d55209b219))
    - Add Size-delta field on oma history; improve file output ([`a760fd3`](https://github.com/AOSC-Dev/oma/commit/a760fd334d1790dff050be56c895f2a76dee18b6))
    - Write history feature is back ([`cfb7d05`](https://github.com/AOSC-Dev/oma/commit/cfb7d0509673b85ea770987f2660f323dc3621e2))
    - Add loop count check in url_no_escape function ([`7d6db8a`](https://github.com/AOSC-Dev/oma/commit/7d6db8af7ac642a2ec66e137e7ab482829ef8089))
    - Check mark pkg is installed ([`9e0f46a`](https://github.com/AOSC-Dev/oma/commit/9e0f46a8a9909dcada3850b3005274f7311ce5d6))
    - Oma mark is back ([`004cf53`](https://github.com/AOSC-Dev/oma/commit/004cf53213308152b780115f50ec55589e08d3ae))
    - Add mark_install_status function ([`d4b01d9`](https://github.com/AOSC-Dev/oma/commit/d4b01d927d5dc96c7047a51fd03097a3aa71caed))
    - Add mark_version_status function ([`578b5e3`](https://github.com/AOSC-Dev/oma/commit/578b5e39890ec6a53b378c56201b0e179107f451))
    - Remove useless dep ([`aac2874`](https://github.com/AOSC-Dev/oma/commit/aac287434f3c9590d4c056ae69e4637f51605fff))
    - Try to fix unmet dep wrong error output ([`e6bfc1b`](https://github.com/AOSC-Dev/oma/commit/e6bfc1ba83cba016ede7e64e32b6e61d3154fc4e))
    - Revert "fix: try to fix some break item wrong result" ([`33027fb`](https://github.com/AOSC-Dev/oma/commit/33027fb464415384fab6cde1a057cff56a80f7d1))
    - Try to fix some break item wrong result ([`135d01e`](https://github.com/AOSC-Dev/oma/commit/135d01e01badb15a2f1c4178213175a1f75b25be))
    - Find unmet dep only display layer 1 dep ([`0dc6354`](https://github.com/AOSC-Dev/oma/commit/0dc6354c69cc885fadcc10d719179ac27dcebcad))
    - Improve oma topics downgrade logic ([`8fe6a31`](https://github.com/AOSC-Dev/oma/commit/8fe6a3147820fbee5a99f3ac16a45b5148be4e7a))
    - Remove useless file; lint ([`336b02c`](https://github.com/AOSC-Dev/oma/commit/336b02cd7f1e950d028724c11d2318bed0495ddc))
    - Oma upgrade add find breaks logic ([`885bc3f`](https://github.com/AOSC-Dev/oma/commit/885bc3f26553bf9540611dd442b672276b5494a6))
    - Use builder api design ([`b097de9`](https://github.com/AOSC-Dev/oma/commit/b097de9165dc0f1a8d970b750c84d6f5fc8ead81))
    - Use version.arch() replaced pkg.arch() to get package arch ([`98b5010`](https://github.com/AOSC-Dev/oma/commit/98b501052ddccfb3aaf009ee22b98b8e727b9a44))
    - Fill of error translate (50%) ([`24ca3e6`](https://github.com/AOSC-Dev/oma/commit/24ca3e6751a08cf5fcbbe0aa9c84d0ae4fc7de6b))
    - Already-installed message is back ([`f14236e`](https://github.com/AOSC-Dev/oma/commit/f14236ef10dc77d1b912c4f29d7e56d6181e1bee))
    - Some display info is back ([`07c07e7`](https://github.com/AOSC-Dev/oma/commit/07c07e77179c318eb3fcce1327238037c212c025))
    - Dry-run mode is back ([`2b37d6c`](https://github.com/AOSC-Dev/oma/commit/2b37d6c7bc57bbfbcaa8175bb6b23b796d7814f4))
    - Fix u64 overflow to oma remove pkg to failed ([`482c3ab`](https://github.com/AOSC-Dev/oma/commit/482c3ab9c766d7b93ed83d38bd4aec92cde663de))
    - Check disk size is back ([`baf2b96`](https://github.com/AOSC-Dev/oma/commit/baf2b96e4a423cfe5e981b486c6975d36b577249))
    - Do some todo ([`7560c55`](https://github.com/AOSC-Dev/oma/commit/7560c558cbfc68ccb488bac29aa15477e74d9607))
    - Unmet dep ui is back ([`5be647a`](https://github.com/AOSC-Dev/oma/commit/5be647ac17030fa8e5aaf3d2366dec454b9f4803))
    - Abstract resolve() function ([`f315784`](https://github.com/AOSC-Dev/oma/commit/f315784d7e1081e906148c5d07c1abe40c033755))
    - Move fix_broken function to command.rs ([`2f367e9`](https://github.com/AOSC-Dev/oma/commit/2f367e93d8bf5568b057d356335900059dd6ff08))
    - Update example ([`8c5ee5f`](https://github.com/AOSC-Dev/oma/commit/8c5ee5fc38c441708aa338bf27a193708e583c39))
    - Remove useless code ([`4f9a075`](https://github.com/AOSC-Dev/oma/commit/4f9a0750c1917a6ea74857a22e60d4564e1b8184))
    - --{no-,}install-{recommend,suggest} ([`448108a`](https://github.com/AOSC-Dev/oma/commit/448108a13054ba37c23a2e08774f1480e234868b))
    - Move logic to command.rs ([`744de14`](https://github.com/AOSC-Dev/oma/commit/744de142f4401e57eaf3e52ec57e467a3ae4157f))
    - Oma show APT-Spirces display ([`4d0860c`](https://github.com/AOSC-Dev/oma/commit/4d0860c01cd043bae912074ffa3f19a022a1ab6f))
    - Cargo fmt ([`b0f6954`](https://github.com/AOSC-Dev/oma/commit/b0f69541f4d8baa5abb92d1db2e73fe6dc4c71f5))
    - Fix cargo clippy ([`6757986`](https://github.com/AOSC-Dev/oma/commit/6757986e906cafe053bffd13dd6768931beb87ea))
    - Oma pkgnames is back ([`a5fddd1`](https://github.com/AOSC-Dev/oma/commit/a5fddd1f052a61442a2e712521cfb6705f4492f9))
    - Oma clean is back ([`916b1c2`](https://github.com/AOSC-Dev/oma/commit/916b1c26ea32039845263e8f39f3e08a4f7719d7))
    - Oma show multi package query ([`cae33b5`](https://github.com/AOSC-Dev/oma/commit/cae33b5250e41fef43710d04a797e413e288cbd5))
    - Oma list is back ([`e2ca9f8`](https://github.com/AOSC-Dev/oma/commit/e2ca9f8ffb9c4e9d80d1017e2fe72aa6b9e22b70))
    - Oma pick is back ([`f6eaf24`](https://github.com/AOSC-Dev/oma/commit/f6eaf24e87ef942889624b277703381f528d3656))
    - Oma fix-broken is back ([`7b87edd`](https://github.com/AOSC-Dev/oma/commit/7b87edd6554a60316f645c538cab61bfcd1bea9c))
    - Oma search is back ([`2bca031`](https://github.com/AOSC-Dev/oma/commit/2bca031a16a01b3474a3136dfaf305f6f6b5e3f7))
    - Oma show is back! ([`03c3957`](https://github.com/AOSC-Dev/oma/commit/03c395718e2b2b73a55767381ad5442c55f367fa))
    - Refresh info is back ([`3f3c7ed`](https://github.com/AOSC-Dev/oma/commit/3f3c7ed693312735efd9714ee3a76a84029fb647))
    - No need to use tracing ([`0ca5be7`](https://github.com/AOSC-Dev/oma/commit/0ca5be73a7ddb70e3a07b63ef21f2f873e420832))
    - Improve debug marco ([`b8fc1a9`](https://github.com/AOSC-Dev/oma/commit/b8fc1a95ccb112e3f0be406f3ab7c6b70fcfefef))
    - Oma remove after autoremove feature is back ([`7839bd1`](https://github.com/AOSC-Dev/oma/commit/7839bd1f49759e5a937e587ab231b9fe384cd119))
    - Add remove/upgrade pending ui ([`5210a35`](https://github.com/AOSC-Dev/oma/commit/5210a35f685fe23ad53af7d0cf1ef5c9f3d3f9a2))
    - Pending ui is back! ([`2edbfb3`](https://github.com/AOSC-Dev/oma/commit/2edbfb3b213d75e9920c314ea8ef831fc9126124))
    - Fix a typo ([`6f4d785`](https://github.com/AOSC-Dev/oma/commit/6f4d7854176377bdc2234ae58a5569549bd8482d))
    - More args back ([`24a69b8`](https://github.com/AOSC-Dev/oma/commit/24a69b81d1eb4c3dd1efc62f305e97831537679e))
    - --install-dbg flag is back ([`4b1e6bf`](https://github.com/AOSC-Dev/oma/commit/4b1e6bf5e29755b80426d84f21cb4daed34c16a2))
    - Fix local package install ([`aa0763b`](https://github.com/AOSC-Dev/oma/commit/aa0763bec84cd98439e15516af6f9ca2f796d618))
    - Improve api design ([`8dc2091`](https://github.com/AOSC-Dev/oma/commit/8dc2091c1e7da9237647ebaa61d83d87888b86d6))
    - Example texlive -> vscodium to save your sweet time ([`047f7c7`](https://github.com/AOSC-Dev/oma/commit/047f7c728924d7b34ecb71e333713eb92da42573))
    - Add download pkgs example ([`2e6b1b2`](https://github.com/AOSC-Dev/oma/commit/2e6b1b2fee9912585bce067497ec5d597d6abfca))
    - Fix ask is essential after will remove package ([`fb8f449`](https://github.com/AOSC-Dev/oma/commit/fb8f449648a1e2f939a77e238f37ec3f40e2b38d))
    - Fmt ([`198a835`](https://github.com/AOSC-Dev/oma/commit/198a835fc7f7cfdcd17ecb74b5a7c0e2faaf63b6))
    - Refactor again ([`90d7308`](https://github.com/AOSC-Dev/oma/commit/90d7308c28f0b89839e18969788ef89464b7c810))
    - Refactor ([`e8e83b6`](https://github.com/AOSC-Dev/oma/commit/e8e83b6dfcfd669a62bc54f2a941a9932bbffcf5))
    - Add example ([`1e01e6c`](https://github.com/AOSC-Dev/oma/commit/1e01e6c459701d05052e5ed178792e647a46bb94))
    - Fill of remove() function ([`1d9b5d1`](https://github.com/AOSC-Dev/oma/commit/1d9b5d15dc76e74672be7b0d610202ad1dc11fdb))
    - Done OmaApt::commit function ([`97e8bff`](https://github.com/AOSC-Dev/oma/commit/97e8bffac933a9c450f38e1626a25888e9a19a51))
    - Sleep ([`c4e817b`](https://github.com/AOSC-Dev/oma/commit/c4e817ba57eddfcda01eb5e6bff606fee2a46815))
    - 111 ([`082f233`](https://github.com/AOSC-Dev/oma/commit/082f233e33bc481bb164ef4e972a6bda8966da41))
    - Done for operation_map ([`4e28e41`](https://github.com/AOSC-Dev/oma/commit/4e28e415ebaceb64cd85145f3a940516d50730e6))
    - 111 ([`29bba29`](https://github.com/AOSC-Dev/oma/commit/29bba29170c5b39b6e8b569f6629b98774fca20d))
    - Add operation.rs ....zzz ([`53363a7`](https://github.com/AOSC-Dev/oma/commit/53363a77097badf966faa33c927d2d8b8d0edd16))
    - Some detail for oma-pm ([`d00927f`](https://github.com/AOSC-Dev/oma/commit/d00927feb013445075eeb5da51377fba651958b3))
    - Remove pkg add protect bool ([`1a43ad7`](https://github.com/AOSC-Dev/oma/commit/1a43ad7b9ebf2111c632b71e735e2289e4d39e12))
    - Add remove package feature ([`530935f`](https://github.com/AOSC-Dev/oma/commit/530935f6d61e8a91681adcaa85a16614585fccd6))
    - Support local package install ([`f0907c7`](https://github.com/AOSC-Dev/oma/commit/f0907c73484046dcc36b17f3040b6e86e1bd96bc))
    - Improve lifetime logic ([`6f199d8`](https://github.com/AOSC-Dev/oma/commit/6f199d842ce0fe9c8493556e2988c6895cacd5c1))
    - Add OmaApt struct ([`8cc1f72`](https://github.com/AOSC-Dev/oma/commit/8cc1f724f1d7ed67355513323d8a1c1bf68d04d2))
    - Add test_branch_search ([`afd08b3`](https://github.com/AOSC-Dev/oma/commit/afd08b3d813a774bc2fbfc28af7b07142de23788))
    - Add virtual pkg support and query_from_branch function ([`b99bf3d`](https://github.com/AOSC-Dev/oma/commit/b99bf3ded4326eb148e19ef83d90497900b15580))
    - Add query_from_version and query_from_branch function ([`ff1362e`](https://github.com/AOSC-Dev/oma/commit/ff1362eb4615acd0220afb5f5677249e411a5217))
    - Run cargo clippy and cargo fmt to lint code ([`9de51fa`](https://github.com/AOSC-Dev/oma/commit/9de51fa2cf2993c10acfd05d3cda133e6140ac44))
    - Add OmaDatabase impl ([`438b8ff`](https://github.com/AOSC-Dev/oma/commit/438b8fffe4c465b22981937bc0979546f797a62a))
    - Remove useless line ([`b0ff13b`](https://github.com/AOSC-Dev/oma/commit/b0ff13ba140eee1634bca70b244f4f58cd63d9d4))
    - Api adjust ([`1871c05`](https://github.com/AOSC-Dev/oma/commit/1871c05c869a7d759dbcee788173d2b4217a4481))
    - Pkg.rs => oma-pm ([`e0208cd`](https://github.com/AOSC-Dev/oma/commit/e0208cd2160358e8125577f990df090f02dc9528))
</details>

<csr-unknown>
 Use feature to select abstract code Table display remove size delta message Do not display apt progress if not is terminal Add Size-delta field on oma history; improve file output Add mark_install_status function Add mark_version_status function Find unmet dep only display layer 1 dep Fill of remove() function Add operation.rs ….zzz Remove pkg add protect bool Add remove package feature Support local package install Add OmaApt struct Add virtual pkg support and query_from_branch function Add query_from_version and query_from_branch function Add OmaDatabase impl<csr-unknown/>

## v0.2.0 (2023-08-21)

<csr-id-42a30f3c99799b933d4ae663c543376d9644c634/>

### Bug Fixes

 - <csr-id-2794b4dee123ec62e657defb107545fac2cd5aa2/> Handle could not find any package for keyword

### Other

 - <csr-id-42a30f3c99799b933d4ae663c543376d9644c634/> fmt

## v0.1.0 (2023-08-18)

<csr-id-50d0af03dad3776a09223050d6cd0ca9acbff0c1/>
<csr-id-5399edd1cfe450be52651b06ae110d06a3d20215/>
<csr-id-87ff82dbeb3199b5f87fe922d276549983ef15d9/>
<csr-id-ae87eb333e10872f028e53818092487ed09b4e84/>
<csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/>
<csr-id-6bc2e8f217e31da36b817f5f9f29bf29bdd2edb3/>
<csr-id-d900e4a30d02215f43d026a998b0a7bd95bbc099/>
<csr-id-43c553925e95a2548b558c6faebf778fcb03fed7/>
<csr-id-0ed23241a26d9fa82deca4c49ee676b905950f74/>
<csr-id-2c31d1e49c03e3a21b2339f157dfc767f719f322/>
<csr-id-bbe38a4fafc8c87a602f78175ae02d3edb60c794/>
<csr-id-a6e9e31fd80bdce5faea0162d3b7b47379dff987/>
<csr-id-718d2ebf3b11fe3e7859d55f0e6b08346a8e6b5f/>
<csr-id-27d03f139c434c43c5f59ed96ff9d5a0999b124c/>
<csr-id-d480d850f660c5accf8543257e45d7c029663a6d/>
<csr-id-3fb54e8ccf5c03219f81ab1c13305f800ca3761f/>
<csr-id-edb249d522e90a361780baa6b09be16411ccf507/>
<csr-id-31d6abe71e498a660b191542b120b44d98d34d2c/>
<csr-id-f5e6bce2c5bbe2775bdd91f7011ca512bb276228/>
<csr-id-ee1ec26815826041317293fe66aefeac3539d131/>
<csr-id-4616a67473fb4a5fa4f460965f699085f57664fa/>
<csr-id-f80e01822c04d7fd675aa2c939fd2c7af52b8fba/>
<csr-id-b724c5f72420aa1b95dd8c2624e82900671f3366/>
<csr-id-e06decc7a3e223ba86d523c0652f8ebe7d6f6cc2/>
<csr-id-aa8dc406e6ddb3f87712a984d4d6fda4a54b7114/>
<csr-id-fc29499f5a18fc9428dbe0d6e3f0bbeabf919dda/>
<csr-id-cac9dddc394c1a9ea51f25f6105fd3eb56aedabe/>
<csr-id-fb167bb7482db2be24d024c65e1a24b09ff7dbc9/>
<csr-id-63bf31f041f821ba1aa52f3b8675e216e7aab3fa/>
<csr-id-1e6ab4bdda4d3eb67c176128818ba649c3febc9c/>
<csr-id-6f65b3656809f431f3da938e7a9eac10b9922d60/>
<csr-id-cb2d04418d2a49b968b186db34437ef42afafd4d/>
<csr-id-ba04d6538388103765e1b1b1923cb13c7e02a912/>
<csr-id-1bcb6014b2e700be7688dd9ce51fdf33c14f58bc/>
<csr-id-ffdf00479d3bfa28584c1631cff02cc244c40095/>
<csr-id-b985f03b86a9e9c6727e5747ff3c05ce81861647/>
<csr-id-8637f8cb127faf50f7499b72e991a3e235dace7a/>
<csr-id-155bf1f3341ec15bf2955b86b42fc86dffa01822/>
<csr-id-bf1b4c19486425a857502ebeaa3f0d5be9723504/>
<csr-id-2bc80ea4e116a625b4acdcf4a9066b424de2e43a/>
<csr-id-90af4a45c3414783e97067a8790ff85e9fa9a1d0/>
<csr-id-ecb46d44b356e994225e00c5cc16439198fd4ff3/>
<csr-id-bb833287d6d439c622e737148d609c1b848e5efa/>
<csr-id-9eeae30d50d3ed3a1c06364bbdb83b6faea47211/>
<csr-id-682ff6893c55727477807993a4cc23a0d34278f7/>
<csr-id-f1296616be46f7fff77aaa7989a6e4028b04b0ba/>
<csr-id-c73e9886486d395cfc9eb119557226c85399406e/>
<csr-id-e6abdebe84c5155c029218685fdafa54086abcb2/>
<csr-id-583e126cd32a3ea19f11084cdcebdc50395f2975/>
<csr-id-3dbc72701d26037b0e569bf3ebeb01f911965313/>
<csr-id-7475d11b51a6488cb77bae231b6a6bee95f603b0/>
<csr-id-63e0c04ee482843bd57c519d386dabfccb889999/>
<csr-id-8ddd6809bbb1037a8c4d860b64adc52d6e9d2d3a/>

### Chore

 - <csr-id-50d0af03dad3776a09223050d6cd0ca9acbff0c1/> Fill in comment
 - <csr-id-5399edd1cfe450be52651b06ae110d06a3d20215/> Add desc and license
 - <csr-id-87ff82dbeb3199b5f87fe922d276549983ef15d9/> Switch to oma-apt (own rust-apt fork)
 - <csr-id-ae87eb333e10872f028e53818092487ed09b4e84/> Remove useless dep
 - <csr-id-fa15124038b9eaf8234766b33a98297c62d5b001/> No need to use tracing

### Chore

 - <csr-id-8ddd6809bbb1037a8c4d860b64adc52d6e9d2d3a/> Fix license

### Chore

 - <csr-id-63e0c04ee482843bd57c519d386dabfccb889999/> Add changelog

### New Features

<csr-id-640d8f6f73d57065970b15a699f45c3647c0ffe9/>
<csr-id-9ba4778e383690b502a4c37b90c2648474c7199d/>
<csr-id-bc470fdee31c413e32f5f9c1abb320297da1d987/>
<csr-id-7e9c1f412d58bd2532b6ca15fbd3d18d699835c9/>
<csr-id-549285c0f005f1961af50e5fc33b63812bb642fb/>
<csr-id-0c8a5d60aacb9b1e1e5c190cfb070d5406e763a9/>
<csr-id-e950cd3b26c2b2b1122ee15069263050bf2889a4/>
<csr-id-63226bb84a938bf352d8652008d57321251395b3/>
<csr-id-5833cb73127a35da5e392a9a900a2b59ce0b43b6/>
<csr-id-f8f3ee90b755e725a595ec223859054311b987a5/>
<csr-id-74e4b4b4e9c94874db2c5d105931cff36ae2f0d4/>
<csr-id-d335d585b79cf93ff32c5eb3946594e98a0a9e51/>
<csr-id-95529678e8ba180957e56bc8d73085becac022a7/>

 - <csr-id-8f49f32850a3f10a17e08f854f83206cebeecec7/> Table display remove size delta message
 - <csr-id-1869ed19076bec3c843f682a9cabcd8781e707f4/> Do not display apt progress if not is terminal
   - Also display progress from stderr not stdout

### Bug Fixes

 - <csr-id-6cc52b2a44a5eef23d8001740fd790670f960a60/> Fix oma topics cancal select topic downgrade pkg
 - <csr-id-5fb3c11afc8ba162cc6fe43da4e702d9c39aa6db/> Add loop count check in url_no_escape function
 - <csr-id-667c065acc9089717a68006e72ecc6cf84de8f5b/> Check mark pkg is installed
 - <csr-id-aa63f9c40967363f1f8d33df648613145aa19f1b/> Try to fix unmet dep wrong error output
 - <csr-id-00ee472c6fe9d9e992ef5976071c137cdf6f2a12/> Try to fix some break item wrong result
 - <csr-id-a256aefd819f776d67f6fe232edbc2025bb80b3f/> Oma upgrade add find breaks logic
 - <csr-id-bb003638edf8ab4c9189f184e130de40e601fae9/> Use version.arch() replaced pkg.arch() to get package arch
 - <csr-id-181c9db4270dd9d919b521c63afec8870f6916b0/> Fix u64 overflow to oma remove pkg to failed
 - <csr-id-694553d3c939ad7bf498311d17933f41ff0040f5/> --{no-,}install-{recommend,suggest}
 - <csr-id-88520045877dc90dd1ef1a046f3bd779f2c089f7/> Oma show APT-Spirces display
 - <csr-id-5ef70b3049b329f58fd970a554af0c78d854d773/> Oma show multi package query
 - <csr-id-44c28c00a7fd8e2662859922e340e32cd9fdcedd/> Fix a typo
 - <csr-id-2d8837b887d079c5011800f60250b3c72cfb63c4/> Fix local package install
 - <csr-id-be08bb3e3bf998ff088d913d4db986090feba396/> Fix ask is essential after will remove package

### Other

 - <csr-id-6bc2e8f217e31da36b817f5f9f29bf29bdd2edb3/> Abstract resolve() function

### Refactor

 - <csr-id-d900e4a30d02215f43d026a998b0a7bd95bbc099/> Re-abstract code
 - <csr-id-43c553925e95a2548b558c6faebf778fcb03fed7/> Write history feature is back
 - <csr-id-0ed23241a26d9fa82deca4c49ee676b905950f74/> Oma mark is back
 - <csr-id-2c31d1e49c03e3a21b2339f157dfc767f719f322/> Improve oma topics downgrade logic
 - <csr-id-bbe38a4fafc8c87a602f78175ae02d3edb60c794/> Remove useless file; lint
 - <csr-id-a6e9e31fd80bdce5faea0162d3b7b47379dff987/> Use builder api design
 - <csr-id-718d2ebf3b11fe3e7859d55f0e6b08346a8e6b5f/> Fill of error translate (50%)
 - <csr-id-27d03f139c434c43c5f59ed96ff9d5a0999b124c/> Already-installed message is back
 - <csr-id-d480d850f660c5accf8543257e45d7c029663a6d/> Some display info is back
 - <csr-id-3fb54e8ccf5c03219f81ab1c13305f800ca3761f/> Dry-run mode is back
 - <csr-id-edb249d522e90a361780baa6b09be16411ccf507/> Check disk size is back
 - <csr-id-31d6abe71e498a660b191542b120b44d98d34d2c/> Do some todo
 - <csr-id-f5e6bce2c5bbe2775bdd91f7011ca512bb276228/> Unmet dep ui is back
 - <csr-id-ee1ec26815826041317293fe66aefeac3539d131/> Move fix_broken function to command.rs
 - <csr-id-4616a67473fb4a5fa4f460965f699085f57664fa/> Remove useless code
 - <csr-id-f80e01822c04d7fd675aa2c939fd2c7af52b8fba/> Move logic to command.rs
 - <csr-id-b724c5f72420aa1b95dd8c2624e82900671f3366/> Oma pkgnames is back
 - <csr-id-e06decc7a3e223ba86d523c0652f8ebe7d6f6cc2/> Oma clean is back
 - <csr-id-aa8dc406e6ddb3f87712a984d4d6fda4a54b7114/> Oma list is back
 - <csr-id-fc29499f5a18fc9428dbe0d6e3f0bbeabf919dda/> Oma pick is back
 - <csr-id-cac9dddc394c1a9ea51f25f6105fd3eb56aedabe/> Oma fix-broken is back
 - <csr-id-fb167bb7482db2be24d024c65e1a24b09ff7dbc9/> Oma search is back
 - <csr-id-63bf31f041f821ba1aa52f3b8675e216e7aab3fa/> Oma show is back!
 - <csr-id-1e6ab4bdda4d3eb67c176128818ba649c3febc9c/> Refresh info is back
 - <csr-id-6f65b3656809f431f3da938e7a9eac10b9922d60/> Improve debug marco
 - <csr-id-cb2d04418d2a49b968b186db34437ef42afafd4d/> Oma remove after autoremove feature is back
 - <csr-id-ba04d6538388103765e1b1b1923cb13c7e02a912/> Add remove/upgrade pending ui
 - <csr-id-1bcb6014b2e700be7688dd9ce51fdf33c14f58bc/> Pending ui is back!
 - <csr-id-ffdf00479d3bfa28584c1631cff02cc244c40095/> More args back
 - <csr-id-b985f03b86a9e9c6727e5747ff3c05ce81861647/> --install-dbg flag is back
 - <csr-id-8637f8cb127faf50f7499b72e991a3e235dace7a/> Improve api design
 - <csr-id-155bf1f3341ec15bf2955b86b42fc86dffa01822/> Done OmaApt::commit function
 - <csr-id-bf1b4c19486425a857502ebeaa3f0d5be9723504/> Done for operation_map
 - <csr-id-2bc80ea4e116a625b4acdcf4a9066b424de2e43a/> Improve lifetime logic
 - <csr-id-90af4a45c3414783e97067a8790ff85e9fa9a1d0/> Api adjust
 - <csr-id-ecb46d44b356e994225e00c5cc16439198fd4ff3/> Pkg.rs => oma-pm

### Style

 - <csr-id-bb833287d6d439c622e737148d609c1b848e5efa/> Run cargo clippy and cargo fmt to lint code
 - <csr-id-9eeae30d50d3ed3a1c06364bbdb83b6faea47211/> Remove useless line

### Test

 - <csr-id-682ff6893c55727477807993a4cc23a0d34278f7/> Update example
 - <csr-id-f1296616be46f7fff77aaa7989a6e4028b04b0ba/> Example texlive -> vscodium to save your sweet time
 - <csr-id-c73e9886486d395cfc9eb119557226c85399406e/> Add download pkgs example
 - <csr-id-e6abdebe84c5155c029218685fdafa54086abcb2/> Refactor again
 - <csr-id-583e126cd32a3ea19f11084cdcebdc50395f2975/> Refactor
 - <csr-id-3dbc72701d26037b0e569bf3ebeb01f911965313/> Add example
 - <csr-id-7475d11b51a6488cb77bae231b6a6bee95f603b0/> Add test_branch_search

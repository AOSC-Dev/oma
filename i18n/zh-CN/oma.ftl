# utils
can-not-run-dpkg-print-arch = 无法执行 `dpkg --print-architecture'：{$e}
dpkg-return-non-zero = `dpkg' 返回错误：{$e}
need-more-size = 存储空间不足：{$a} 可用，但需要 {$n}。
old-pid-still-running = 目前有另外一个正在运行 Omakase 的实例 (PID: {$pid})，现中止操作。
can-not-create-lock-dir = 无法创建 /run/lock：{$e}
can-not-create-lock-file = 无法创建进程锁文件：{$e}
can-not-write-lock-file = 无法写入进程锁文件：{$e}
can-not-unlock-oma = 无法解锁 Omakase 进程：{$e}
can-not-create-oma-log-dir = 无法创建 Omakase 日志目录：{$e}
can-not-create-oma-log = 无法创建 Omakase 历史记录：{$e}
execute-pkexec-fail = 无法执行 `pkexec' 命令：{$e}

# verify
fail-load-certs-from-file = 无法从 {$path} 载入软件源签名。
cert-file-is-bad = 位于 {$path} 的软件源签名无效。
inrelease-bad-signature = InRelease 文件包含无效签名数据：{$e}
inrelease-must-signed = PGP 签名格式无效，InRelease 必须签名。

# topics
can-not-find-specified-topic = 找不到测试源：{$topic}
failed-to-enable-following-topics = 无法启用测试源：{$topic}
saving-topic-settings = 正在保存测试源开关状态 ...
do-not-edit-topic-sources-list = # 本文件使用 Omakase 生成，请勿编辑！
select-topics-dialog = 打开测试源以获取实验性更新，关闭测试源以回滚到稳定版本：
removing-topic = 正在关闭测试源：{$name}
tips = 按 [Space]/[Enter] 开关测试源，按 [q] 应用更改，按 [Ctrl-c] 退出。
scan-topic-is-removed = 测试源 {$name} 已从软件源中被删除，现将关闭 ...

# pkg
can-not-get-pkg-from-database = 无法从本机数据库中获取软件包 {$name} 的元数据。
can-not-get-pkg-version-from-database = 无法从本机获取软件包 {$name} ({$version}) 的元数据。
can-not-get-package-with-branch = 无法获取 {$branch} 分支上的软件包 {$name}。
debug-symbol-available = （调试符号可用）
full-match = 完整匹配
already-installed = 软件包 {$name} ({$version}) 已经安装。
can-not-mark-reinstall = 无法重装软件包 {$name} ({$version})，因为当前可用的软件源中找不到指定的软件包和版本。
mayble-dep-issue = 由于依赖问题，无法安装软件包 {$name}。
pkg-is-essential = 软件包 {$name} 是不允许删除的必备组件。

# pager
question-tips-with-x11 = 按 [q] 结束审阅并应用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn]、方向键或使用鼠标滚轮翻页。
normal-tips-with-x11 = 按 [q] 或 [Ctrl-c] 退出，按 [PgUp/Dn]、方向键或使用鼠标滚轮翻页。
question-tips = 按 [q] 结束审阅并应用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn] 或方向键翻页。
normal-tips = 按 [q] 或 [Ctrl-c] 退出，按 [PgUp/Dn] 或方向键翻页。

# oma
no-need-to-do-anything = 无需进行任何操作。
retry-apt = `apt' 返回错误，重试第 {$count} 次 ...
system-has-broken-dep = Omakase 探测到系统中存在依赖问题。
system-has-broken-dep-due-to = Omakase 可自动解决依赖问题，请使用 `{$cmd}' 命令。如果此命令无法解决问题，请移步 https://github.com/aosc-dev/aosc-os-abbs 报告问题。
additional-version = 另有 {$len} 个可用版本。请使用 `-a' 列出所有可用版本。
could-not-find-pkg-from-keyword = 无法找到匹配关键字 {$c} 的软件包
broken-by = Broken by
pre-depended-by = Pre-depended by
successfully-download-to-path = 已下载 {$len} 个软件包到该路径：{$path}
no-need-to-remove = 软件包 {$name} 尚未安装，因此无需卸载。
packages-can-be-upgrade = 有 {$len} 个可升级的软件包。
packages-can-be-removed = 有 {$len} 个可删除的软件包。
run-oma-upgrade-tips = 使用 `oma upgrade' 命令即可更新系统。
comma = ，
full-comma = 。
successfully-refresh-with-tips = Successfully refreshed the package database. {$s}
successfully-refresh = Successfully refreshed the package database. System is up to date.
no-candidate-ver = Current version for package {$pkg} is not available from the repository.
pkg-is-not-installed = Unable to mark package {$pkg}, as it is not yet installed.
dpkg-data-is-broken = Omakase failed to parse the dpkg database. The dpkg database may be corrupted.
already-hold = Package {$name} is already marked for version hold.
set-to-hold = Marked package {$name} for version hold.
already-unhold = Package {$name} is not yet marked for version hold.
set-to-unhold = Marked package {$name} for version unhold.
already-manual = Package {$name} is already marked as manually installed.
setting-manual = Marked package {$name} as manually installed.
already-auto = Package {$name} is already marked as automatically installed.
setting-auto = Marked package {$name} as automatically installed.
command-not-found-with-result = {$kw}: command not found. This command may be found from the following package(s):
command-not-found = {$kw}: command not found.
clean-successfully = Successfully cleaned Omakase database and cache.
dpkg-get-selections-non-zero = `dpkg --get-selections' returned an error. The dpkg database may be corrupted.
can-not-parse-line = Failed to parse line {$i} in the `dpkg --get-selections' output. The dpkg database may be corrupted.
dpkg-was-interrupted = A previous `dpkg' operation was interrupted, Omakase will now resume that operation ...
dpkg-configure-a-non-zero = `dpkg --configure -a' returned an error:
verifying-the-interity-of-pkgs = Verifying the integrity of downloaded packages ...
automatic-mode-warn = Running Omakase in unattended mode. If this is not intended, press Ctrl + C now to abort the operation!
has-no-symbol-pkg = Package {$name} has no debug symbol available.
pkg-no-version = Failed to get version of package {$name}.
removed-as-unneed-dep = removed as unneeded dependency
purge-file = purge configuration files
semicolon = ;
should-installed = BUG: Package {$name} marked for pending operation but it is not installed. This is a program exception. Please report this issue at https://github.com/AOSC-Dev/oma.

# main
user-aborted-op = User aborted the operation.

# formatter
download-not-done = Omakase has finished downloading packages, but the APT backend returned an inconsistent state. Please run Omakase in debug mode (using the `--debug' switch) and submit the log in a bug report at https://github.com/AOSC-Dev/oma.
force-auto-mode = Running Omakase in unattended mode with FORCED operations. If this is not  intended, press Ctrl + C now to stop the operation!
dpkg-force-all-mode = Running Omakase with DPKG FORCE ALL mode. If this is not intended, press Ctrl + C now to stop the operation!
dep-does-not-exist = Dependency package {$name} is not available from any available repository.
count-pkg-has-desc = {$count} package(s) has
dep-error = Dependency Error
dep-error-desc = Omakase has detected dependency errors(s) in your system and cannot proceed with
    your specified operation(s). This may be caused by missing or mismatched\npackages, or that you have specified a version of a package that is not
    compatible with your system.
contact-admin-tips = Please contact your system administrator or developer.
how-to-abort = Press [q] or [Ctrl-c] to abort.
how-to-op-with-x = Press [PgUp/Dn], arrow keys, or use the mouse wheel to scroll.
end-review = Press [q] to end review
cc-to-abort = Press [Ctrl-c] to abort
how-to-op = Press [PgUp/Dn] or arrow keys to scroll.
total-download-size = Total download size:
change-storage-usage = Estimated change in storage usage:
pending-op = Pending Operations
review-msg = Shown below is an overview of the pending changes Omakase will apply to your
    system, please review them carefully.
removed = REMOVED
installed = installed
upgrade = upgrade
downgraded = downgraded
reinstall = reinstall
oma-may = Omakase may {$a}, {$b}, {$c}, {$d}, or {$e} packages in order
    to fulfill your requested changes.
install = install
remove = remove
upgrade = upgrade
downgrade = downgrade

# download
invalid-url = BUG: URL is not valid. Please report this issue at https://github.com/AOSC-Dev/oma.
invaild-filename = Invalid file name: {$name}.
invaild-ver = Invalid version: {$ver}.
checksum-mismatch-try-next-url = Checksum verification failed for package {$c}. Retrying using the next available mirror ...
checksum-mismatch-retry = Checksum verification failed for package {$c}. Retrying {$retry} times ...
can-not-get-source-next-url = Failed to download package {$e}. Retrying using the next available mirror ...
checksum-mismatch = Checksum verification failed for package file {$filename} at {$dir}.
maybe-mirror-syncing = This could be caused by an incomplete or in progress mirror sync.
can-not-download-file = Failed to download package {$filename}: {$e}
check-network-settings = Please verify your network settings.
can-not-download-file-with-why = Failed to write package file {$filename} to {$dir}: {$e}
downloading-count-pkg = Downloading {$count} packages ...
progress = Progress:
success-download-pkg = Downloaded {$download_len} package.
no-need-to-fetch-anything = No need to fetch anything.
can-not-get-filename = BUG: Cannot read file {$name}. Please report this issue at https://github.com/AOSC-Dev/oma.

# db
setting-path-does-not-exist = Specified package cache directory {$path} does not exist. Falling back to /var/cache/apt/archives.
invaild-url-with-err = Invalid URL {$url}: {$e}
cant-parse-distro-repo-data = Failed to parse distribution repository data file {$mirror}: {$e}.
distro-repo-data-invalid-url = Distribution repository data file contains invalid URL {$url}: {$e}
host-str-err = Failed to detect the hostname of the specified mirror.
can-nnot-read-inrelease-file = Failed to parse InRelease at {$path}: {$e}
inrelease-date-empty = InRelease file is invalid: The Date field is empty.
inrelease-valid-until-empty = InRelease file is invalid: The Valid-Until entry is empty.
can-not-parse-date = BUG: Failed to parse the Date field {$date} to the RFC2822 format. Please report this issue at https://github.com/AOSC-Dev/oma.
can-not-parse-valid-until = BUG: Failed to parse the Valid-Until field {$valid_until} to the RFC2822 format. Please report this issue at https://github.com/AOSC-Dev/oma.
earlier-signature = InRelease file is invalid: System time is earlier than the signature timestamp in InRelease.
expired-signature = InRelease file is invalid: The signature file has expired
inrelease-sha256-empty = InRelease file is invalid: The SHA256 field is empty!
inrelease-checksum-can-not-parse = InRelease file is invalid: Failed to parse checksum entry {$i}
inrelease-parse-unsupport-file-type = BUG: InRelease Parser has encountered an unsupport file format. Please report this issue at https://github.com/AOSC-Dev/oma.
can-not-parse-sources-list = Failed parse the sources.list file: {$e}
unsupport-cdrom = Omakase does not support the cdrom:// protocol: {$url}
unsupport-some-mirror = Omakase has detected unsupported entries in sources.list.
unsupport-sourceentry = Unsupported sources.list entry(ies):
refreshing-repo-metadata = Refreshing local database ...
can-not-get-suite = Failed to detect suite from sources.list entry: {$url}
not-found = Failed to download InRelease from URL {$url}: 404 Not found.
contents = `Contents' file
pkg_list = `Packages' file
bincontents = `BinContents' file
decompressing = Decompressing the
unsupport-decompress-file = BUG: Omakase has encountered an unsupported compression method in {$name}. Please report this issue at https://github.com/AOSC-Dev/oma.

# contents
contents-does-not-exist = Package contents database (Contents) does not exist. Use the {$cmd} command to refresh the contents database.
contents-may-not-be-accurate = The local package contents database {$file} has not been updated for over a week, search results may not be accurate. Use the `oma refresh' command to refresh the contents database.
execute-ripgrep-failed = Failed to execute `rg': {$e}
searching = Searching ...
parse-rg-result-failed = BUG: Failed to parse `rg' result {$i}: {$e}. Please report this issue at https://github.com/AOSC-Dev/oma.
search-with-result-count = Searching, found {$count} results so far ...
contents-entry-missing-path-list = BUG: Omakase failed to parse an entry {$entry} in the local contents database. Please report this issue at https://github.com/AOSC-Dev/oma.
rg-non-zero = `rg' returned an error.

# checksum
sha256-bad-length = Malformed SHA256 checksum: bad length.
can-not-checksum = BUG: Failed to parse SHA256 checksum {$e}. Please report this issue at https://github.com/AOSC-Dev/oma.
failed-to-open-to-checksum = BUG: Failed to open {$path} for checksum verification. Please report this issue at https://github.com/AOSC-Dev/oma.

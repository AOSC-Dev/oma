# utils
can-not-run-dpkg-print-arch = 无法执行 `dpkg --print-architecture'：{$e}。
dpkg-return-non-zero = `dpkg' 返回错误：{$e}.
need-more-size = 存储空间不足：{$a} 可用，但需要 {$n}。
old-pid-still-running = 目前有另外一个正在运行 Omakase 的实例 (PID: {$pid})，现中止操作。
can-not-create-lock-dir = 无法创建 /run/lock：{$e}。
can-not-create-lock-file = 无法创建进程锁文件：{$e}。
can-not-write-lock-file = 无法写入进程锁文件：{$e}。
can-not-unlock-oma = 无法解锁 Omakase 进程：{$e}。
execute-pkexec-fail = 无法执行 `pkexec' 命令：{$e}。

# history
can-not-create-oma-log-dir = 无法创建 Omakase 日志目录：{$e}。
can-not-create-oma-log = 无法创建 Omakase 历史记录文件：{$e}。
can-not-create-oma-log-database = 无法创建 Omakase 历史数据库文件：{$e}。
can-not-read-oma-log-database = 无法读取 Omakase 历史数据库文件：{$e}，该数据库可能已损坏。
can-not-ser-oma-log-database = BUG：无法序列化 Omakase 历史数据库：{$e}，请于 https://github.com/AOSC-Dev/oma 报告问题。
can-not-deser-oma-log-database = BUG：无法反序列化 Omakase 历史数据库：{$e}，请于 https://github.com/AOSC-Dev/oma 报告问题。
invaild-index = 历史操作编号无效：{$index}。
index-is-nothing = 历史操作编号 {$index} 未包含任何操作记录。
select-op-undo = 请选择要撤销的历史操作编号：
select-op-redo = 请选择要复用的历史操作编号：
history-tips-1 = Omakase 已成功应用对系统的更改。
history-tips-2 = 如需撤销本次操作，请使用 `oma history {$undo_or_redo}' 命令。

# verify
fail-load-certs-from-file = 无法从 {$path} 载入软件源签名。
cert-file-is-bad = 位于 {$path} 的软件源签名无效。
inrelease-bad-signature = InRelease 文件包含无效签名数据：{$e}。
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
refreshing-topic-metadata = 正在刷新测试源数据 ...

# pkg
can-not-get-pkg-from-database = 无法从本机数据库中获取软件包 {$name} 的元数据。
can-not-get-pkg-version-from-database = 无法从本机获取软件包 {$name} ({$version}) 的元数据。
can-not-get-package-with-branch = 无法获取 {$branch} 分支上的软件包 {$name}。
invaild-path = 非法路径：{$p}
debug-symbol-available = （调试符号可用）
full-match = 完整匹配
already-installed = 软件包 {$name} ({$version}) 已经安装。
can-not-mark-reinstall = 无法重装软件包 {$name} ({$version})，因为当前可用的软件源中找不到指定的软件包和版本。
mayble-dep-issue = 由于依赖问题，无法安装软件包 {$name}。
pkg-is-essential = 软件包 {$name} 是不允许删除的必备组件。
pkg-search-avail = AVAIL
pkg-search-installed = INSTALLED
pkg-search-upgrade = UPGRADE
pkg-no-checksum = 软件包 {$name} 没有校验码。
flushing-data = 正在将数据写入至磁盘 …

# pager
question-tips-with-x11 = 按 [q] 结束审阅并应用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn]、方向键或使用鼠标滚轮翻页。
normal-tips-with-x11 = 按 [q] 或 [Ctrl-c] 退出，按 [PgUp/Dn]、方向键或使用鼠标滚轮翻页。
question-tips = 按 [q] 结束审阅并应用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn] 或方向键翻页。
normal-tips = 按 [q] 或 [Ctrl-c] 退出，按 [PgUp/Dn] 或方向键翻页。

# oma
no-need-to-do-anything = 无需进行任何操作。
retry-apt = `apt' 返回错误，重试第 {$count} 次 ...
apt-error = `apt' 返回了错误：{$e}
invaild-pattern = 非法的表达式：{$p}
system-has-broken-dep = Omakase 探测到系统中存在依赖问题。
system-has-broken-dep-due-to = Omakase 可自动解决依赖问题，请使用 `{$cmd}' 命令。如果此命令无法解决问题，请移步 https://github.com/aosc-dev/aosc-os-abbs 报告问题。
additional-version = 另有 {$len} 个可用版本。请使用 `-a' 列出所有可用版本。
could-not-find-pkg-from-keyword = 无法找到匹配关键字 {$c} 的软件包
depends = Depends
pre-depends = Pre-Depends
suggests = Suggests
recommends = Recommends
conflicts = Conflicts
replaces = Replaces
obsoletes = Obsoletes
breaks = Breaks
enhances = Enhances
depended-by = Depended by
broken-by = Broken by
pre-depended-by = Pre-Depended by
suggested-by = Suggested by
recommended-by = Recommended by
conflicted-by = Conflicted by
replaced-by = Replaced by
obsoleted-by = Obsoleted by
enhanced-by = Enhanced by
successfully-download-to-path = 已下载 {$len} 个软件包到该路径：{$path}
no-need-to-remove = 软件包 {$name} 尚未安装，因此无需卸载。
packages-can-be-upgrade = 有 {$len} 个可升级的软件包。
packages-can-be-removed = 有 {$len} 个可删除的软件包。
run-oma-upgrade-tips = 使用 `oma upgrade' 命令即可更新系统。
comma = ，
full-comma = 。
successfully-refresh-with-tips = 成功刷新本机软件包数据库。{$s}
successfully-refresh = 成功刷新本机软件包数据库。系统各软件包均为最新。
no-candidate-ver = 无法从软件包仓库中获取当前版本的软件包 {$pkg} 。
pkg-is-not-installed = 无法标记软件包 {$pkg} 的属性，因为该软件包尚未安装。
dpkg-data-is-broken = Omakase 无法解析 dpkg 数据库。该数据库可能已损坏。
already-hold = 软件包 {$name} 已被标记为版本锁定。
set-to-hold = 成功标记软件包 {$name} 属性：版本锁定。
already-unhold = 软件包 {$name} 尚未标记为版本锁定。
set-to-unhold = 成功标记软件包 {$name} 属性：版本解锁。
already-manual = 软件包 {$name} 已被标记为手动安装。
setting-manual = 成功标记软件包 {$name} 属性：手动安装。
already-auto = 软件包 {$name} 已被标记为自动安装。
setting-auto = 成功标记软件包 {$name} 属性：自动安装。
command-not-found-with-result = {$kw}：找不到命令。该命令由如下软件包提供：
command-not-found = {$kw}：找不到命令。
clean-successfully = 成功清理 Omakase 本机数据库和缓存。
dpkg-get-selections-non-zero = `dpkg --get-selections' 返回错误。dpkg 数据库可能已损坏。
can-not-parse-line = 无法解析 `dpkg --get-selections' 命令输出的第 {$i} 行。dpkg 数据库可能已损坏。
dpkg-was-interrupted = 先前 `dpkg' 操作被打断，Omakase 现将继续操作 ...
dpkg-configure-a-non-zero = `dpkg --configure -a' 返回错误：{$e}
verifying-the-interity-of-pkgs = 正在验证本机软件包的完整性 ...
automatic-mode-warn = 正以无人值守模式运行 Omakase。如非本人所为，请立即按 Ctrl + C 中止操作！
has-no-symbol-pkg = 软件包 {$name} 没有可用调试符号。
pkg-no-version = 无法获取软件包 {$name} 的版本号。
removed-as-unneed-dep = 清理未使用的依赖
purge-file = 清理配置文件
semicolon = ；
should-installed = BUG：待操作清单中包含软件包 {$name}，但系统中尚未安装该软件包。请于 https://github.com/AOSC-Dev/oma 报告该程序异常。
pick-tips = 请指定软件包 {$pkgname} 的版本：

# main
user-aborted-op = 用户已中止操作。

# formatter
download-not-done = Omakase 已下载软件包，但 APT 后端报告运行状态不一致。请使用调试模式运行 Omakase （`--debug' 参数）并于 https://github.com/AOSC-Dev/oma 提交日志。
force-auto-mode = 正以无人值守模式运行 Omakase，且开启了强制模式。如非本人所为，请立即按 Ctrl + C 中止操作！
dpkg-force-all-mode = 正以 dpkg 强制执行模式运行 Omakase（此时将忽略依赖不满足等问题）。如非本人所为，请立即按 Ctrl + C 中止操作！
dep-does-not-exist = 无法从软件包仓库中获取依赖 {$name} 。
count-pkg-has-desc = {$count} 个软件包将被
dep-error = 依赖关系错误
dep-error-desc = Omakase 探测到依赖关系问题，因此无法继续进行指定操作。该问题可能是
    软件包缺失或来源不统一，抑或是指定的软件包版本与当前系统不兼容导致的。
contact-admin-tips = 请联系您的系统管理员或开发者。
how-to-abort = 按 [q] 或 [Ctrl-c] 中止操作
how-to-op-with-x = 按 [PgUp/Dn]、方向键或使用鼠标滚轮翻页
end-review = 按 [q] 结束审阅并应用更改
cc-to-abort = 按 [Ctrl-c] 中止操作
how-to-op = 按 [PgUp/Dn] 或方向键翻页
total-download-size = {"总下载大小： "}
change-storage-usage = {"预计磁盘占用变化： "}
pending-op = 待操作清单
review-msg = Omakase 将执行如下操作，请仔细验证。
removed = 卸载
installed = 安装
upgrade = 更新
downgraded = 降级
reinstall = 重装
oma-may = 为应用您指定的更改，Omakase 可能 {$a}、{$b}、{$c}、{$d} 或 {$e} 软件包。
install = 安装
remove = 卸载
downgrade = 降级
unmet-dep = 无法满足
colon = ：
unmet-dep-before = 有 {$count} 个软件包的依赖

# download
invalid-url = BUG：URL 无效，请于 https://github.com/AOSC-Dev/oma 报告问题。
invaild-filename = 文件名 {$name} 无效。
invaild-ver = 版本号 {$ver} 无效。
checksum-mismatch-try-next-url = 软件包 {$c} 完整性验证失败，将使用下一个镜像源重试 ...
checksum-mismatch-retry = 软件包 {$c} 完整性验证失败，将重试 {$retry} 次 ...
can-not-get-source-next-url = 无法下载软件包 {$e}，将使用下一个镜像源重试 ...
checksum-mismatch = 位于 {$dir} 的软件包文件 {$filename} 完整性验证失败。
maybe-mirror-syncing = 该问题可能是软件镜像源同步未完成造成的。
can-not-download-file = 无法下载软件包 {$filename}：{$e}
check-network-settings = 请检查您的网络设置。
can-not-download-file-with-why = 无法在 {$dir} 写入软件包文件 {$filename}：{$e}
downloading-count-pkg = 正在下载 {$count} 个软件包 ...
progress = 总进度：
success-download-pkg = 成功下载 {$download_len} 个软件包。
no-need-to-fetch-anything = 所有软件包均已于本机缓存，无需下载。
can-not-get-filename = BUG：无法读取文件 {$name}，请于 https://github.com/AOSC-Dev/oma 报告问题。
can-not-get-file = 无法打开文件 {$name}: {$e}
not-found-other = 无法从 {$url} 下载文件：找不到远端文件 (HTTP 404)。
io-error = Omakase 遇到了 I/O 错误：{$e}

# db
setting-path-does-not-exist = 找不到指定的软件包缓存目录 {$path}，将使用默认缓存目录 /var/cache/apt/archives 。
invaild-url-with-err = URL {$url} 无效：{$e}
cant-parse-distro-repo-data = 无法解析软件包镜像源数据文件 {$mirror}：{$e}。
distro-repo-data-invalid-url = 软件包镜像源数据文件中包含无效 URL {$url}：{$e}。
host-str-err = 无法探测指定镜像源的主机名。
can-nnot-read-inrelease-file = 无法解析位于 {$path} 的 InRelease 文件：{$e}。
inrelease-date-empty = InRelease 文件无效：Date 值为空。
inrelease-valid-until-empty = InRelease 文件无效：Valid-Until 值为空。
can-not-parse-date = BUG：无法将 Date 值 {$date} 转换为 RFC2822 格式，请于 https://github.com/AOSC-Dev/oma 报告问题。
can-not-parse-valid-until = BUG：无法将 Valid-Until 值 {$valid_until} 转换为 RFC2822 格式，请于 https://github.com/AOSC-Dev/oma 报告问题。
earlier-signature = InRelease 文件 {$filename} 无效：系统时间早于内联签名时间戳。
expired-signature = InRelease 文件 {$filename} 无效：内联签名已过期。
inrelease-sha256-empty = InRelease 文件无效：SHA256 值为空。
inrelease-checksum-can-not-parse = InRelease 文件无效：无法解析校验和条目 {$i}。
inrelease-parse-unsupport-file-type = BUG：解析器不支持该 InRelease 文件的格式，请于 https://github.com/AOSC-Dev/oma 报告问题。
can-not-parse-sources-list = 无法解析 sources.list 文件 {path}：{$e}。
unsupport-cdrom = Omakase 不支持 cdrom:// 协议：{$url}。
unsupport-some-mirror = Omakase 在 sources.list 文件中探测到无效条目。
unsupport-sourceentry = 探测到不受支持的 sources.list 文件条目：
refreshing-repo-metadata = 正在刷新本机软件包数据库 ...
can-not-get-suite = 无法从 sources.list 条目中解析套件信息：{$url}。
not-found = 无法从 {$url} 下载 InRelease 文件：找不到远端文件 (HTTP 404)。
contents = `Contents'
pkg_list = `Packages'
bincontents = `BinContents'
decompressing = 正在解压
unsupport-decompress-file = BUG：Omakase 不支持文件 {$name} 所使用的压缩方式，请于 https://github.com/AOSC-Dev/oma 报告问题。
downloading-database = {$source} {$file}

# contents
contents-does-not-exist = 找不到软件包内容数据库文件 (Contents)，请使用 {$cmd} 命令刷新该数据库。
contents-may-not-be-accurate-1 = 本机软件包内容数据库文件已超过一周未有更新，因此搜索结果可能不准确。
contents-may-not-be-accurate-2 = 请使用 `oma refresh' 命令刷新该数据库。
execute-ripgrep-failed = 无法执行 `rg' 命令：{$e}。
searching = 正在搜索 ...
parse-rg-result-failed = BUG：无法解析 `rg' 命令输出 {$i}：{$e}。请于 https://github.com/AOSC-Dev/oma 报告问题。
search-with-result-count = 正在搜索，已找到 {$count} 个结果 ...
contents-entry-missing-path-list = BUG：Omakase 无法解析本机软件包内容数据库中的条目 {$entry}，请于 https://github.com/AOSC-Dev/oma 报告问题。
rg-non-zero = `rg' 报错退出。

# checksum
sha256-bad-length = SHA256 校验和无效：长度不正确。
can-not-checksum = BUG：无碍发解析 SHA256 校验和 {$e}，请于 https://github.com/AOSC-Dev/oma 报告问题。
failed-to-open-to-checksum = BUG：无法打开用于验证校验和的路径 {$path}，请于 https://github.com/AOSC-Dev/oma 报告问题。

# config
config-invaild = Invaild Config /etc/oma.toml! fallbacking to default configuration.

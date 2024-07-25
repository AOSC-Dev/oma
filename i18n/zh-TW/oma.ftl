# utils
can-not-run-dpkg-print-arch = 無法執行 `dpkg --print-architecture'。
execute-pkexec-fail = 無法執行 `pkexec' 指令：{ $e }。
# history
history-tips-1 = oma 已成功套用對系統的更改。
history-tips-2 = 如需取消本次操作，請使用 `oma undo' 指令。
# verify
fail-load-certs-from-file = 無法從 { $path } 載入軟體庫簽章。
cert-file-is-bad = 位於 { $path } 的軟體庫簽章無效。
# topics
can-not-find-specified-topic = 找不到測試庫：{ $topic }。
do-not-edit-topic-sources-list = # 本檔案使用 oma 產生，請勿編輯！
select-topics-dialog = 打開測試庫以取得實驗性更新，關閉測試庫以回退到穩定版本：
tips = 按 [Space]/[Enter] 開關測試庫，按 [Esc] 套用更改，按 [Ctrl-c] 結束。
scan-topic-is-removed = 測試庫 { $name } 已從軟體庫移除，現將關閉……
refreshing-topic-metadata = 正在重新整理測試庫資料……
failed-to-read = 無法讀取狀態檔案。
# pkg
can-not-get-pkg-from-database = 無法從本機資料庫中取得軟體套件 { $name } 的後設資料。
invaild-path = 路徑格式有誤：{ $p }
debug-symbol-available = 除錯符號可用
full-match = 完全符合
already-installed = 軟體套件 { $name } ({ $version }) 已經安裝
can-not-mark-reinstall = 無法重裝軟體套件 { $name } ({ $version })，因為目前可用的軟體庫中找不到指定的軟體套件和版本。
pkg-is-essential = 軟體套件 { $name } 是不允許移除的關鍵組件。
pkg-search-avail = AVAIL
pkg-search-installed = INSTALLED
pkg-search-upgrade = UPGRADE
pkg-no-checksum = 軟體套件 { $name } 沒有雜湊值。
pkg-unavailable = 軟體庫中找不到 { $ver } 版本的軟體套件 { $pkg }。
# pager
question-tips-with-x11 = 按 [q] 結束檢閱並套用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁。
normal-tips-with-x11 = 按 [q] 或 [Ctrl-c] 結束，按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁。
question-tips = 按 [q] 結束檢閱並套用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn] 或方向鍵翻頁。
normal-tips = 按 [q] 或 [Ctrl-c] 結束，按 [PgUp/Dn] 或方向鍵翻頁。
# oma
no-need-to-do-anything = 無需進行任何操作。
apt-error = `apt' 回傳了錯誤。
invaild-pattern = 表達式格式有誤：{ $p }
additional-version = 另有 { $len } 個可用版本。請使用 `-a' 列出所有可用版本。
could-not-find-pkg-from-keyword = 無法找到符合關鍵字 { $c } 的軟體套件。
no-need-to-remove = 軟體套件 { $name } 尚未安裝，因此無需解除安裝。
packages-can-be-upgrade = 有 { $len } 個可升級的軟體套件。
packages-can-be-removed = 有 { $len } 個可移除的軟體套件。
comma = ，
successfully-refresh-with-tips = 成功重新整理本機軟體套件資料庫。{ $s }
successfully-refresh = 成功重新整理本機軟體套件資料庫。系統各軟體套件均為最新。
no-candidate-ver = 無法從軟體套件資料庫中取得目前版本的軟體套件 { $pkg } 。
pkg-is-not-installed = 無法標記軟體套件 { $pkg } 的屬性，因為該軟體套件尚未安裝。
already-hold = 軟體套件 { $name } 已被標記為版本鎖定。
set-to-hold = 成功標記軟體套件 { $name } 屬性：版本鎖定。
already-unhold = 軟體套件 { $name } 尚未標記為版本鎖定。
set-to-unhold = 成功標記軟體套件 { $name } 屬性：版本解鎖。
already-manual = 軟體套件 { $name } 已被標記為手動安裝。
setting-manual = 成功標記軟體套件 { $name } 屬性：手動安裝。
already-auto = 軟體套件 { $name } 已被標記為自動安裝。
setting-auto = 成功標記軟體套件 { $name } 屬性：自動安裝。
command-not-found-with-result = { $kw }：找不到指令。該指令由如下軟體套件提供：
command-not-found = { $kw }：找不到指令。
clean-successfully = 成功清理 oma 本機資料庫和快取。
dpkg-configure-a-non-zero = `dpkg --configure -a' 回傳了錯誤。
automatic-mode-warn = 正以無人值守模式執行 oma。如非本人所為，請立即按 Ctrl + C 中止操作！
removed-as-unneed-dep = 清理未使用的依賴
purge-file = 清理設定檔
semicolon = ；
pick-tips = 請指定要安裝的 { $pkgname } 的版本：
battery = 您的電腦目前似乎正在使用電池供電。oma 在執行任務時可能消耗大量電量，建議插電以防斷電導致資料損壞。
continue = 您確定要繼續嗎？
changing-system = oma 正在更改你的系統。
failed-to-lock-oma = 無法解鎖 oma 行程鎖檔案 (/run/lock/oma.lock)
# main
user-aborted-op = 使用者已中止操作。
# formatter
count-pkg-has-desc = { $count } 個軟體套件將被
dep-issue-1 = oma 發現指定軟體套件存在依賴問題，故無法安裝。
dep-issue-2 = 如下是該依賴問題的完整分析報告，請複製或截圖如下信息並聯系 AOSC OS 維護人員：
<<<<<<< HEAD
how-to-op-with-x = 按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁
=======
contact-admin-tips = 請聯繫您的系統管理員或開發人員。
how-to-abort = 按 [q] 或 [Ctrl-c] 中止操作。
how-to-op-with-x = 按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁。
>>>>>>> weblate/master
end-review = 按 [q] 結束檢閱並套用更改
cc-to-abort = 按 [Ctrl-c] 中止操作
how-to-op = 按 [PgUp/Dn] 或方向鍵翻頁。
total-download-size = { "總下載大小： " }
change-storage-usage = { "預計磁碟佔用變化： " }
pending-op = 待操作清單
review-msg = oma 將執行如下操作，請仔細驗證。
install = 安裝
installed = 安裝
remove = 解除安裝
removed = 解除安裝
upgrade = 更新
upgraded = 更新
downgrade = 降級
downgraded = 降級
reinstall = 重裝
reinstalled = 重裝
colon = ：
<<<<<<< HEAD

=======
unmet-dep-before = 有 { $count } 個軟體套件的依賴
>>>>>>> weblate/master
# download
invaild-filename = 檔案名 { $name } 無效。
checksum-mismatch-retry = 檔案 { $c } 完整性驗證失敗，正在重試第 { $retry } 次……
can-not-get-source-next-url = 無法下載檔案：{ $e }，將使用下一個鏡像源重試……
checksum-mismatch = 檔案 { $filename } 完整性驗證失敗。
# db
invaild-url = URL { $url } 無效
can-not-parse-date = BUG：無法將 Date 值轉換為 RFC2822 格式，請於 https://github.com/AOSC-Dev/oma 報告問題。
can-not-parse-valid-until = BUG：無法將 Valid-Until 值轉換為 RFC2822 格式，請於 https://github.com/AOSC-Dev/oma 報告問題。
earlier-signature = InRelease 檔案 { $filename } 無效：系統時間早於內附簽章時間戳。
expired-signature = InRelease 檔案 { $filename } 無效：內附簽章已過期。
inrelease-sha256-empty = InRelease 檔案無效：SHA256 值為空。
inrelease-checksum-can-not-parse = InRelease 檔案無效：無法解析雜湊值條目 { $i }。
inrelease-parse-unsupport-file-type = BUG：解析器不支援該 InRelease 檔案的格式，請於 https://github.com/AOSC-Dev/oma 報告問題。
can-not-parse-sources-list = 無法解析 sources.list 檔案 { $path }。
unsupport-protocol = oma 不支援協定：{ $url }。
refreshing-repo-metadata = 正在重新整理本機軟體套件資料庫……
not-found = 無法從 { $url } 下載 InRelease 檔案：找不到遠端檔案 (HTTP 404)。
inrelease-syntax-error = 位於 { $path } 的 InRelease 檔案解析失敗。
# contents
contents-does-not-exist = 找不到軟體套件內容資料庫檔案 (Contents)。
contents-may-not-be-accurate-1 = 本機軟體套件內容資料庫檔案已超過一週未有更新，因此搜尋結果可能不準確。
contents-may-not-be-accurate-2 = 請使用 `oma refresh' 指令重新整理該資料庫。
execute-ripgrep-failed = 無法執行 `rg' 指令。
searching = 正在搜尋……
search-with-result-count = 正在搜尋，已找到 { $count } 個結果……
contents-entry-missing-path-list = BUG：oma 無法解析本機軟體套件內容資料庫中的條目 { $entry }，請於 https://github.com/AOSC-Dev/oma 報告問題。
rg-non-zero = `rg' 報錯結束。
# checksum
sha256-bad-length = SHA256 雜湊值無效：長度不正確。
can-not-checksum = 無法解析 SHA256 雜湊值。
failed-to-open-to-checksum = BUG：無法打開用於驗證雜湊值的路徑 { $path }，請於 https://github.com/AOSC-Dev/oma 報告問題。
# config
config-invaild = oma 設定檔 (/etc/oma.toml) 似乎已損壞！將使用預設設定。
cleaning = 正在清理本地軟體套件……
download-failed-with-len = { $len } 個軟體套件下載失敗。
download-failed = 下載 { $filename } 檔案失敗！
download-failed-no-name = 下載檔案失敗！
need-more-size = 儲存空間不足：{ $a } 可用，但需要 { $n }。
successfully-download-to-path = 已下載 { $len } 個軟體套件到該路徑：{ $path }
oma-may = 為套用您指定的更改，oma 可能 { $a }、{ $b }、{ $c }、{ $d } 或 { $e } 軟體套件。
failed-to-read-decode-inrelease = 無法讀取解密後的 InRelease 檔案。
failed-to-operate-path = 無法在路徑 { $p } 中執行檔案操作。
failed-to-get-parent-path = 無法取得路徑 { $p } 的上層目錄。
failed-to-read-file-metadata = 無法讀取 { $p } 的檔案後設資料。
failed-to-get-rg-process-info = 無法取得 `rg' 的行程狀態。
failed-to-calculate-available-space = 無法計算可用儲存空間。
failed-to-create-http-client = 無法建立 HTTP 用戶端。
failed-to-connect-history-database = 無法連接到歷史資料庫。
failed-to-execute-query-stmt = 無法在歷史資料庫中執行查詢指令。
failed-to-parse-history-object = 無法解析歷史資料庫中的物件。

failed-to-set-lockscreen = 無法設定系統熒幕鎖定狀態。
failed-to-create-proxy = 無法建立系統訊息匯流排 (D-Bus) 代理：{ $proxy }。

failed-check-dbus = 由於目前管理的系統尚未啟動，oma 無法探測系統執行狀態。
failed-check-dbus-tips-1 = 在此狀態下，oma 無法檢查電源及使用者工作階段等關鍵系統狀態，如繼續操作可能會導致系統故障！
failed-check-dbus-tips-2 = 如果您正嘗試用 chroot 等方式修復系統，請在 oma 指令後加 --no-check-dbus 參數。
failed-check-dbus-tips-3 = 如果您的系統是長期在容器或 chroot 環境下執行的，請更改 oma 設定檔 (/etc/oma.toml) 下的 `no_check_dbus' 選項為 `true' 。
no-check-dbus-tips = 目前 oma 已被設定為不探測系統執行狀態，將忽略電源及使用者工作階段等關鍵系統狀態；如繼續操作可能會導致系統故障。
oma-history-is-empty = oma 歷史記錄為空。
tui-pending = 待辦事項
tui-search = 搜尋軟體套件
tui-packages = 軟體套件列表（{ $u } 可更新，{ $r } 可移除，{ $i } 已安裝）
tui-start-1 = 歡迎使用小熊貓套件管理器！
tui-start-2 = 切換面板
tui-start-3 = 顯示/隱藏待辦事項
tui-start-4 = 套用更改
tui-start-5 = 添加/移除操作條目
tui-start-6 = 搜尋
tui-start-7 = 結束
tui-start = 開始使用
oma = 小熊貓套件管理器
another-oma-is-running = 有另一個 oma 實例正在執行中：{ $s }。
table-name = 套件名稱
table-version = 版本
table-size = 大小
table-detail = 註記
reading-database = 正在讀取軟體套件資料庫
has-error-on-top = 無法執行您指定的操作，請參閱上述報錯細節。
# pager
question-tips-with-gui = 按 [q] 結束檢閱並套用更改，按 [Ctrl-c] 中止操作，按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁。
normal-tips-with-gui = 按 [q] 或 [Ctrl-c] 退出， 按 [PgUp/Dn]、方向鍵或使用滑鼠滾輪翻頁。

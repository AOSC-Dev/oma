function _oma_packages
    oma pkgnames 2> /dev/null
end

function _oma_packages_installed
    oma pkgnames --installed 2> /dev/null
end

complete -c oma -n "__fish_use_subcommand" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from install" -s o -l apt-options -r
complete -c oma -n "__fish_use_subcommand" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_use_subcommand" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_use_subcommand" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_use_subcommand" -l no-check-dbus
complete -c oma -n "__fish_use_subcommand" -s v -l version -d 'Print version'
complete -c oma -n "__fish_use_subcommand" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_use_subcommand" -f -a "install" -d 'Install package(s) from the repository'
complete -c oma -n "__fish_use_subcommand" -f -a "upgrade" -d 'Upgrade packages installed on the system'
complete -c oma -n "__fish_use_subcommand" -f -a "download" -d 'Download package(s) from the repository'
complete -c oma -n "__fish_use_subcommand" -f -a "remove" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_use_subcommand" -f -a "purge" -d 'purge (like apt purge) the specified package(s)'
complete -c oma -n "__fish_use_subcommand" -f -a "refresh" -d 'Refresh repository metadata/catalog'
complete -c oma -n "__fish_use_subcommand" -f -a "show" -d 'Show information on the specified package(s)'
complete -c oma -n "__fish_use_subcommand" -f -a "search" -d 'Search for package(s) available from the repository'
complete -c oma -n "__fish_use_subcommand" -f -a "files" -d 'List files in the specified package'
complete -c oma -n "__fish_use_subcommand" -f -a "provides" -d 'Search for package(s) that provide(s) certain patterns in a path'
complete -c oma -n "__fish_use_subcommand" -f -a "fix-broken" -d 'Resolve broken system dependencies in the system'
complete -c oma -n "__fish_use_subcommand" -f -a "pick" -d 'Install specific version of a package'
complete -c oma -n "__fish_use_subcommand" -f -a "mark" -d 'Mark status for one or multiple package(s)'
complete -c oma -n "__fish_use_subcommand" -f -a "command-not-found"
complete -c oma -n "__fish_use_subcommand" -f -a "list" -d 'List package(s) available from the repository'
complete -c oma -n "__fish_use_subcommand" -f -a "depends" -d 'Lists dependencies of one or multiple packages'
complete -c oma -n "__fish_use_subcommand" -f -a "rdepends" -d 'List reverse dependency(ies) for the specified package(s)'
complete -c oma -n "__fish_use_subcommand" -f -a "clean" -d 'Clear downloaded package cache'
complete -c oma -n "__fish_use_subcommand" -f -a "history" -d 'Show a history/log of package changes in the system'
complete -c oma -n "__fish_use_subcommand" -f -a "undo" -d 'Undo system changes operation'
complete -c oma -n "__fish_use_subcommand" -f -a "pkgnames"
complete -c oma -n "__fish_use_subcommand" -f -a "mirror" -d ''
complete -c oma -n "__fish_use_subcommand" -f -a "mirrors" -d ''
complete -c oma -n "__fish_use_subcommand" -f -a "tui" -d 'Oma tui interface'
complete -c oma -n "__fish_use_subcommand" -f -a "topics" -d 'Manage testing topics enrollment'
complete -c oma -n "__fish_use_subcommand" -f -a "mirror" -d 'Manage Mirrors enrollment'
complete -c oma -n "__fish_use_subcommand" -f -a "mirrors" -d 'Manage Mirrors enrollment'
complete -c oma -n "__fish_use_subcommand" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from install" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from install" -l install-dbg -d 'Install debug symbols for (a) package(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -l reinstall -d 'Reinstall package(s) by downloading a current copy from the repository'
complete -c oma -n "__fish_seen_subcommand_from install" -l install-recommends -d 'Install recommended packages(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -l install-suggests -d 'Install suggested package(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-install-recommends -d 'Do not install recommend package(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-install-suggests -d 'Do not install recommend package(s)'
complete -c oma -n "__fish_seen_subcommand_from install" -s f -l fix-broken -d 'Fix apt broken status'
complete -c oma -n "__fish_seen_subcommand_from install" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_seen_subcommand_from install" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_seen_subcommand_from install" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_seen_subcommand_from install" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_seen_subcommand_from install" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from install" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from install" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from install" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from install" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from install" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from upgrade" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from upgrade" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_seen_subcommand_from upgrade" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from download" -s p -l path -d 'The path where package(s) should be downloaded to' -r
complete -c oma -n "__fish_seen_subcommand_from download" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from download" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from download" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from download" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from download" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from download" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from download" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from download" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from remove" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from remove" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from remove" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_seen_subcommand_from remove" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_seen_subcommand_from remove" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_seen_subcommand_from remove" -l no-autoremove -d 'Do not remove package(s) without reverse dependencies'
complete -c oma -n "__fish_seen_subcommand_from remove" -l remove-config -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_seen_subcommand_from remove" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from remove" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from remove" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from remove" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from purge" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from purge" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from purge" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_seen_subcommand_from purge" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_seen_subcommand_from purge" -l no-autoremove -d 'Do not remove package(s) without reverse dependencies'
complete -c oma -n "__fish_seen_subcommand_from purge" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from purge" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from purge" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from purge" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from purge" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from purge" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from purge" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_seen_subcommand_from purge" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from refresh" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from refresh" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from refresh" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from refresh" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from refresh" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from refresh" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from refresh" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from refresh" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from refresh" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from show" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from show" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from show" -s a -l all -d 'Show information on all available version(s) of (a) package(s) from all repository(ies)'
complete -c oma -n "__fish_seen_subcommand_from show" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from show" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from show" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from show" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from search" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from search" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from search" -l no-pager -d 'Output result to stdout, not pager'
complete -c oma -n "__fish_seen_subcommand_from search" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from search" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from search" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from search" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from search" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from search" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from files" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from files" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from files" -l bin -d 'Search binary of package(s)'
complete -c oma -n "__fish_seen_subcommand_from files" -l no-pager -l println -d 'Set output mode as current println mode'
complete -c oma -n "__fish_seen_subcommand_from files" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from files" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from files" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from files" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from files" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from files" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from provides" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from provides" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from provides" -l no-pager -l println -d 'Set output mode as current println mode'
complete -c oma -n "__fish_seen_subcommand_from provides" -l bin -d 'Search binary of package(s)'
complete -c oma -n "__fish_seen_subcommand_from provides" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from provides" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from provides" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from provides" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from provides" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from provides" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from fix-broken" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from pick" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from pick" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from pick" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_seen_subcommand_from pick" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from pick" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from pick" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from pick" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from pick" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from pick" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from pick" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from pick" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mark" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mark" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mark" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from mark" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mark" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mark" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mark" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mark" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from list" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from list" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from list" -s a -l all -d 'List all available version(s) of (a) package(s) from all repository(ies)'
complete -c oma -n "__fish_seen_subcommand_from list" -s i -l installed -d 'List only package(s) currently installed on the system'
complete -c oma -n "__fish_seen_subcommand_from list" -s u -l upgradable -d 'List only package(s) with update(s) available'
complete -c oma -n "__fish_seen_subcommand_from list" -s m -l manually-installed -d 'List only package(s) with manually installed'
complete -c oma -n "__fish_seen_subcommand_from list" -l automatic -d 'List only package(s) with automatic installed'
complete -c oma -n "__fish_seen_subcommand_from list" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from list" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from list" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from list" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from list" -l autoremovable -d 'List only package(s) with autoremovable'
complete -c oma -n "__fish_seen_subcommand_from list" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from depends" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from depends" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from depends" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from depends" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from depends" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from depends" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from depends" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from depends" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from rdepends" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from rdepends" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from rdepends" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from rdepends" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from rdepends" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from rdepends" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from rdepends" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from rdepends" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from clean" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from clean" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from clean" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from clean" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from clean" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from clean" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from clean" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from clean" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from history" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from history" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from history" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from history" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from history" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from history" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from history" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from history" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from undo" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from undo" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from undo" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from undo" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from undo" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from undo" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from undo" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from undo" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from tui" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from tui" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from tui" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from tui" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from tui" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from tui" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from tui" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from tui" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from topics" -l opt-in -d 'Enroll in one or more topic(s), delimited by space' -r
complete -c oma -n "__fish_seen_subcommand_from topics" -l opt-out -d 'Withdraw from one or more topic(s) and rollback to stable versions, delimited by space' -r
complete -c oma -n "__fish_seen_subcommand_from topics" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from topics" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from topics" -l dry-run -d 'Run oma in “dry-run” mode'
complete -c oma -n "__fish_seen_subcommand_from topics" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from topics" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from topics" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from topics" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from topics" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from topics" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from sort-mirrors" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirror" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l set-fastest -d 'Also set fastest as mirror'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from speedtest" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_seen_subcommand_from mirror; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from sort-mirrors" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -lcomplete -c oma -n "__fish_seen_subcommand_from install" -l sysroot -d 'Set sysroot target directory' -r
complete -c oma -n "__fish_seen_subcommand_from mirrors" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -s o -l apt-options -r
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l set-fastest -d 'Also set fastest as mirror'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l debug -d 'Run oma with debug mode'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l no-color -d 'No color output to result'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -l no-check-dbus
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from speedtest" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_seen_subcommand_from mirrors; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "install" -d 'Install package(s) from the repository'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "upgrade" -d 'Upgrade packages installed on the system'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "download" -d 'Download package(s) from the repository'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "purge" -d 'purge (like apt purge) the specified package(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "refresh" -d 'Refresh repository metadata/catalog'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "show" -d 'Show information on the specified package(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "search" -d 'Search for package(s) available from the repository'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "files" -d 'List files in the specified package'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "provides" -d 'Search for package(s) that provide(s) certain patterns in a path'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "fix-broken" -d 'Resolve broken system dependencies in the system'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "pick" -d 'Install specific version of a package'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "mark" -d 'Mark status for one or multiple package(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "command-not-found"
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "list" -d 'List package(s) available from the repository'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "depends" -d 'Lists dependencies of one or multiple packages'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "rdepends" -d 'List reverse dependency(ies) for the specified package(s)'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "clean" -d 'Clear downloaded package cache'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "history" -d 'Show a history/log of package changes in the system'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "undo" -d 'Undo system changes operation'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "pkgnames"
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "mirror" -d ''
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "mirrors" -d ''
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "tui" -d 'Oma tui interface'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "topics" -d 'Manage testing topics enrollment'
complete -c oma -n "__fish_seen_subcommand_from help; and not __fish_seen_subcommand_from install; and not __fish_seen_subcommand_from upgrade; and not __fish_seen_subcommand_from download; and not __fish_seen_subcommand_from remove; and not __fish_seen_subcommand_from purge; and not __fish_seen_subcommand_from refresh; and not __fish_seen_subcommand_from show; and not __fish_seen_subcommand_from search; and not __fish_seen_subcommand_from files; and not __fish_seen_subcommand_from provides; and not __fish_seen_subcommand_from fix-broken; and not __fish_seen_subcommand_from pick; and not __fish_seen_subcommand_from mark; and not __fish_seen_subcommand_from command-not-found; and not __fish_seen_subcommand_from list; and not __fish_seen_subcommand_from depends; and not __fish_seen_subcommand_from rdepends; and not __fish_seen_subcommand_from clean; and not __fish_seen_subcommand_from history; and not __fish_seen_subcommand_from undo; and not __fish_seen_subcommand_from pkgnames; and not __fish_seen_subcommand_from mirror; and not __fish_seen_subcommand_from mirrors; and not __fish_seen_subcommand_from tui; and not __fish_seen_subcommand_from topics; and not __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
# Enhanced completions
complete -xc oma -n "__fish_seen_subcommand_from install" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from depends" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from download" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from list" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from files" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from pick" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from rdepends" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from search" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from show" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from remove" -a "(_oma_packages_installed)"
complete -xc oma -n "__fish_seen_subcommand_from purge" -a "(_oma_packages_installed)"

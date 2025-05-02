# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_oma_global_optspecs
	string join \n dry-run debug color= follow-terminal-color no-progress no-check-dbus v/version sysroot= apt-options= no-bell h/help
end

function __fish_oma_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_oma_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_oma_using_subcommand
	set -l cmd (__fish_oma_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

function _oma_packages
    oma pkgnames 2> /dev/null
end

function _oma_packages_installed
    oma pkgnames --installed 2> /dev/null
end

complete -c oma -n "__fish_oma_needs_command" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_needs_command" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_needs_command" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_needs_command" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_needs_command" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_needs_command" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_needs_command" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_needs_command" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_needs_command" -s v -l version -d 'Print version'
complete -c oma -n "__fish_oma_needs_command" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_needs_command" -f -a "install" -d 'Install package(s) from the repository'
complete -c oma -n "__fish_oma_needs_command" -f -a "add" -d 'Install package(s) from the repository'
complete -c oma -n "__fish_oma_needs_command" -f -a "upgrade" -d 'Upgrade packages installed on the system'
complete -c oma -n "__fish_oma_needs_command" -f -a "full-upgrade" -d 'Upgrade packages installed on the system'
complete -c oma -n "__fish_oma_needs_command" -f -a "download" -d 'Download package(s) from the repository'
complete -c oma -n "__fish_oma_needs_command" -f -a "remove" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "del" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "rm" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "autoremove" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "refresh" -d 'Refresh repository metadata/catalog'
complete -c oma -n "__fish_oma_needs_command" -f -a "show" -d 'Show information on the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "info" -d 'Show information on the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "search" -d 'Search for package(s) available from the repository'
complete -c oma -n "__fish_oma_needs_command" -f -a "files" -d 'List files in the specified package'
complete -c oma -n "__fish_oma_needs_command" -f -a "provides" -d 'Search for package(s) that provide(s) certain patterns in a path'
complete -c oma -n "__fish_oma_needs_command" -f -a "fix-broken" -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_needs_command" -f -a "pick" -d 'Install specific version of a package'
complete -c oma -n "__fish_oma_needs_command" -f -a "mark" -d 'Mark status for one or multiple package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "list" -d 'List package(s) available from the repository'
complete -c oma -n "__fish_oma_needs_command" -f -a "depends" -d 'Lists dependencies of one or multiple packages'
complete -c oma -n "__fish_oma_needs_command" -f -a "dep" -d 'Lists dependencies of one or multiple packages'
complete -c oma -n "__fish_oma_needs_command" -f -a "rdepends" -d 'List reverse dependency(ies) for the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "rdep" -d 'List reverse dependency(ies) for the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "clean" -d 'Clear downloaded package cache'
complete -c oma -n "__fish_oma_needs_command" -f -a "history" -d 'Show a history/log of package changes in the system'
complete -c oma -n "__fish_oma_needs_command" -f -a "log" -d 'Show a history/log of package changes in the system'
complete -c oma -n "__fish_oma_needs_command" -f -a "undo" -d 'Undo system changes operation'
complete -c oma -n "__fish_oma_needs_command" -f -a "tui" -d 'Oma tui interface'
complete -c oma -n "__fish_oma_needs_command" -f -a "version" -d 'Print version'
complete -c oma -n "__fish_oma_needs_command" -f -a "topics" -d 'Manage testing topics enrollment'
complete -c oma -n "__fish_oma_needs_command" -f -a "topic" -d 'Manage testing topics enrollment'
complete -c oma -n "__fish_oma_needs_command" -f -a "mirror" -d 'Manage Mirrors enrollment'
complete -c oma -n "__fish_oma_needs_command" -f -a "mirrors" -d 'Manage Mirrors enrollment'
complete -c oma -n "__fish_oma_needs_command" -f -a "purge" -d 'purge (like apt purge) the specified package(s)'
complete -c oma -n "__fish_oma_needs_command" -f -a "command-not-found" -d 'command-not-found'
complete -c oma -n "__fish_oma_needs_command" -f -a "pkgnames" -d 'Pkgnames (used for completion)'
complete -c oma -n "__fish_oma_needs_command" -f -a "generate" -d 'Generate shell completions and manpages'
complete -c oma -n "__fish_oma_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand install" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand install" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand install" -l install-recommends -d 'Install recommended package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l reinstall -d 'Reinstall package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l install-suggests -d 'Install suggested package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-install-recommends -d 'Do not install recommended package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-install-suggests -d 'Do not install suggested package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand install" -l install-dbg -d 'Install debug symbol package'
complete -c oma -n "__fish_oma_using_subcommand install" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand install" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand install" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand install" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand install" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand install" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand install" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand install" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand install" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand install" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand install" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand install" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand add" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand add" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand add" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand add" -l install-recommends -d 'Install recommended package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -l reinstall -d 'Reinstall package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -l install-suggests -d 'Install suggested package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-install-recommends -d 'Do not install recommended package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-install-suggests -d 'Do not install suggested package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand add" -l install-dbg -d 'Install debug symbol package'
complete -c oma -n "__fish_oma_using_subcommand add" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand add" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand add" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand add" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand add" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand add" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand add" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand add" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand add" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand add" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand add" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-fixbroken -d 'Do not fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand upgrade" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-fixbroken -d 'Do not fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand full-upgrade" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand download" -s p -l path -d 'The path where package(s) should be downloaded to' -r -F
complete -c oma -n "__fish_oma_using_subcommand download" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand download" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand download" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand download" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand download" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand download" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand download" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand download" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand download" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand download" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand remove" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand remove" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand remove" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand remove" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand remove" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand remove" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand remove" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand remove" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand remove" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand remove" -l no-autoremove -d 'Do not auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand remove" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand remove" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand remove" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand remove" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand remove" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand del" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand del" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand del" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand del" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand del" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand del" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand del" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand del" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand del" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand del" -l no-autoremove -d 'Do not auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand del" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand del" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand del" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand del" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand del" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand del" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand del" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand del" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand rm" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand rm" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand rm" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand rm" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand rm" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand rm" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand rm" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand rm" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand rm" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand rm" -l no-autoremove -d 'Do not auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand rm" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand rm" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand rm" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand rm" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand rm" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand rm" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand rm" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand rm" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand autoremove" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l no-autoremove -d 'Do not auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand autoremove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand refresh" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand refresh" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand refresh" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand refresh" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand refresh" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand show" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand show" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand show" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand show" -s a -l all -d 'how information on all available version(s) of (a) package(s) from all repository(ies)'
complete -c oma -n "__fish_oma_using_subcommand show" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand show" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand show" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand show" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand show" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand show" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand show" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand info" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand info" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand info" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand info" -s a -l all -d 'how information on all available version(s) of (a) package(s) from all repository(ies)'
complete -c oma -n "__fish_oma_using_subcommand info" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand info" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand info" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand info" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand info" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand info" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand info" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand info" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand search" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand search" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand search" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand search" -l no-pager -d 'Output result to stdout, not pager'
complete -c oma -n "__fish_oma_using_subcommand search" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand search" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand search" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand search" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand search" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand search" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand search" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand search" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand files" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand files" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand files" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand files" -l bin -d 'Search binary of package(s)'
complete -c oma -n "__fish_oma_using_subcommand files" -l no-pager -l println -d 'Output result to stdout, not pager'
complete -c oma -n "__fish_oma_using_subcommand files" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand files" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand files" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand files" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand files" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand files" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand files" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand provides" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand provides" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand provides" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand provides" -l bin -d 'Search binary of package(s)'
complete -c oma -n "__fish_oma_using_subcommand provides" -l no-pager -l println -d 'Output result to stdout, not pager'
complete -c oma -n "__fish_oma_using_subcommand provides" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand provides" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand provides" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand provides" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand provides" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand provides" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand provides" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand fix-broken" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand pick" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand pick" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand pick" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand pick" -s f -l fix-broken -d 'Fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand pick" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand pick" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand pick" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand pick" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand pick" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand pick" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand pick" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand pick" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand pick" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand pick" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand pick" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand pick" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand pick" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand pick" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand pick" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mark" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mark" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mark" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mark" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mark" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mark" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mark" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mark" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mark" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mark" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand list" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand list" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand list" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand list" -s a -l all -d 'List all available version(s) of (a) package(s) from all repository(ies)'
complete -c oma -n "__fish_oma_using_subcommand list" -s i -l installed -d 'List only package(s) currently installed on the system'
complete -c oma -n "__fish_oma_using_subcommand list" -s u -l upgradable -d 'List only package(s) with update(s) available'
complete -c oma -n "__fish_oma_using_subcommand list" -s m -l manually-installed -d 'List only package(s) with manually installed'
complete -c oma -n "__fish_oma_using_subcommand list" -l automatic -d 'List only package(s) with automatic installed'
complete -c oma -n "__fish_oma_using_subcommand list" -l autoremovable -d 'List only package(s) with autoremovable'
complete -c oma -n "__fish_oma_using_subcommand list" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand list" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand list" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand list" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand list" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand list" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand list" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand depends" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand depends" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand depends" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand depends" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand depends" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand depends" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand depends" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand depends" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand depends" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand depends" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand depends" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand dep" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand dep" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand dep" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand dep" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand dep" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand dep" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand dep" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand dep" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand dep" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand dep" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand dep" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand rdepends" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand rdep" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand rdep" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand rdep" -l json -d 'Set output format as JSON'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand rdep" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand rdep" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand clean" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand clean" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand clean" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand clean" -l keep-downloadable -d 'Keep downloadable packages'
complete -c oma -n "__fish_oma_using_subcommand clean" -l keep-downloadable-and-installed -d 'Keep downloadable and installed packages'
complete -c oma -n "__fish_oma_using_subcommand clean" -l keep-installed -d 'Keep installed packages'
complete -c oma -n "__fish_oma_using_subcommand clean" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand clean" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand clean" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand clean" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand clean" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand clean" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand clean" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand history" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand history" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand history" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand history" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand history" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand history" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand history" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand history" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand history" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand history" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand log" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand log" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand log" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand log" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand log" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand log" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand log" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand log" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand log" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand log" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand undo" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand undo" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand undo" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand undo" -l no-fixbroken -d 'Do not fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand undo" -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand undo" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand undo" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand undo" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand undo" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand undo" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand undo" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand undo" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand undo" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand undo" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand undo" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand undo" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand undo" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand tui" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand tui" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand tui" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand tui" -s f -l fix-broken -d 'Fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand tui" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand tui" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand tui" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand tui" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand tui" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand tui" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand tui" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand tui" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand tui" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand tui" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand tui" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand tui" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand tui" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand tui" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand version" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand version" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand version" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand version" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand version" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand version" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand version" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand version" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand version" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand version" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand topics" -l opt-in -d 'Enroll in one or more topic(s), delimited by space' -r
complete -c oma -n "__fish_oma_using_subcommand topics" -l opt-out -d 'Withdraw from one or more topic(s) and rollback to stable versions, delimited by space' -r
complete -c oma -n "__fish_oma_using_subcommand topics" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand topics" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand topics" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand topics" -l no-fixbroken -d 'Fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand topics" -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand topics" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand topics" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand topics" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand topics" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand topics" -l all -d 'Display all topics on list (include draft status topics)'
complete -c oma -n "__fish_oma_using_subcommand topics" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand topics" -l always-write-status -d 'Always write status to atm file and sources.list'
complete -c oma -n "__fish_oma_using_subcommand topics" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand topics" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand topics" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand topics" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand topics" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand topics" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand topics" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand topic" -l opt-in -d 'Enroll in one or more topic(s), delimited by space' -r
complete -c oma -n "__fish_oma_using_subcommand topic" -l opt-out -d 'Withdraw from one or more topic(s) and rollback to stable versions, delimited by space' -r
complete -c oma -n "__fish_oma_using_subcommand topic" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand topic" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand topic" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand topic" -l no-fixbroken -d 'Fix apt broken status'
complete -c oma -n "__fish_oma_using_subcommand topic" -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand topic" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand topic" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand topic" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand topic" -l autoremove -d 'Auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand topic" -l all -d 'Display all topics on list (include draft status topics)'
complete -c oma -n "__fish_oma_using_subcommand topic" -l remove-config -l purge -d 'Remove package(s) also remove configuration file(s), like apt purge'
complete -c oma -n "__fish_oma_using_subcommand topic" -l always-write-status -d 'Always write status to atm file and sources.list'
complete -c oma -n "__fish_oma_using_subcommand topic" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand topic" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand topic" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand topic" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand topic" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand topic" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand topic" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_oma_using_subcommand mirror; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from sort-mirrors" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l set-fastest -d 'Also set fastest as mirror'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from speedtest" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_oma_using_subcommand mirror; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and not __fish_seen_subcommand_from set add remove sort-mirrors speedtest help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from sort-mirrors" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l set-fastest -d 'Also set fastest as mirror'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l no-refresh-topics -d 'Do not refresh topics manifest.json file'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l no-refresh -d 'Do not refresh repository metadata'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from speedtest" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_oma_using_subcommand mirrors; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand purge" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand purge" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand purge" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand purge" -s y -l yes -d 'Bypass confirmation prompts'
complete -c oma -n "__fish_oma_using_subcommand purge" -s f -l fix-broken -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand purge" -s n -l no-fix-dpkg-status -d 'Do not fix dpkg broken status'
complete -c oma -n "__fish_oma_using_subcommand purge" -l force-unsafe-io -d 'Install package(s) without fsync(2)'
complete -c oma -n "__fish_oma_using_subcommand purge" -l force-yes -d 'Ignore repository and package dependency issues'
complete -c oma -n "__fish_oma_using_subcommand purge" -l force-confnew -d 'Replace configuration file(s) in the system those shipped in the package(s) to be installed (invokes `dpkg --force-confnew`)'
complete -c oma -n "__fish_oma_using_subcommand purge" -l no-autoremove -d 'Do not auto remove unnecessary package(s)'
complete -c oma -n "__fish_oma_using_subcommand purge" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand purge" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand purge" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand purge" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand purge" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand purge" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand purge" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand command-not-found" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l installed -d 'Only query installed package'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand pkgnames" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -f -a "shell"
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -f -a "man"
complete -c oma -n "__fish_oma_using_subcommand generate; and not __fish_seen_subcommand_from shell man help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from shell" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -s p -l path -r -F
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l color -d 'Represents the color preferences for program output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l sysroot -d 'Set sysroot target directory' -r -F
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l apt-options -d 'Set apt options' -r
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l dry-run -d 'Run oma in "dry-run" mode'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l debug -d 'Run oma with debug output'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l follow-terminal-color -d 'Output result with terminal theme color'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l no-progress -d 'Do not display progress bar'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l no-check-dbus -d 'Run oma do not check dbus'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -l no-bell -d 'Don\'t ring if oma completes the transaction'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from man" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from help" -f -a "shell"
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from help" -f -a "man"
complete -c oma -n "__fish_oma_using_subcommand generate; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "install" -d 'Install package(s) from the repository'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "upgrade" -d 'Upgrade packages installed on the system'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "download" -d 'Download package(s) from the repository'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "remove" -d 'Remove the specified package(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "refresh" -d 'Refresh repository metadata/catalog'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "show" -d 'Show information on the specified package(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "search" -d 'Search for package(s) available from the repository'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "files" -d 'List files in the specified package'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "provides" -d 'Search for package(s) that provide(s) certain patterns in a path'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "fix-broken" -d 'Resolve broken dependencies in the system'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "pick" -d 'Install specific version of a package'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "mark" -d 'Mark status for one or multiple package(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "list" -d 'List package(s) available from the repository'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "depends" -d 'Lists dependencies of one or multiple packages'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "rdepends" -d 'List reverse dependency(ies) for the specified package(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "clean" -d 'Clear downloaded package cache'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "history" -d 'Show a history/log of package changes in the system'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "undo" -d 'Undo system changes operation'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "tui" -d 'Oma tui interface'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "version" -d 'Print version'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "topics" -d 'Manage testing topics enrollment'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "mirror" -d 'Manage Mirrors enrollment'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "purge" -d 'purge (like apt purge) the specified package(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "command-not-found" -d 'command-not-found'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "pkgnames" -d 'Pkgnames (used for completion)'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "generate" -d 'Generate shell completions and manpages'
complete -c oma -n "__fish_oma_using_subcommand help; and not __fish_seen_subcommand_from install upgrade download remove refresh show search files provides fix-broken pick mark list depends rdepends clean history undo tui version topics mirror purge command-not-found pkgnames generate help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from mirror" -f -a "set" -d 'Set mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from mirror" -f -a "add" -d 'Add mirror(s) to sources.list'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from mirror" -f -a "remove" -d 'Remove mirror(s) from sources.list'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from mirror" -f -a "sort-mirrors" -d 'Sort mirror(s) order'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from mirror" -f -a "speedtest" -d 'Speedtest mirror(s)'
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from generate" -f -a "shell"
complete -c oma -n "__fish_oma_using_subcommand help; and __fish_seen_subcommand_from generate" -f -a "man"
# Enhanced completions
complete -xc oma -n "__fish_seen_subcommand_from install" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from add" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from upgrade" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from depends" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from dep" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from download" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from list" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from files" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from pick" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from rdepends" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from rdep" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from search" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from show" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from info" -a "(_oma_packages)"
complete -xc oma -n "__fish_seen_subcommand_from remove" -a "(_oma_packages_installed)"
complete -xc oma -n "__fish_seen_subcommand_from del" -a "(_oma_packages_installed)"
complete -xc oma -n "__fish_seen_subcommand_from rm" -a "(_oma_packages_installed)"
complete -xc oma -n "__fish_seen_subcommand_from purge" -a "(_oma_packages_installed)"

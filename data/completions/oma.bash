_oma_packages() {
    COMPREPLY+=($(oma pkgnames "$cur" 2> /dev/null))
}

_oma() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="oma"
                ;;
            oma,clean)
                cmd="oma__clean"
                ;;
            oma,command-not-found)
                cmd="oma__command__not__found"
                ;;
            oma,depends)
                cmd="oma__depends"
                ;;
            oma,download)
                cmd="oma__download"
                ;;
            oma,files)
                cmd="oma__files"
                ;;
            oma,fix-broken)
                cmd="oma__fix__broken"
                ;;
            oma,help)
                cmd="oma__help"
                ;;
            oma,history)
                cmd="oma__history"
                ;;
            oma,install)
                cmd="oma__install"
                ;;
            oma,list)
                cmd="oma__list"
                ;;
            oma,mark)
                cmd="oma__mark"
                ;;
            oma,mirror)
                cmd="oma__mirror"
                ;;
            oma,mirrors)
                cmd="oma__mirrors"
                ;;
            oma,pick)
                cmd="oma__pick"
                ;;
            oma,pkgnames)
                cmd="oma__pkgnames"
                ;;
            oma,provides)
                cmd="oma__provides"
                ;;
            oma,purge)
                cmd="oma__purge"
                ;;
            oma,rdepends)
                cmd="oma__rdepends"
                ;;
            oma,refresh)
                cmd="oma__refresh"
                ;;
            oma,remove)
                cmd="oma__remove"
                ;;
            oma,search)
                cmd="oma__search"
                ;;
            oma,show)
                cmd="oma__show"
                ;;
            oma,topics)
                cmd="oma__topics"
                ;;
            oma,tui)
                cmd="oma__tui"
                ;;
            oma,undo)
                cmd="oma__undo"
                ;;
            oma,upgrade)
                cmd="oma__upgrade"
                ;;
            oma__help,clean)
                cmd="oma__help__clean"
                ;;
            oma__help,depends)
                cmd="oma__help__depends"
                ;;
            oma__help,download)
                cmd="oma__help__download"
                ;;
            oma__help,files)
                cmd="oma__help__files"
                ;;
            oma__help,fix-broken)
                cmd="oma__help__fix__broken"
                ;;
            oma__help,help)
                cmd="oma__help__help"
                ;;
            oma__help,history)
                cmd="oma__help__history"
                ;;
            oma__help,install)
                cmd="oma__help__install"
                ;;
            oma__help,list)
                cmd="oma__help__list"
                ;;
            oma__help,mark)
                cmd="oma__help__mark"
                ;;
            oma__help,mirror)
                cmd="oma__help__mirror"
                ;;
            oma__help,mirrors)
                cmd="oma__help__mirrors"
                ;;
            oma__help,pick)
                cmd="oma__help__pick"
                ;;
            oma__help,provides)
                cmd="oma__help__provides"
                ;;
            oma__help,purge)
                cmd="oma__help__purge"
                ;;
            oma__help,rdepends)
                cmd="oma__help__rdepends"
                ;;
            oma__help,refresh)
                cmd="oma__help__refresh"
                ;;
            oma__help,remove)
                cmd="oma__help__remove"
                ;;
            oma__help,search)
                cmd="oma__help__search"
                ;;
            oma__help,show)
                cmd="oma__help__show"
                ;;
            oma__help,topics)
                cmd="oma__help__topics"
                ;;
            oma__help,tui)
                cmd="oma__help__tui"
                ;;
            oma__help,undo)
                cmd="oma__help__undo"
                ;;
            oma__help,upgrade)
                cmd="oma__help__upgrade"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        oma)
            opts="-v -h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --version --sysroot --help install upgrade download remove purge refresh show search files provides fix-broken pick mark list depends rdepends clean history undo mirror mirrors tui topics help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__clean)
            opts="-h --debug --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__depends)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help ."
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__download)
            opts="-p -h --path --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help ."
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -p)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__fix__broken)
            opts="-h --dry-run --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"

            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help)
            opts="install upgrade download remove purge refresh show search files provides fix-broken pick mark command-not-found list depends rdepends clean history undo pkgnames mirror mirrors tui topics help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__clean)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__depends)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__fix__broken)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__list__files)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__mark)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__pick)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__provides)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__rdepends)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__refresh)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__remove)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__topics)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__help__upgrade)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__history)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__install)
            opts="-f -y -h --install-dbg --reinstall --install-recommends --install-suggests --no-install-recommends --no-install-suggests --fix-broken --no-refresh --yes --force-yes --force-confnew --dry-run --dpkg-force-all --no-refresh-topics --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__list)
            opts="-a -i -u -m -h --all --installed --upgradable --manually-installed --automatic --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__files)
            opts="-h --bin --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help [package]"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__mark)
            opts="-h --dry-run --debug --no-color --follow-terminal-color --help hold unhold manual auto"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__pick)
            opts="-h --no-refresh --dry-run --dpkg-force-all --no-refresh-topics --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__mirror)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help [COMMANDS]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__mirrors)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help [COMMANDS]..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__provides)
            opts="-h --bin --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help [pattern]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__purge)
            opts="-y -h --yes --force-yes --no-autoremove --dry-run --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help ."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__rdepends)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help ."
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__refresh)
            opts="-h --no-refresh-topics --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__remove)
            opts="-y -h --yes --force-yes --no-autoremove --remove-config --dry-run --debug --no-color --no-progress --follow-terminal-color --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__search)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__show)
            opts="-a -h --all --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages "${cur}"
            return 0
            ;;
        oma__topics)
            opts="-h --opt-in --opt-out --dry-run --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --opt-in)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --opt-out)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__tui)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__undo)
            opts="-h --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        oma__upgrade)
            opts="-y -h --yes --force-yes --force-confnew --dry-run --dpkg-force-all --autoremove --no-refresh-topics --debug --no-color --follow-terminal-color --no-progress --no-check-dbus --sysroot --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _oma -o bashdefault -o default oma

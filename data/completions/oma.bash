_oma_packages() {
    COMPREPLY+=($(oma pkgnames "$cur" 2> /dev/null))
}

_oma_packages_installed() {
    COMPREPLY+=($(oma pkgnames --installed "$cur" 2> /dev/null))
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
            oma,add)
                cmd="oma__install"
                ;;
            oma,clean)
                cmd="oma__clean"
                ;;
            oma,command-not-found)
                cmd="oma__command__not__found"
                ;;
            oma,del)
                cmd="oma__remove"
                ;;
            oma,dep)
                cmd="oma__depends"
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
            oma,full-upgrade)
                cmd="oma__upgrade"
                ;;
            oma,generate)
                cmd="oma__generate"
                ;;
            oma,help)
                cmd="oma__help"
                ;;
            oma,history)
                cmd="oma__history"
                ;;
            oma,info)
                cmd="oma__show"
                ;;
            oma,install)
                cmd="oma__install"
                ;;
            oma,list)
                cmd="oma__list"
                ;;
            oma,log)
                cmd="oma__history"
                ;;
            oma,mark)
                cmd="oma__mark"
                ;;
            oma,mirror)
                cmd="oma__mirror"
                ;;
            oma,mirrors)
                cmd="oma__mirror"
                ;;
            oma,pick)
                cmd="oma__pick"
                ;;
            oma,pkgnames)
                cmd="oma__pkgnames"
                ;;
            oma,prvides)
                cmd="oma__prvides"
                ;;
            oma,purge)
                cmd="oma__purge"
                ;;
            oma,rdep)
                cmd="oma__rdepends"
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
            oma,rm)
                cmd="oma__remove"
                ;;
            oma,autoremove)
                cmd="oma__remove"
                ;;
            oma,search)
                cmd="oma__search"
                ;;
            oma,show)
                cmd="oma__show"
                ;;
            oma,topic)
                cmd="oma__topics"
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
            oma__generate,help)
                cmd="oma__generate__help"
                ;;
            oma__generate,man)
                cmd="oma__generate__man"
                ;;
            oma__generate,shell)
                cmd="oma__generate__shell"
                ;;
            oma__generate__help,help)
                cmd="oma__generate__help__help"
                ;;
            oma__generate__help,man)
                cmd="oma__generate__help__man"
                ;;
            oma__generate__help,shell)
                cmd="oma__generate__help__shell"
                ;;
            oma__help,clean)
                cmd="oma__help__clean"
                ;;
            oma__help,command-not-found)
                cmd="oma__help__command__not__found"
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
            oma__help,generate)
                cmd="oma__help__generate"
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
            oma__help,pick)
                cmd="oma__help__pick"
                ;;
            oma__help,pkgnames)
                cmd="oma__help__pkgnames"
                ;;
            oma__help,prvides)
                cmd="oma__help__prvides"
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
            oma__help__generate,man)
                cmd="oma__help__generate__man"
                ;;
            oma__help__generate,shell)
                cmd="oma__help__generate__shell"
                ;;
            oma__help__mirror,add)
                cmd="oma__help__mirror__add"
                ;;
            oma__help__mirror,remove)
                cmd="oma__help__mirror__remove"
                ;;
            oma__help__mirror,set)
                cmd="oma__help__mirror__set"
                ;;
            oma__help__mirror,sort-mirrors)
                cmd="oma__help__mirror__sort__mirrors"
                ;;
            oma__help__mirror,speedtest)
                cmd="oma__help__mirror__speedtest"
                ;;
            oma__mirror,add)
                cmd="oma__mirror__add"
                ;;
            oma__mirror,help)
                cmd="oma__mirror__help"
                ;;
            oma__mirror,remove)
                cmd="oma__mirror__remove"
                ;;
            oma__mirror,set)
                cmd="oma__mirror__set"
                ;;
            oma__mirror,sort-mirrors)
                cmd="oma__mirror__sort__mirrors"
                ;;
            oma__mirror,speedtest)
                cmd="oma__mirror__speedtest"
                ;;
            oma__mirror__help,add)
                cmd="oma__mirror__help__add"
                ;;
            oma__mirror__help,help)
                cmd="oma__mirror__help__help"
                ;;
            oma__mirror__help,remove)
                cmd="oma__mirror__help__remove"
                ;;
            oma__mirror__help,set)
                cmd="oma__mirror__help__set"
                ;;
            oma__mirror__help,sort-mirrors)
                cmd="oma__mirror__help__sort__mirrors"
                ;;
            oma__mirror__help,speedtest)
                cmd="oma__mirror__help__speedtest"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        oma)
            opts="-v -h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --version --sysroot --apt-options --help install add autoremove upgrade full-upgrade download remove del rm refresh show info search files prvides fix-broken pick mark list depends dep rdepends rdep clean history log undo tui topics topic mirror mirrors purge command-not-found pkgnames generate help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__clean)
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__command__not__found)
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <KEYWORD>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__depends)
            opts="-h --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__depends)
            opts="-h --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-p -h --path --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
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
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-h --bin --println --no-pager --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <PACKAGE>"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-h --force-unsafe-io --force-yes --force-confnew --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__generate)
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help shell man help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__generate__help)
            opts="shell man help"
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
        oma__generate__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__generate__help__man)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__generate__help__shell)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__generate__man)
            opts="-p -h --path --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
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
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__generate__shell)
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help bash elvish fish powershell zsh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="install upgrade download remove refresh show search files prvides fix-broken pick mark list depends rdepends clean history undo tui topics mirror purge command-not-found pkgnames generate help"
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
        oma__help__command__not__found)
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
        oma__help__files)
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
        oma__help__generate)
            opts="shell man"
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
        oma__help__generate__man)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__generate__shell)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__mirror)
            opts="set add remove sort-mirrors speedtest"
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
        oma__help__mirror__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__mirror__remove)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__mirror__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__mirror__sort__mirrors)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__mirror__speedtest)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__help__pkgnames)
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
        oma__help__prvides)
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
        oma__help__purge)
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
        oma__help__tui)
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
        oma__help__undo)
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
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__history)
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-y -f -h --install-recommends --reinstall --install-suggests --no-install-recommends --no-install-suggests --yes --install-dbg --fix-broken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__install)
            opts="-y -f -h --install-recommends --reinstall --install-suggests --no-install-recommends --no-install-suggests --yes --install-dbg --fix-broken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-a -i -u -m -a -a -h --all --installed --upgradable --manually-installed --automatic --autoremovable --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-h --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help hold unhold manual auto"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help set add remove sort-mirrors speedtest help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help set add remove sort-mirrors speedtest help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror__add)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <NAMES>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror__help)
            opts="set add remove sort-mirrors speedtest help"
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
        oma__mirror__help__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__help__remove)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__help__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__help__sort__mirrors)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__help__speedtest)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
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
        oma__mirror__remove)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <NAMES>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror__set)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <NAMES>..."
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror__sort__mirrors)
            opts="-h --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__mirror__speedtest)
            opts="-h --set-fastest --no-refresh-topics --no-refresh --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__pick)
            opts="-f -h --fix-broken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <PACKAGE>"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__pkgnames)
            opts="-h --installed --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help [KEYWORD]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__prvides)
            opts="-h --bin --println --no-pager --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <PATTERN>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-y -f -h --yes --fix-broken --force-unsafe-io --force-yes --force-confnew --no-autoremove --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
             _oma_packages_installed "${cur}"
            return 0
            ;;
        oma__rdepends)
            opts="-h --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__rdepends)
            opts="-h --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-h --no-refresh-topics --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-y -f -h --yes --fix-broken --force-unsafe-io --force-yes --force-confnew --no-autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            _oma_packages_installed "${cur}"
            return 0
            ;;
        oma__search)
            opts="-h --no-pager --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help <PATTERN>"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__show)
            opts="-a -h --all --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__show)
            opts="-a -h --all --json --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-n -h --opt-in --opt-out --no-fixbroken --force-unsafe-io --force-yes --force-confnew --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
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
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__topics)
            opts="-n -h --opt-in --opt-out --no-fixbroken --force-unsafe-io --force-yes --force-confnew --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
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
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-f -h --fix-broken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-n -h --no-fixbroken --force-unsafe-io --force-yes --force-confnew --autoremove --purge --remove-config --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
            opts="-n -y -h --no-fixbroken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --autoremove --purge --remove-config --yes --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
        oma__upgrade)
            opts="-n -y -h --no-fixbroken --force-unsafe-io --no-refresh --force-yes --force-confnew --no-refresh-topics --autoremove --purge --remove-config --yes --dry-run --debug --color --follow-terminal-color --no-progress --no-check-dbus --sysroot --apt-options --help "
            if [[ ${cur} == -* ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --sysroot)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --apt-options)
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
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _oma -o nosort -o bashdefault -o default oma
else
    complete -F _oma -o bashdefault -o default oma
fi

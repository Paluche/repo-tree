use std::{
    env::{args_os, current_dir, var_os},
    io::{Error, Write},
    process::exit,
    str::FromStr,
};

use clap::CommandFactory;
use clap_complete::{CompleteEnv, Shell};
use indoc::indoc;

use crate::cli::Args;

fn generate_sub_completer(
    name: &str,
    var: &str,
    completer: &str,
    args: &[&str],
    buf: &mut dyn Write,
) -> Result<(), Error> {
    let script = indoc! { r#"
    #compdef __NAME__
    function ___NAME__() {
        local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
        local _CLAP_IFS=$'\n'

        local words=("${words[1]}"__ARGS__ "${words[@]:2}")

        local completions=("${(@f)$( \
            _CLAP_IFS="$_CLAP_IFS" \
            _CLAP_COMPLETE_INDEX="$((_CLAP_COMPLETE_INDEX + __ARGS_NB__))" \
            __VAR__="zsh" \
            __COMPLETER__ -- "${words[@]}" 2>/dev/null \
        )}")

        if [[ -n $completions ]]; then
            local -a dirs=()
            local -a other=()
            local completion
            for completion in $completions; do
                local value="${completion%%:*}"
                if [[ "$value" == */ ]]; then
                    local dir_no_slash="${value%/}"
                    if [[ "$completion" == *:* ]]; then
                        local desc="${completion#*:}"
                        dirs+=("$dir_no_slash:$desc")
                    else
                        dirs+=("$dir_no_slash")
                    fi
                else
                    other+=("$completion")
                fi
            done
            [[ -n $dirs ]] && _describe 'values' dirs -S '/' -r '/'
            [[ -n $other ]] && _describe 'values' other
        fi
    }

    compdef ___NAME__ __NAME__"#
    }
    .replace("__NAME__", name)
    .replace("__COMPLETER__", completer)
    .replace("__VAR__", var)
    .replace(
        "__ARGS__",
        &args
            .iter()
            .flat_map(|a| {
                let mut chars = vec![' ', '"'];
                chars.extend(a.chars());
                chars.push('"');
                chars
            })
            .collect::<String>(),
    )
    .replace("__ARGS_NB__", &args.len().to_string());
    writeln!(buf, "{script}\n")
}

fn generate_rcd_zsh(
    var: &str,
    completer: &str,
    buf: &mut dyn Write,
) -> Result<(), Error> {
    let script = indoc! {r#"
        function _update_repo_root()
        {
            unset __GIT_REPO
            unset __JJ_REPO

            if res=("${(@f)$(rt repo root --print-type 2> /dev/null)}")
            then
                repo_root="${res[1]}"
                git_repo="${res[2]}"
                jj_repo="${res[3]}"

                if [ -n "${repo_root}" ] && [[ "${__REPO_ROOT}" != "${repo_root}" ]]
                then
                    export __PREVIOUS_REPO_ROOT=${__REPO_ROOT}
                    export __REPO_ROOT=${repo_root}
                fi

                if [ "${git_repo}" = "true" ]
                then
                    export __GIT_REPO=1
                fi

                if [ "${jj_repo}" = "true" ]
                then
                    export __JJ_REPO=1
                fi
            else
                unset __REPO_ROOT
            fi
        }

        precmd_functions+='_update_repo_root'

        function rcd()
        {
            if [[ "$@" == *-h* ]] || [[ "$@" == *--help* ]]
            then
                echo "Resolve the name of a repository and cd in its location"
                echo ""
                echo "Usage: rcd [REPO_ID]"
                echo ""
                echo "Arguments:"
                echo "  [REPO_ID]  Repository identifier to resolve, if not provided, and"
                echo "             fzf is installed, you will be asked using fzf to select"
                echo "             the repository. If you provide the - as repository"
                echo "             identifier then we will cd to the previous repository you"
                echo "             were in if there is one."
                echo ""
                echo "Note: For support of - as REPO_ID value, you need to have "
                echo "'typeset -ga precmd_functions' in your .zshrc."
                return 0
            fi

            if [ "${*}" = '-' ]
            then
                if [ -n "${__PREVIOUS_REPO_ROOT}" ]
                then
                    p=${__PREVIOUS_REPO_ROOT}
                    a=0
                else
                    echo "No previous repository"
                    return 1
                fi
            else
                p=$(rt resolve $@)
                a=$?
            fi

            if [[ $a -ne 0 ]]
            then
                echo $p
                return $a
            fi

            cd $p
        }
    "#};
    writeln!(buf, "{script}\n")?;
    generate_sub_completer("rcd", var, completer, &["resolve"], buf)
}

fn generate_repo_clone_zsh(
    var: &str,
    completer: &str,
    buf: &mut dyn Write,
) -> Result<(), Error> {
    let script = indoc! { r#"
        function repo-clone()
        {
            # Upon success, rt prints the location where the repository has been
            # cloned to.
            if output=$(rt clone "$@" | tee /dev/tty)
            then
                local last_line=$(echo "$output" | tail -n 1)
                if [ ! -d "${last_line}" ]
                then
                    echo "`rt clone` did not return new repository location as last line" > /dev/stderr
                fi
                if ! cd "$last_line"
                then
                    echo "Unable to go to \"${repo_root}\"" > /dev/stderr
                    return 1
                fi
            else
                return $?
            fi
        }
    "#};
    writeln!(buf, "{script}\n")?;
    generate_sub_completer("repo-clone", var, completer, &["clone"], buf)
}

fn generate_gtr_zsh(buf: &mut dyn Write) -> Result<(), Error> {
    let script = indoc! { r#"
        function gtr()
        {
            if repo_root=$(rt repo root --parent) && [ "${repo_root}" != "${PWD}" ]
            then
                if ! cd "${repo_root}"
                then
                    echo "Unable to go to \"${repo_root}\"" > /dev/stderr
                    return 1
                fi
            else
                echo -e "Nowhere to go" > /dev/stderr
                return 1
            fi
        }
    "#};
    writeln!(buf, "{script}\n")
}

fn generate_gs_zsh(
    var: &str,
    completer: &str,
    buf: &mut dyn Write,
) -> Result<(), Error> {
    writeln!(buf, "alias gs='rt git status'\n")?;
    generate_sub_completer("gs", var, completer, &["git", "status"], buf)
}

fn complete_utils_zsh(
    var: &str,
    completer: &str,
    buf: &mut dyn Write,
) -> Result<(), Error> {
    generate_rcd_zsh(var, completer, buf)?;
    generate_repo_clone_zsh(var, completer, buf)?;
    generate_gtr_zsh(buf)?;
    generate_gs_zsh(var, completer, buf)?;

    Ok(())
}

pub fn complete_utils(
    shell: Shell,
    var: &str,
    completer: &str,
) -> Result<(), Error> {
    let mut buf = Vec::new();

    // At this stage, we now the environment exists and is a valid string
    // representation of a Shell enum value.
    match shell {
        Shell::Zsh => complete_utils_zsh(var, completer, &mut buf)?,
        shell => {
            eprintln!("Utils generation not implemented yet for shell {shell}")
        }
    }
    std::io::stdout().write_all(&buf)
}

pub fn complete() {
    let bin = "rt";
    let var = "COMPLETE";
    let current_dir = current_dir().ok();
    let completer = CompleteEnv::with_factory(Args::command).bin(bin).var(var);
    let shell = var_os(var)
        .and_then(|v| v.to_str().and_then(|v| Shell::from_str(v).ok()));

    if completer
        .try_complete(args_os(), current_dir.as_deref())
        .unwrap_or_else(|e| e.exit())
    {
        exit(
            if var_os("_CLAP_COMPLETE_INDEX").is_some()
                || complete_utils(shell.unwrap(), var, bin)
                    .inspect_err(|e| {
                        eprintln!("Failed to generate {bin} utils: {e}")
                    })
                    .is_ok()
            {
                0
            } else {
                1
            },
        );
    }
}

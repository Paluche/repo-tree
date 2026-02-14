# Repo-tree: Repositories Manager

This repository provides a series of tool to organize the repositories you
clone within a single executable `rt`.

The repositories are organized in the Repository Tree root directory specified
by the `REPO_TREE_DIR` environment variable.

Features:

- Keep the Repo Tree organized, with feature to clean.
- Repository resolution, from name to an actual location. The util shell
  function `rcd` provides that feature.
  specifying

## Configuration

The tool has the following defaults values to associate an URL host to a folder
name:

```yaml
hosts:
  github.com:
    name: github
    repr: 
    repr_color: 15 # White
  gitlab.com:
    name: gitlab
    repr: 󰮠
    repr_color: 166 # Orange
  bitbucket.org:
    name: bitbucket
    repr: 
    repr_color: 12 # Blue
  git.kernel.org:
    name: kernel
    repr: 
    repr_color: 15 # White
```

You can override or extend this configuration with editing the configuration
file `${HOME}/.config/repo-tree/config.yml`, for example:

```yaml
hosts:
  bitbucket.org:
    name: atlassian
  my_company.gitlab.org:
    name: my_company
```

## Completion

We are using clap_complete with the unstable feature for dynamic completion.
This could change at any moment.

To enable it, configure your shell based on the following command lines:

Bash

```bash
echo "source <(COMPLETE=bash rt)" >> ~/.bashrc
```

Zsh

```
echo "source <(COMPLETE=zsh rt)" >> ~/.zshrc
```

Elvish

```bash
echo "eval (E:COMPLETE=elvish rt | slurp)" >> ~/.elvish/rc.elv
```

Fish

```bash
echo "COMPLETE=fish rt | source" >> ~/.config/fish/config.fish
```

Powershell

```bash
$env:COMPLETE = "powershell"
echo "rt | Out-String | Invoke-Expression" >> $PROFILE
Remove-Item Env:\COMPLETE
```

To disable completions, you can set COMPLETE= or COMPLETE=0

## Shell functions using rt

> I am using zsh so all function are zsh syntax and compatible.

### Jumping from one repository to another

The following ZSH function allows you to easily jump from one repository to
another, with auto-completion.


If no argument are provided, then `rt` will ask you which repository to jump to
interactively using `fzf` if installed.

```zsh
function rcd()
{
    if p=$(rt resolve $1)
    then
        cd $p
    fi
}

function _rcd() {
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
    local _CLAP_IFS=$'\n'

    # Insert "resolve" as the subcommand as rcd() arguments forwards its
    # arguments "rt resolve ..." so we need to adjust for that to obtain
    # the correct completions.
    # _CLAP_COMPLETE_INDEX is also adjusted accordingly below.
    local words=("${words[1]}" "resolve" "${words[@]:2}")

    local completions=("${(@f)$( \
        _CLAP_IFS="$_CLAP_IFS" \
        _CLAP_COMPLETE_INDEX="$((_CLAP_COMPLETE_INDEX + 1))" \
        COMPLETE="zsh" \
        rt -- "${words[@]}" 2>/dev/null \
    )}")

    if [[ -n $completions ]]; then
        _describe 'values' completions
    fi
}
compdef _rcd rcd
```

# Go To repository Root

This function allow you to jump to the root of the current repository. If you
are in a submodule, and already at the root it will jump to the root of the
repository containing the submodule.

```zsh
function gtr()
{
    if repo_root=$(rt repo root --parent) && [ "${repo_root}" != "${PWD}" ]
    then
        cd ${repo_root}
    else
        echo "Nowhere to go"
        return 1
    fi
}
```

# Clone a repository then jump at its root

```zsh
function repo_clone()
{
    # Upon success, rt prints the location where the repository has been
    # cloned to.
    output=$(rt repo clone "$@" | tee /dev/tty)

    if [[ $? -eq 0 ]]
    then
        local last_line=$(echo "$output" | tail -n 1)
        cd "$last_line" || return 1
    else
        return $?
    fi
}

function _repo_clone() {
    local _CLAP_COMPLETE_INDEX=$(expr $CURRENT - 1)
    local _CLAP_IFS=$'\n'

    # Insert "repo" a "clone" as the subcommand as repo_clone() arguments
    # forwards its arguments "rt repo lone..." so we need to adjust for that to
    # obtain # the correct completions.
    # _CLAP_COMPLETE_INDEX is also adjusted accordingly below.
    local words=("${words[1]}" "repo" "clone" "${words[@]:2}")

    local completions=("${(@f)$( \
        _CLAP_IFS="$_CLAP_IFS" \
        _CLAP_COMPLETE_INDEX="$((_CLAP_COMPLETE_INDEX + 2))" \
        COMPLETE="zsh" \
        rt -- "${words[@]}" 2>/dev/null \
    )}")

    if [[ -n $completions ]]; then
        _describe 'values' completions
    fi
}
compdef _repo_clone repo_clone
```

# Having a cron to periodically fetch all your repositories

The following script, will call `rt fetch` to have all your repositories
fetched. It will print some information as desktop notification.

It will be enhance to give you as summary the information if you need to check
some repositories to update them following the fetch.

```bash
#!/usr/bin/env bash

# Optional, sourcing a file which will enrich default environment variables.
source "${HOME}/<file containing env variables>"

XDG_RUNTIME_DIR=/run/user/$(id -u)
export XDG_RUNTIME_DIR

# Search for the ssh-agent socket to utilize the unlocked SSH key to use to
# fetch your repositories
if [ -z "${SSH_AUTH_SOCK}" ]
then
    SSH_AUTH_SOCK=$(find /tmp -path "/tmp/ssh-*/agent.*" -uid 1001 2> /dev/null)

    export SSH_AUTH_SOCK
fi

notify-send "Fetching repositories" --expire-time 20000

# Optional, check that SSH agent socket has been found.
if [ -z "${SSH_AUTH_SOCK}" ]
then
    notify-send "ssh agent not started" --expire-time 10000 --urgency critical
fi

# stderr is dumped in a temporary file.
# In quiet mode, stdout is limited to a summary log describing what has happen.
SUMMARY=$(rt fetch --quiet 2> "/tmp/rt_fetch.log")

notify-send "Fetching done" "${SUMMARY}" --expire-time 20000
```

Then use `crontab -e` to have the script executed once in a while.

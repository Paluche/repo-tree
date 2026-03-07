# Repo-tree: Repositories Manager

This repository provides a series of tool to organize the repositories you
clone within a single executable `rt`.

## Organized your repositories

The repositories are organized in the Repository Tree root directory specified
by the `REPO_TREE_DIR` environment variable.

The path where the repositories are stored is computed based on the remote URL
of the repository. The repositories are then organized by host.

Raw example of output of `rt tree`

```
/home/user/work
├── github
│   ├── Paluche/repo-tree
│   │       git@github.com:Paluche/repo-tree.git 󰊢
│   └── jj-vcs/jj
│           git@github.com:jj-vcs/jj.git 
├── gitlab/hpaluche/configort
│   │   git@gitlab.com:hpaluche/configort.git 󰊢
│   ├── home/dot_config/awesome/external_awesome-wm-widgets
│   │       c257e22ccbc1536de46e8ae83935d173926fd9ec
│   │       https://github.com/streetturtle/awesome-wm-widgets.git
│   └── home/dot_config/zsh/external_zsh-syntax-highlighting
│           1d85c692615a25fe2293bdd44b34c217d5d2bf04
│           https://github.com/zsh-users/zsh-syntax-highlighting.git
└── my-company
    ├── project-Z
    │   ├── system-A
    │   │   ├── sub-system-0
    │   │   │       git@my-company.com:project-Z/system-A/sub-system-0.git 󰊢
    │   │   └── sub-system-1
    │   │           git@my-company.com:project-Z/system-A/sub-system-1.git 󰊢
    │   └── system-B
    │       ├── sub-system-0
    │       │       git@my-company.com:project-Z/system-B/sub-system-0.git 󰊢
    │       └── sub-system-1
    │               git@my-company.com:project-Z/system-B/sub-system-1.git 󰊢
    └── ...
```

Main features:

- Keep the Repo Tree organized:
  - [x] Clone a repository directly at the correct location
        (`rt clone`).
  - [x] Repository resolution, from a name to an actual location
        (`rt resolve`).
  - [x] Check and re-organize the Repo Tree (`rt clean`).
- [x] Able to list all repositories within your configured directory (`rt
list`).
- Offer a way to interact globally with different types of repositories.
  - [x] [`git`](https://git-scm.com/) (`󰊢`)
  - [x] [`jujutsu`](https://docs.jj-vcs.dev) (``) (with `git` backend)
  - [ ] [`mercurial`](https://www.mercurial-scm.org/) (``)
  - [ ] [`subversion`](https://subversion.apache.org/) (``)
- Local interaction with the current repository, no matter its type:
  - [x] Get the root of the current repository (`rt repo root`).
  - [x] Generate a prompt (`rt prompt`). _Still some work todo_.
- [ ] Manage all submodule as `jj` workspaces. Reduce risk of desynchronization
      from one repository and the copies as submodules.
- [x] Fetch all your repositories (`rt fetch`)
- [ ] Get a status of your repositories. The main idea is to find out if there
      is user required action to be done on some repositories to keep them
      updated (`rt todo`)

> [!NOTE]
> This tool is my own at the moment, so I am prioritizing stuff for my usage.
> This means that the features will mostly resolve around interaction with:
>
> - zsh as shell
> - Nerdfont modified fonts
> - jujutsu as main vcs. My respositories are usually jj colocated with git.
>   But I'll favored interaction through jj by default.

## Configuration

The tool has the following defaults values to associate an URL host to a folder
name and prompt representation:

```yaml
hosts:
  github.com:
    name: github
    repr: 
    repr_color: white
  gitlab.com:
    name: gitlab
    repr: 󰮠
    repr_color: 166 # Orange
  git.kernel.org:
    name: kernel
    repr: 
    repr_color: white
  bitbucket.org:
    name: bitbucket
    repr: 
    repr_color: blue
  codeberg.org:
    name: codeberg
    repr: 
    repr_color: blue

local:
  name: local
  repr: 󰋊

vcs: jujutsu-git
```

The special characters comes from the [NerdFonts](https://www.nerdfonts.com/)
extra characters.

You can override or extend this configuration with editing the configuration
file `${HOME}/.config/repo-tree/config.yml`, for example:

```yaml
hosts:
  bitbucket.org:
    name: atlassian
    dir_name: bitbucket
  my_company.gitlab.org:
    name: my_company
    repr: 󰮠
    repr_color: 40 # Green
```

## Completion and utils

We are using [`clap_complete`](https://docs.rs/clap_complete/latest/clap_complete/index.html)
with the unstable feature for dynamic completion.

The completion will also generate some shell utils with their autocompletion:

- `rcd`: Jumping from one repository to another by referencing the repository
  name. Build on top of `rt resolve` command, use it as `rcd [REPO_ID]`.
  `REPO_ID` could either be:
  - Not specified, and if `fzf` is installed ask for the user to select the
    repository through `fzf` interface.
  - A valid repository identifier.
  - The value `-` which will let you go to the previous repository you were
    into if there is one. For that feature to work you need to enable the
    `precmd_functions`.
- `gtr`: Go To repository Root. This function allow you to jump to the root of
  the current repository. If you are in a submodule, and already at the root
  it will jump to the root of the repository containing the submodule.
- `repo-clone`: Clone a repository then `cd` to its root

To obtain the completion and utils, configure your shell based on the following
command lines:

Zsh

```bash
echo "source <(COMPLETE=zsh rt)" >> ~/.zshrc
```

> [!NOTE]
> For the following shells, the utils are not generated yet.
> Bash

```bash
echo "source <(COMPLETE=bash rt)" >> ~/.bashrc
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

To disable completions, you can set `COMPLETE=` or `COMPLETE=0`

### Having a cron to periodically fetch all your repositories

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

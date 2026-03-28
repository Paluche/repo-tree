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
- [x] Get a status of your repositories. The main idea is to find out if there
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

You can configure `rt` by editing the TOML configuration file. The file
will be searched at `${XDG_CONFIG_HOME}/repo-tree/config.toml` if the
environment variable `XDG_CONFIG_HOME` is set,
`${HOME}/.config/repo-tree/config.toml` otherwise.

### Configuring known hosts

In order to know how to organize the repositories, `rt` needs to know how. The
repositories are organized based on their remotes, for each remote host (e.g.
`github.com`) we need to know which directory name to use (e.g. `github`),
where all the associated repositories will be stored in.

```toml
[hosts."<URL>"]
name = '<NAME>'  # Pretty name for the host.
dir_name = '<DIR_NAME>'  # Name of the directory the host's repositories will
                         # be stored. Optional, defaults to the value set to
                         # 'name'.
repr = '<REPR>' # Host representation used in the prompt. Optional, defaults to
                # the value set to 'name'.
repr_color = 'white'  # Color to use to colorize the 'repr' value. Optional,
                      # defaults to no color.
```

> [!NOTE]
> `repr_color` can be specified as u8 (integer) or string.
>
> If u8 then it will be the ANSI color associated with that number.
>
> If string, the valid values are: "black", "red", "green", "yellow", "blue",
> "magenta" | "purple", "cyan", "white", "bright black" "bright red",
> "bright green", "bright yellow", "bright blue", "bright magenta",
> "bright cyan", "bright white".

The default configuration for the hosts is the following:

```toml
[hosts."github.com"]
name = 'github'
repr = ''
repr_color = 'white'

[hosts."gitlab.com"]
name = 'gitlab'
repr = '󰮠'
repr_color = 166 # Orange

[hosts."git.kernel.org"]
name = 'kernel'
repr = ''
repr_color = 'white'

[hosts."bitbucket.org"]
name = 'bitbucket'
repr = ''
repr_color = 'blue'

[hosts."codeberg.org"]
name = 'codeberg'
repr = ''
repr_color = 'blue'
```

The special `repr` characters comes from the
[NerdFonts](https://www.nerdfonts.com/) extra characters.

> [!NOTE]
> You can override these configuration. If you are doing so, you need to
> redefine the whole host, you cannot override specific elements.

### Configuring local repositories

For repositories which exists only locally, you can define too the directory
name as similar host configuration.

```toml
[local]  # Optional, see default configuration below.
name = '<NAME>'  # Pretty name for the local "host".
dir_name = '<DIR_NAME>'  # Name of the directory the local repositories will
                         # be stored. Optional, defaults to the value set to
                         # 'name'.
repr = '<REPR>' # Host representation used in the prompt. Optional, defaults to
                # the value set to 'name'.
repr_color = 'white'  # Color to use to colorize the 'repr' value. Optional,
                      # defaults to no color.
```

The default configuration for the local host is the following:

```toml
[local]
name = 'local'
repr = '󰋊'
```

### Configuring `rt resolve` command

Configure repository ID aliases for repository resolution.

```toml
[command.resolve.aliases]
'<alias_name>' = 'full/repository/id'
```

For instance the following configuration will allow you to do `rt resolve rt`
instead of `rt resolve repo-tree`.

```toml
[command.resolve.aliases]
rt = 'Paluche/repo-tree'
```

These aliases would obviously applies for utils using `rt resolve` such as
`rcd`.

### Configuring `rt clone` command

Configuring the default version control system to use to clone the repositories
using `rt clone`.

```toml
[command.clone]
vcs = '' # 'jujutsu', 'git' or 'jujutsu-git' (default)
```

### Configuring `rt todo` command

Configuring repositories to be ignored by the `rt todo` command.

```toml
[command.todo]
ignore = [  # List of repositories to ignore.
  'full/repository/id'
]
```

```
[command.clone]
vcs = 'jujutsu-git'
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

The script [rt_fetch_notify.sh](/scripts/rt_fetch_notify.sh), will call
`rt fetch` to have all your repositories fetched. Then `rt todo` to indicate
your repositories state.

Information are showed to the user using desktop notification (`notify-send`).

Install the script where you want (for instance
`~/.local/tools/rt_fetch_notify.sh`) make it executable, then use `crontab -e`
to have the script executed once in a while.

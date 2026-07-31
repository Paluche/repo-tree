# Repo-tree: Repositories Manager

This repository provides a series of tool to organize the repositories you
clone within a single executable `rt`.

## Organize your repositories

The repositories are organized in the Repository Tree root directory specified
by the `root` configuration or the `REPO_TREE_DIR` environment variable.

The path where the repositories are stored is computed based on the remote URL
of the repository. The repositories are then organized by host.

We have 4 "tree-spaces":

- `dev/` Contains active repositories.
- `archive/` Contains repositories marked as archived by the forge they a
  stored on.
- `agent/` Contains repositories workspaces where agents runs. This contains jj
  workspaces of the repository which should exists on the `dev` tree space.
- `local/` Repositories which has no remote associated and exists only locally.

Raw example of output of `rt tree`

```text
/home/user/work
├── dev
│   ├── github
│   │   ├── Paluche/repo-tree
│   │   │       git@github.com:Paluche/repo-tree.git 󰊢
│   │   └── jj-vcs/jj
│   │           git@github.com:jj-vcs/jj.git 
│   ├── gitlab/hpaluche/configort default@
│   │   │   git@gitlab.com:hpaluche/configort.git 󰊢
│   │   ├── home/dot_config/awesome/external_awesome-wm-widgets
│   │   │       c257e22ccbc1536de46e8ae83935d173926fd9ec
│   │   │       https://github.com/streetturtle/awesome-wm-widgets.git
│   │   └── home/dot_config/zsh/external_zsh-syntax-highlighting
│   │           1d85c692615a25fe2293bdd44b34c217d5d2bf04
│   │           https://github.com/zsh-users/zsh-syntax-highlighting.git
│   ├── my-company
│   │   ├── project-Z
│   │   │   ├── system-A
│   │   │   │   ├── sub-system-0
│   │   │   │   │       git@my-company.com:project-Z/system-A/sub-system-0.git 󰊢
│   │   │   │   └── sub-system-1
│   │   │   │           git@my-company.com:project-Z/system-A/sub-system-1.git 󰊢
│   │   │   └── system-B
│   │   │       ├── sub-system-0
│   │   │       │       git@my-company.com:project-Z/system-B/sub-system-0.git 󰊢
│   │   │       └── sub-system-1
│   │   │               git@my-company.com:project-Z/system-B/sub-system-1.git 󰊢
│   │   └── ...
│   └── ...
├── archive
│   ├── github
│   │   ├── Paluche/Lawsty
│   │   │         git@github.com:Paluche/Lawsty.git 󰊢
│   │   └── ...
│   └── ...
├── agent
│   ├── github
│   │   └── Paluche/repo-tree agent@
│   └── gitlab/hpaluche/configort agent@
└── local
    └── ... # Repositories which has no remote configured.
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
> - jujutsu as main vcs. My repositories are usually jj colocated with Git.
>   But I'll favored interaction through jj by default.

## Configuration

You can configure `rt` by editing the TOML configuration file. The file
will be searched at `${XDG_CONFIG_HOME}/repo-tree/config.toml` if the
environment variable `XDG_CONFIG_HOME` is set,
`${HOME}/.config/repo-tree/config.toml` otherwise.

### Color configuration

On the following configuration, some fields are colored text (`local.repr` for
instance). You specify the `text` and the `color` to color the text with.
You have several option to specify the color to use.
to color the text with.

- `string`: The valid values being: `"black"` `"red"`, `"green"`, `"yellow"`,
  `"blue"`, `"magenta"`, `"purple"` (equivalent to `"magenta"`), `"cyan"`,
  `"white"`, `"bright black"`, `"bright red"`, `"bright green"`,
  `"bright yellow"`, `"bright blue"`, `"bright magenta"`, `"bright cyan"`,
  `"bright white"`.
- `int`: Value between 0 and 255 included, corresponding to the
  [ANSI color](https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit) associated
  with that number.
- `[int, int, int]`: Corresponding to the three color components, respectively
  red, green and blue, of the color you want. Each component value being
  between 0 and 255 included.

### Tree categories

The configuration allows you to configure the tree categories. Those categories
corresponds to directories into which the repositories are stored and they
categorize them. We currently have two type of categories:

- The tree-spaces directories
- The host directories

The configuration for tree categories will always include the following keys:

```toml
name = '<NAME>'  # Pretty name for the category.
dir_name = '<DIR_NAME>'  # Name to use for the directory that stores the
                         # categorized repositories will. Optional, defaults to
                         # the value set to 'name'.
repr = {
  text = '<REPR>', # Host representation used in the prompt. Optional, defaults
                   # to the value set to 'name'.
  color = <COLOR>  # Color to use to colorize the 'repr.text' value. Optional,
                   # defaults to no color. See Color Configuration chapter
                   # above.
}
```

In the following configuration examples, when we use `<CATEGORY_KEYS>` you can
consider that you have the key describe above, this is done to avoid copy paste.

> [!NOTE]
> You can override these configuration. If you are doing so, note that you will
> not inherit bits of the default category configuration. For instance if you
> want to set the `dir_name` and inherit the rest of the values, you will need
> to specify the `name` and `repr` values.

### Configuring tree-spaces

#### Dev

For default tree-space for the repositories which have a remote configured.

```toml
[tree.dev]  # Optional, see default configuration below.
<CATEGORY_KEYS>
```

The default configuration for the local tree-space is the following:

```toml
[tree.dev]
name = 'dev'
repr = { text = '', color = 'blue' }
```

#### Local

The tree-space where the repositories, which has no remote configured (therefore
exists only locally), are stored.

```toml
[tree.local]  # Optional, see default configuration below.
<CATEGORY_KEYS>
```

The default configuration for the local tree-space is the following:

```toml
[tree.local]
name = 'local'
repr = { text = '󰋊', color = 'white' }
```

### Configuring known hosts

In order to know how to organize the repositories, `rt` needs to know how. The
repositories are organized based on their remotes, for each remote host (e.g.
`github.com`) we need to know which directory name to use (e.g. `github`),
where all the associated repositories will be stored in.

```toml
[host."<URL>"]
<CATEGORY_KEYS>
```

The default configuration for the hosts is the following:

```toml
[host."github.com"]
name = 'github'
repr = { text = '', color = 'white' }

[host."gitlab.com"]
name = 'gitlab'
repr = { text = '󰮠', color = 166 } # Orange

[host."git.kernel.org"]
name = 'kernel'
repr = { text = '', color = 'white' }

[host."bitbucket.org"]
name = 'bitbucket'
repr = { text = '', color = 'blue' }

[host."codeberg.org"]
name = 'codeberg'
repr = { text = '',  color = 'blue' }
```

The special `repr` characters comes from the
[NerdFonts](https://www.nerdfonts.com/) extra characters.

> [!NOTE]
> You can override these configuration. If you are doing so, you need to
> redefine the whole host, you cannot override specific elements.

### Configuring management of unknown remote

A remote is considered unknown if you did not provide the associated
configuration for the host remote URL.

```toml
[unknown_host.repr]
text = '<REPR>' # Representation used in the prompt for unknown hosts.
color = <COLOR>  # Color to use to colorize the 'repr.text' value. Optional,
                 # defaults to no color. See Color Configuration chapter above.
```

The default configuration for the unknown hosts is the following:

```toml
[unknown_host.repr]
text = ''
color: 'red'
```

### Configuring the prompt output

You can customize the prompt produce by `rt repo prompt`, you already
encountered some of those customization through the host representations or
`repr`.

The prompt has the following fields:

- `vcs` representation: Which version control system, is available for the
  current repository.
- Repository remote representation. Representation for the remote host of the
  repository. Corresponding to the configuration `host.'<URL>'.repr` of the
  remote host `URL` associated to the repository.
- Full repository ID / name.
- If the repository is a Git repository:
  - Ongoing Git operations, if any. Display when you are in a middle of an
    operation such as `rebase`, `am`, `cherry-pick`, `bisect`, `merge` or
    `revert`. This happens when Git asks you to resolve a conflict. You should
    only resolve the conflict, then execute `git <operation> --continue`.
  - If the repository is not collocated with `jj`:
    - List of branches which your `HEAD` is currently at.
    - List of tags which your `HEAD` is currently at.
    - Upstream information. If you are at a branch which has an upstream, what
      is your state relative to that upstream. Do need to push / pull? Did you
      diverged? Has the upstream branch been deleted? Otherwise show a
      representation telling you that you are detached from any branch, or that
      there is no upstream set yet.
    - stage / unstaged status.
  - Status of the repository's submodules.
  - Status of the repository's stash. Show a flag if you have pending stashes.
- If the repository is a Jujutsu repository:
  - List of bookmarks, from the following categories. Each bookmark will be
    suffixed a `*` if there are some changes to that bookmark that must be
    pushed to the remote.
    - Bookmarks set the `parent` of the current commit you are editing (`@-` in
      revset language).
    - Bookmarks set on the `current` commit you are editing (`@` in revset
      language).
    - Bookmarks on which you have an impact on by editing the current commit,
      said otherwise, the bookmarks set on any of the `descendants` commits of
      the current one (`@::` in revset language).
  - List of `deleted` bookmarks. Those are bookmarks which you just deleted the
    local one and then they are pending on an action, either you push and then
    delete the branch on the remote to, or you forget about this deleted
    bookmark.
  - List of tags, from the same `parent`, `current` and `descendants`
    categories as the bookmarks. Local tags, tags tracking a remote one
    (suffixed with a `*` when they differ) and remote only tags are colored
    differently.
  - List of `deleted` tags, pending to be pushed or forgotten, as for the
    bookmarks.
  - Show if there is any commits with conflicts to resolve.

Each of this fields are display in the order as listed above, if there is
nothing to show for a field, nothing is shown. We limit the output length for
the branches and tags too avoid too noisy prompt, there is already a lot of
information potentially displayed.

The prompt output starts with a `prefix`, and each fields are separated from
the previous one by a `separator`.

Here are the following configurations available and their default value.

```toml
[prompt]
prefix = { text = '┣━┫', color = 'cyan' }
separator = { text = '|', color = 'cyan' }

# Configure representation for the different version control systems supported.
[prompt.vcs]
git = { text = "󰊢" , color = 166 }
jj = { text = "" , color = "blue" }

# Configure Git prompt output.
[prompt.git]
ongoing_operations = { prefix = "⛏", separator = "🞍", color = "red" }
branches = { prefix = "󰫍", separator = "🞍", color = "blue" }
tags = { prefix = "", separator = "🞍", color = "yellow" }
upstream = {
  gone = "",
  up_to_date = "",
  ahead = "",
  behind = "",
  diverged = "",
  local = "",
  detached = "",
  color = 208
}
stash = { text = "", color = "white" }

# Configure Jujutsu prompt output.
[prompt.jj.bookmark]
parent = { prefix = "󰫍", separator = "🞍", color = "yellow" }
current = { prefix = "󰫍", separator = "🞍", color = "bright blue" }
descendants = { prefix = "󰫎", separator = "🞍", color = "bright blue" }
deleted = { prefix = "󰠙", separator = "🞍", color = 166 }

[prompt.jj.tag]
local = "bright yellow"
tracked = "yellow"
untracked = { name = "yellow", remote = 94 }
parent = { prefix = "", separator = "🞍", color = "yellow" }
current = { prefix = "", separator = "🞍", color = "bright blue" }
descendants = { prefix = "", separator = "🞍", color = "bright blue" }
deleted = { prefix = "󱈠", separator = "🞍", color = 166 }

[prompt.jj]
conflict = { text = "󰝧", color = "bright red" }
```

### Configuring repositories to ignore

You can allow some repositories to leave outside the repo tree in specific
location. You cannot ignore repositories within the repo tree.

To configure how to ignore repositories, specify a list of glob matching
patterns which would match repository location to ignore.

By default the list is the following:

```toml
[repository]
ignore = [
  '/tmp/**',   # Ignore any repository cloned in /tmp.
  '**/.*/**',  # Ignore any repository cloned within a hidden directory.
]
```

You can either override this list, or extend it by using the
`repository.extend_ignore` configuration.

At the moment "ignoring" means that no error message will be displayed when
executing a command for an ignored repository outside the repo tree.

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
  'full/repository/id',
]
```

```
[command.clone]
default_vcs = 'jujutsu-git'
```

### Configuring identities

`rt` can manage the identity you use in each repository: the name and email
authoring the commits and how the commits are signed. You define a `default`
identity, used in all the repositories, and other identities overriding part
of it in some repositories. When no identity is configured, `rt` leaves the
identity of your repositories alone.

```toml
[identities.default] # Mandatory as soon as one identity is defined.
name = 'Your Name'
email = 'you@personal.org'
signing = { backend = 'gpg', key = '26B104C8AD4182E5', on = 'push' }

[identities.work] # Only the fields to override.
email = 'you@company.com'
scopes = [
  { host = 'github.com', path = 'company/' }, # All the company repositories.
  { host = 'gitlab.company.com' },            # The whole host.
]

[identities.legacy]
signing = 'none'
scopes = [{ host = 'github.com', path = 'company/legacy' }] # One repository.
```

- `scopes`: the repositories an identity applies to. `host` is the host of the
  remote URL. `path` is optional: ending with `/` it matches all the
  repositories whose name starts with it, otherwise only the repository with
  exactly that name. The `default` identity has no scopes, and a scope can only
  be used by one identity.
- `name`, `email`: the author of the commits. The `default` identity must
  define both.
- `signing`: either `'none'` not to sign the commits, or the signing `backend`
  (`'gpg'`, `'ssh'` or `'gpgsm'`), the `key` to sign with and when to sign:
  `on = 'commit'` (default) or `on = 'push'`. Only Jujutsu can sign on push, Git
  signs at commit time. Without `signing`, `rt` leaves the signing configuration
  of the repositories alone.

The identity of a repository starts from the `default` identity. The identities
whose scopes match the repository are then applied from the least to the most
specific scope: host, then directory (longest last), then repository. Here a
repository in `github.com/company/legacy` is authored by
`Your Name <you@company.com>` and its commits are not signed.

`rt identity show` displays the identity of the current repository, the
identities it is made of and how the repository configuration differs from it.
`rt identity apply` writes the identity in the configuration of the repository
(`--all` for all the repositories of the repo tree): `user.name`, `user.email`
and the signing configuration of Jujutsu, and of Git for Git and colocated
repositories. Only the configuration of the repository itself is written, so
your user configuration stays the fallback for the repositories outside the repo
tree.

`rt clone` sets the identity of the repositories it clones, and refreshing the
repositories cache (`rt refresh-cache`, `rt insert`, ...) sets it in the
repositories whose configuration differs, with a hint listing the changes. When
you edit the configuration file, the next `rt` command using the repositories
cache (`rcd`, `rt list`, ...) sets the new identities.

#### Tokens and secrets

Identities also tell which tokens to use, per host: `git` to fetch and push over
HTTPS, `api` to access the forge API. The configuration only holds identifiers;
the tokens are in `secrets.toml`, next to `config.toml`, which should only be
readable by you (`chmod 600`, `rt` warns otherwise). A token is only ever used
for its host.

```toml
# config.toml
[identities.default]
tokens.'github.com' = { git = 'github-personal', api = 'github-personal' }

[identities.work]
tokens.'github.com' = { git = 'github-work' } # Still the personal API token.
```

```toml
# secrets.toml
[github-personal]
token = 'github_pat_...'

[github-work]
username = 'you-at-company' # Optional for GitHub, GitLab and Bitbucket.
token = 'github_pat_...'
```

`rt credential` is a Git credential helper giving Git these tokens. To have Git
use it, add to your global Git configuration, for each host with a Git token:

```gitconfig
[credential "https://github.com"]
	helper =
	helper = !rt credential
	useHttpPath = true
```

The empty `helper` drops the other credential helpers for that host, so that
only `rt` answers. `useHttpPath` makes Git give the path of the repository to
`rt`, which then knows the identity to use, clones included. This also applies
to Jujutsu, which runs Git to fetch and push. `rt` must be in the `PATH` of the
programs running Git.

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

Bash

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

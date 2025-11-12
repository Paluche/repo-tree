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
  github.com: github
  gitlab.com: gitlab
  bitbucket.org: bitbucket
  git.kernel.org: kernel
```

You can override or extend this configuration with editing the configuration
file `${HOME}/.config/repo-tree/config.yml`, for example:

```yaml
hosts:
  bitbucket.org: atlassian
  my_company.gitlab.org: my_company
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

# Workspace Manager

This repository provides a series of tool to organize your repositories you
clone. It basically revolves around the path resolution, from a git URL to a
certain path in your `WORKSPACE_DIR`.

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

You can overide or extend this configuration with editing the configuration
file `${HOME}/.config/workspace/config.yml`, for example:

```yaml
hosts:
   bitbucket: bitbucket
   my_company.gitlab.org: my_company
```

## Completion

We are using clap_complete with the unstable feature for dynamic completion.
This could change at any moment.

To enable it, configure your shell based on the following command lines:

Bash

```bash
echo "source <(COMPLETE=bash workspace)" >> ~/.bashrc
```

Zsh

```
echo "source <(COMPLETE=zsh workspace)" >> ~/.zshrc
```

Elvish

``` bash
echo "eval (E:COMPLETE=elvish workspace | slurp)" >> ~/.elvish/rc.elv
```

Fish

``` bash
echo "COMPLETE=fish workspace | source" >> ~/.config/fish/config.fish
```

Powershell

``` bash
$env:COMPLETE = "powershell"
echo "workspace | Out-String | Invoke-Expression" >> $PROFILE
Remove-Item Env:\COMPLETE
```

To disable completions, you can set COMPLETE= or COMPLETE=0

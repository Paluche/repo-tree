# Workspace Manager

This repository provides a series of tool to organize your repositories you
clone. It basically revolves around the path resolution, from a git URL to a
certain path in your `WORK_DIR`.

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

# Workflow
This page revolves around your workflow you'll have when working with pusta. This includes the most used commands, those for installing and removing modules and repositories. From experience, the most used command will be the update command.

> Note that Pusta is designed to be used in userspace (although it can totally be used to modify things outside of it). Because of that **never run Pusta as root**. Pusta will automatically elevate to root privileges if needed.

## Repositories
When working with repositories, you'll frequently (or at least once) have to add and possibly remove a repository to and from Pusta. This is generally done over the subcommand `pusta source`, since repositories are the sources of your modules.

### Adding
To add a repository, use the `add` subcommand. By default, this will add the current directory as a pusta repository.

```shell
pusta source add (your-repository) -a (alias) 
```
- `your-repository` (optional) - Specify the path of the repository as a relative or absolute path, overriding the current directory.
- `-a alias` (optional) - Override the repository alias that is defined by the directory or the repository itself. This is useful if using repositories from other users.

### Removing
To remove a repository, use the opposite, the `remove` subcommand. To remove a repository, you'll need to specify its alias rather than the directory.

```shell
pusta source remove [alias]
```

- `alias` - Alias of the repository to remove. Keep in mind that most of the time the alias does not correspond to the directory name of the repository.

## Modules
The commands you'll use the most often will be to interact with your modules. Because of that, each action has a dedicated subcommand. Most of these commands operate using module qualifiers, supporting both normal and unique qualifiers. Find more about what the difference is on the [Modules](../structure/module.md#qualifiers) page.

### Installing
To install a module, use the `install` command.

```shell
pusta install [module]
```
- `module` - Specify which module to install by providing a qualifier. It currently only supports taking one single qualifier.

### Removing
To remove a module, run the `remove` command.

```shell
pusta remove [module]
```
- `module` - Specify which module to install by providing a qualifier. It currently also only supports taking one single qualifier.

### Updating
To update modules, use the `update` command. Other than the previous commands, this command updates all modules by default. You can specify a single module though, if you want.

```shell
pusta update (module)
```

- `module` (optional) - Specify a specific module you want to update, otherwise, every module will get updated.

## Examples
Here are a few example usages of the commands explained here:
```shell
# add a repository at ~/.dotfiles with its default alias
pusta source add ~/.dotfiles

# remove a repository under the alias virt
pusta source remove virt

# install the module firefox from the virt repository
pusta install virt/firefox

# remove the module again
pusta remove firefox

# update all modules
pusta update
```





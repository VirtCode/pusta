# Repositories
Handling each module on its own would be very tedious. That's why they're arranged together in repositories. The repository is the outermost important construct of your pusta configuration. Inside it, you'll have all your modules in dedicated directories. To use any of these modules with pusta, you only need to add your repository to pusta, and pusta will have access to every module inside this repository.

Pusta supports having added multiple repositories, so you can install modules from all of them. Usually, a repository represents one source of configuration. These sources can be from you, like if you have a public dotfiles repository, and some configs which you'd rather keep private. Or different repositories can be from entirely different sources, like from other users all over the internet. Keep in mind though, that repositories from different sources are not guaranteed to be compatible.

> Although the name almost implies it, Pusta repositories does have nothing to do with repositories from version control systems. Pusta leaves it up to you whether you want to use a VCS for managing your dotfiles, because one can definitely be used on a pusta repository. 

## Definition
A repository is really simple, since it is just a directory. So to create a repository, just create a directory. This directory will be the base directory of your repository, and will directly contain all module directories.

Repositories are identified by an alias. This alias is by default determined by the name of the directory. However, this is generally discouraged, you should set an alias explicitly in the repository file, we'll take a look at after this paragraph. This way, your repository alias is the same on every system, be it one of yours or if another user is using your repository. If you encounter a repository which you don't want to edit and doesn't hava an alias, you can additionally set one when adding it to pusta. 

Contrary to modules, **repository aliases are static and fixed**. This means that they are stored independently once you add a repository to pusta. So you can't change the alias pusta uses for your repository after having added it. This is important to keep in mind if you want to change the alias of your repository. Because you can still do so by removing and adding the repository again. 

For this directory to actually qualify as a Pusta repository, it still needs to have one file, the `pusta.yml` file. Inside this file, metadata and other information about your specific repository will be stored. Look at the [Properties](#properties) section for information about the properties of this file. Conveniently, this file also serves as an easy indicator to tell whether dotfiles are managed with pusta, if you find a `pusta.yml` file, they probably are.

As already mentioned, beside this file you'll have your modules, with each module having its own directory. Go to the [Modules](module.md) page to see how a module is exactly defined. Besides that, you are allowed to have arbitrary files and directories inside your repository directory, they'll just be ignored by pusta.

This is how an example repository directory might look like:
```
my-repository/
    my-module-1/
    my-module-2/
    my-module-3/
    pusta.yml
    README.md
```
## Properties
The repository config file currently only holds one property:

```yml
# pusta.yml

alias: [string] # optional - override the alias of the repository
variables: # optional - repository specific variables 
  ...
```

- `alias` (optional) - Overrides the alias that is otherwise determined from the directory name. Using this property is strongly encourage, as it will avoid confusion and increase the portability of your repository.
- `variables` (optional) - Repository specific variables structure, provided as a normal YAML structure. See [Variables](variables.md#loading) for more information.

By the way, for more information about how the repository alias affects its modules, read the [Qualifiers](module.md#qualifiers) section on the Modules page.

## Workflow
Adding and removing repositories is also quite easy to pull off. Just run one of the sub commands of `pusta source` to manage your module sources, which are your repositories.

Have a look at the [Repositories](../working/workflow.md#repositories) section over on the Workflow page for more fine-grained instructions.

## Example
This very sophisticated example specifies an alias and a variable for your repository:
```yml
# pusta.yml

alias: virt

variables:
  color: red
```
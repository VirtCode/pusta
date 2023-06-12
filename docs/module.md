# Modules
In Pusta, your entire configuration is split up into Modules. 

_But what is a module?_ A module is an independent part of your configuration that is scoped to only one component of your system. Such a component could be a software program, like a terminal emulator or a code editor, it could be an important component like a wayland compositor or a display manager, or it could even be something more crucial like your sound backend or your bootloader, or any independent component of your system you can imagine. So as you can see, you can and should have a module for everything. 

_And what does a module contain?_ Obviously, a module contains all the configuration for the system component it takes care of. But that's not all. Usually, a module also installs its component over the system package manager or something similar. It also contains scripts and commands that need to be executed for your component to be enabled or for it to run correctly. In summary, a module tries to be as exhaustive as possible about all installation, setup and configuration that is necessary to get that component to the state you'll need it.

_So what's the benefit of all that?_ Well, by modularizing your entire configuration, you'll gain a ton of flexibility. If you have two systems with different needs but want to share parts of your configuration, you can easily install different modules on either machines, while having everything in the same place. Even if you have only one system, it becomes very easy to swap out one of your system components for another and still have the possibility to revert later. It also allows you to borrow configuration from someone else, just download their repository and install the modules you want. In addition, since these modules are exhaustive, your entire system is in one place and setting up a new one becomes very easy.

## Definition
To define a new module, head into the base directory of a [repository](repository) and create a new directory. This directory will be the base directory for that module. The name of the directory is, unless otherwise specified, also the alias for the module. So the directory should ideally be the name of the component it is scoped to, or something similar.

Inside that directory, create the module file `module.yml`. Inside this file, the properties and behaviour of that module is defined. This file is what actually defines that module, see its properties under the [Properties section](#properties). 

Beside the module file, module assets like scripts, configuration files and other resources are placed. These have to be referenced inside the module file and do nothing on their own.

A typical module directory looks like the following:
```
my-module/
    module.yml
    my-config.conf
    my-script.sh
    my-other-file.txt
```
## Properties
At the moment, the properties inside the module file span three broad categories.

Usually at the top of a module file, there is the metadata:
```yml
# module.yml

name: [string] # display name
description: [string] # small description
author: [string] # optional - basic information about the author
version: [string] # some version number or similar
```
- `name` - Display name which is easier to understand.
- `description` - Small description of the module contents or its component.
- `author` (optional) - The name or something similar of the author.
- `version` - Current version number of the module.

It is important to note that the metadata is entirely cosmetic. It does not affect the function or content of the module in any way. Even the version number does not matter. The only purpose of it is to give some more context to a module in addition to its alias.

The next category encompasses properties used for dependency resolving and similar things:
```yml
# module.yml

alias: [string] # optional - overrides the module alias

provides: [string] # optional - alternate alias this module provides
depends: [string1 string2 string3 ...] # optional - dependencies of the module
```

- `alias` (optional) - This overrides the alias, which is normally determined with the directory name. Setting the alias this way is generally discouraged, since it makes the repository directory less informative.
- `provides` (optional) - Set another alias which this module provides. This is a common practice also found in package managers and similar software. It allows for multiple modules providing the same alias without conflicts, and allows other modules to depend on any of those.
- `depends` (optional) - Set other modules as dependencies, which are installed alongside this module if it is installed.

For more information about the dependency system, visit the [Dependencies page](dependencies).

The last category is simply the jobs array. Here the jobs are defined.
```yml
# module.yml

jobs: # array of jobs of the module
  - [job1]
  - [job2]
  ...
```
- `jobs` - Array containing the jobs that define a modules functionality.

This is the most important part of a module file. Here, in the form of jobs, all changes that a module does to a system are defined. If a module is installed, these jobs will be installed from top to bottom. On removal, they are removed in reverse order. Find out more about jobs at their dedicated [Jobs page](jobs).

## Qualifiers
Up until now, we have talked about the identifier of a module as an alias. On a more technical level, you would call such an alias an ordinary *qualifier*. Ordinary qualifiers are just one word, and are simple to remember and easy to work with. An example for such a qualifier would simply be `my-module`. The problem is though, that they are not unique. In a real scenario, a qualifier can match multiple different modules, since each different repository can have a module of the same alias.

Because of that, we also have *unique qualifiers*. Unique qualifiers do what their name says, contrary to normal qualifiers, they are unique. They are comprised of their repository alias, a slash, and the normal qualifier of the module. An example for a unique qualifier is `my-repository/my-module`. Internally, pusta always works with the unique qualifier of a module, and will always show the unique one in its output. 

When you work with pusta, you can usually use either of the two types of qualifiers. Normal qualifiers are easier to type and remember, while unique qualifiers can be more precise. Because of the fuzziness of the normal qualifier, Pusta will prompt you if there are two possible modules that match your qualifier.

It is also important to note, that since pusta internally resolves the modules by their unique qualifier, changing it can lead to severe consequences. If either the repository or the module alias changes, that module or all the modules of the repository will have a different unique qualifier and thus can no longer be resolved correctly by Pusta. Pusta will then no longer know whether such a module is installed or not. So be careful when changing aliases.

## Workflow
Pusta makes it really easy to install, update and remove modules. In this regard, it works just like an ordinary package manager, as you can use the `pusta install`, `pusta remove` and `pusta udpate` in your terminal.

Learn more about these commands over on the [Workflow](workflow#modules) page.

## Example
This example module incorporates three different jobs, has all metadata, and also provides a more general qualifier.
```yml
# module.yml

name: hyprland
description: An automatic tiling wayland compositor with glorious animations
author: Virt
version: 0.9

provides: wayland-compositor

jobs:
  - title: Installing git version of Hyprland
    job:
      type: package
      names: hyprland-git

  - title: Adding startup script for easy access
    job:
      type: file
      file: start.sh
      location: /usr/bin/de
      root: true

  - job:
      type: file
      file: config.conf
      location: ~/.config/hypr/hyprland.conf
```
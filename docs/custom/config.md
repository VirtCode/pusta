# Configuration
To adapt Pusta to your system and your needs, it can be configured over its config file. Pusta has built in default values, which for basic usage are enough. For certain things, mainly the use of the package job, a configuration is necessary.

To get started, create the config file at `$XDG_CONFIG_HOME/pusta/config.yml` (on most common systems or by default `~/.config/pusta/config.yml`). Now adjust the values as you need them. You only need to set the values you want different from the default, Pusta will use the default value automatically for the others. This means, **all config values are optional**.

> **Bootstrapping Pusta**
> Of course, you'd want to have this config file in your dotfiles as well. It is quite intuitive, just create a module that holds this config file and has a file job that installs it. You only need to make sure that this one module installs with the default values pusta ships.

## General
The configuration comprises a few general attributes, many of which are subcategories for configuration of a specific component. These are the few general attributes:
```yml
# config.yml

cache_dir: [path] # directory where pusta stores its state

system: # subcategory for your system environment
security: # subcategory for security specific settings
```

- `cache_dir` - Pusta stores its state (which modules are installed, and so on) in the directory set here. By default this is at `$XDG_STATE_HOME/pusta` or `~/.local/state/pusta`. It is **not recommended** to change this option, since you'll also have to move the cache to your new directory, or otherwise pusta won't know what you have installed. **Note that "cache" is not at all a good description for the content of this directory - it can't just be deleted without any consequences.**
- `system` - This subcategory holds options for your system environment, learn more in the [Environment](#environment) section.
- `security` - This subcategory holds options for security specific settings, mainly when Pusta should prompt for manual confirmation. Learn more in the [Security](#security) section.

## Environment
This subcategory holds options which are related to your system configuration and the programs you want pusta to use. Contrary to the title of this section, these options are under the `system` attribute. The following attributes can be changed:
```yml
# config.yml

system:
  default_directory: [path] # default directory where relative paths start

  root_elevator: [command(COMMAND)] # program to elevate to root privileges
  file_previewer: [command(FILE)] # program to preview files
  
  package_manager: # subcategory for your package manager
```

As you can see, some of these properties require commands, which have dynamic arguments in them. To specify where each argument goes, a special syntax is used, giving the argument name in caps surrounded by ampersands. This special string is replaced by the actual argument on runtime. Take the default for the `root_elevator` as an example: `sudo %COMMAND%`

- `default_directory` - The directory where shell commands are executed if not set otherwise. Relative paths provided in module definitions are subpaths of this directory, unless specified otherwise in the documentation. Since this can impact how certain modules are installed, it is **not recommended** to change this property. The default is the home directory (`$HOME`).
- `root_elevator` - The tool that is used to acquire root privileges. This used to perform things on the system that require root privileges. Because everything pusta does is over the shell, this program will be used in the shell to acquire root privileges for just that single action. This command needs to contain the `%COMMAND%` argument. In most cases this will be `sudo %COMMAND%` (the default), or `doas %COMMAND%`.
- `file_previewer` - This tool is used to preview scripts before executing them, if so configured in the security settings. This command takes the `%FILE%` argument. The default is `less %FILE%`.
- `package_manager` - This is an entire category for how to use the system package manager. The default for this category are dummy values, which will print an error if they are used.


### Package Manager
The `package_manager` category inside the `system` attribute holds information about which package manager is used. Pusta is very flexible about which package manager is used. Its only criteria is that it has a command for installing and removing and can take multiple packages, split by spaces between them, as arguments for these commands. However, it is recommended to use set this config to your system package manager, from which most of your software comes from.

These package manager attributes are solely used for the `package` job, to install system packages. Naturally, the package job can only install packages which the package manager configured here can.

> Bear in mind, that the package manager is often a critical point for compatibility between repositories from different users. If they use different package registries (like arch packages and ubuntu packages), they are bound to be incompatible when used on the same system. Always check which packages a user, and therefore his repositories use, before trying to install their configurations.

The package manager can be configured over the following three properties. These three properties do all have to be either defined, or undefined, so there are no individual default values. 
```yml
# config.yml > system

package_manager:
  root: [boolean] # must the package manager be run as root
  install: [command(PACKAGE)] # command to run to install packages
  remove: [command(PACKAGE)] # command to run to remove packages
```

- `root` - Sets whether the package manager should be run as root. Most of the time, this is set to true, but some, mainly AUR helpers or something similar explicitly need to be run as a normal user.
- `install` - The command to install packages. It needs to take the argument `%PACKAGE%`, which is a list of packages, split by a single space. It is recommended to set this command to one with options, that skip most prompts for the user as Pusta can be configured to prompt before running these commands anyway.
- `remove` - The command to remove packages, which also takes the argument `%PACKAGE%`.

## Security
The subcategory under `security` houses options to configure Pusta when to prompt the user before doing something. This is to greatly improve security, so pusta can be set to not run anything on the system without user consent. The default setting is less strict, as it runs everything, except when root privileges are involved.

This configuration is by no means a guarantee, even if configured strictly, that no unsafe things are executed on your system. Always check what the modules do and install before running them, especially when they are from another user.

The following options are to your disposal:
```yml
# config.yml

security: 
  extra_confirm_everything: [boolean] # prompt extra for everything that is being done to the system

  preview_scripts: [preview] # preview scripts

  confirm_packages: [boolean] # confirm package installs
  confirm_execution: [confirm] # confirm execution of commands and scripts
  confirm_files: [confirm] # confirm the copying or linking of files
```

Notice that there are a few datatypes expected that are not booleans. One of them is `preview`. This is a type which accepts the values `always`, `never`, `root`, `ask` and `ask-root`, which do what they say on the tin. The other type is `confirm` which only discerns between root actions and not, by accepting `true`, `false` and `root`. 

- `extra_confirm_everything` - This options allows you to extra confirm everything that will be run in the shell. This really includes everything, like internal things like the creation of directories for the file job, or an additional confirmation regardless of the options set below. It is either used for debugging or if you are extra cautious about your system. Bear in mind that the amount of prompts with this option can get annoying. By default, this is set to false.
- `preview_scripts` - This specifies when to preview scripts before executing them. It supports many different values. The ask variants ask before launching the preview tool, whilst the other preview automatically. It can also be differentiated between only confirming things with root. By default, this is set to ask if executed with root.
- `confirm_packages` - This enables a prompt before launching the package manager to install packages. By default, this is set to true.
- `confirm_execution` - Sets whether you are prompted before executing a command or a script of a module on your system. Here, it can also be differentiated between whether it is being run as root or not. For scripts which have been previewed, a confirmation prompt will always been shown regardless of this option. By default, this is set to root only.
- `confirm_files` - Whether to confirm the copying or linking of files. It can also be differentiated between root and non-root operations. By default, this is set to root only.

## Example
As an example, here is a config file with a few options changed. You can see how only those are defined and the default values for the rest are kept as a consequence.

```yml
# config.yml

system:
  package_manager:
    root: true
    install: pacman -S %PACKAGE%
    remove: pacman -Rs %PACKAGE%
    
security:
  preview_scripts: ask
  confirm_execution: true
```


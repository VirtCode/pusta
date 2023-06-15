# Other Commands
Besides the common workflow commands, there are a few commands which are also quite useful. Currently, most of them are commands to see the state of Pusta, like which modules are installed, and which are available.

## List
The `list` command is probably the most useful command of the bunch here. It lists all modules and repositories you have currently installed or added. This is very useful, as it gives you a quick overview over your system and configuration, and provides insight of where your modules are coming from.

```shell
pusta list
```

This will produce something along these lines as output:
```
Added source repositories:
   virt (/home/joshua/.dotfiles)
   test (/home/joshua/code/pusta/repo)

Installed modules:
   Alacritty (virt/alacritty-1.0) at 04/11/23
   Fonts (virt/fonts-1.0-orphaned) at 03/12/23
   Pusta Config (virt/pusta-0.9) at 05/25/23
   PipeWire (virt/pipewire-1.1-outdated) at 05/25/23
```

Up top, we have the added source repositories. This will show a line for each repository, giving insight about its current alias and the path where it lies on the filesystem.

Below, the more valuable information can be found. Here, pusta tells you information about which modules you have installed. It shows you their unique qualifiers together with the installed version number, as well as an installation date. Additionally special attributes are shown after the version number, that indicate something about the module's state:
- `orphaned` - If a module is orphaned, it means that it is installed, but the source of it no longer exists in its repository. This often happens when the unique qualifier of that module was changed, or the module was deleted.
- `outdated` - This means that there is a newer version of the module available, which can be installed by updating it.

## Query
The `query` command can be used to query your available modules. This is mainly used if you have two different modules with the same alias, and you quickly want to see which is which. Additionally, it can be used to check whether a module is available. For example:

```shell
pusta query keyring
```

This will produce the following output:
```
virt/keyring-1.0 installed
 Keyring by Virt
 Installs a libsecret keyring implementation and creates a keyring.
```

This will show you the name and description for all modules that qualify for the given alias or qualifier. In addition, it tells you whether you have this module already installed.

## Help
If you want some quick information about the commands and their arguments, you can use the `help` command. Incredible, I know. Just type:

```shell
pusta help
```

## Examples
Here is an example for every command listed on this page.
```shell
# list all installed modules and added repositories
pusta list

# query information about another module
pusta query hyprpaper

# and last but not least, display help page
pusta help
```
# Command Job
The `command` job is the simplest job pusta has to offer. It simply can run a command on installation and one when uninstalling. This is useful when something small needs to be run during installation, like refreshing a cache, setting some settings through the installed utility, and so on.

The command job is very similar to its sibling, the [`script` job](script.md).

## Definition and Properties
The command job uses `command` as its type identifier. All properties are listed here:
```yml
# module.yml > jobs

- job:
    type: command
    
    install: [command] # command to run during installation
    uninstall: [command] # optional - command to run when uninstalling
    
    reinstall: [boolean] # optional - reinstall instead of update
    show_output: [boolean] # optional - show the output during installation
    root: [boolean] # optional - run the command as root
    running_directory: [path] # optional - directory where the command is run
```

- `install` - The command that is run on installation.
- `uninstall` (optional) - A command that is run when the job is removed.
- `reinstall` (optional) - If true, a reinstall is performed if the job is updated. This means, that the uninstall command is run and the install command again when updating. Otherwise and by default, only the install command will be run on an update.
- `show_output` (optional) - Whether to show the output in the console when installing or removing. By default, this is true.
- `root` (optional) - Whether the commands are run with root. This is false by default.
- `running_directory` (optional) - Directory where the command is executed. This is the module directory by default.

## Security
With the command job, arbitrary things can be executed on your system. This is especially important, when installing modules from repositories of other users. Pusta does not guarantee anything about the safety of a module when executing. 

However, there are configuration options which will show the command for review before execution. You can find those under the [security configuration](../../custom/config.md#security). By default, pusta will only do that, when the command will execute as root.

Always be cautious when installing modules from other users and review their modules source before doing so.

## Example
In this example, a command is run on installation, which is also undone if the job is uninstalled. It does not show its output during the installation.

```yml
# module.yml > jobs

- title: Set git to use libsecret for credentials
  job:
    type: command
    install: git config --global credential.helper /usr/lib/git-core/git-credential-libsecret
    uninstall: git config --global credential.helper cache
    show_output: false
```


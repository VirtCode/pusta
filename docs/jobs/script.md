# Script Job
The `script` job is the most versatile job of them all. With it, you can run a custom script when installing, making it very powerful.

The script job is very similar to its sibling, the [`command` job](package.md).

## Definition and Properties
The script job uses `script` as its type identifier. All properties are listed here:
```yml
# module.yml > jobs

- job:
    type: script
    
    install: [path] # filename of script to run during installation
    uninstall: [path] # optional - filename of script to run when uninstalling
    
    reinstall: [boolean] # optional - reinstall instead of update
    show_output: [boolean] # optional - show the output during installation
    root: [boolean] # optional - run the scripts as root
```

- `install` - A filename of the script in the module directory to run when installing.
- `uninstall` (optional) - The filename of the script run when uninstalling.
- `reinstall` (optional) - If true, a reinstall is performed if the job is updated. This means, that the uninstall script is run and the install script again when updating. Otherwise and by default, only the install script will be run again on an update.
- `show_output` (optional) - Whether to show the output in the console when installing or removing. By default, this is true.
- `root` (optional) - Whether the scripts are run with root. This is false by default.

## Security
As with the command job, the script job can execute arbitrary scripts, which can do anything on your system. This is especially important, when installing modules from repositories of other users. Pusta does not guarantee anything about the safety of a module when executing.

However, there are configuration options which will show the script for review before execution. You can find those under the [security configuration](config#security). By default, pusta will only do that, when the script will be run as root.

Always be cautious when installing modules from other users and review their modules source before doing so.

## Example
This job will simply execute a script which is found in its module directory.
```yml
# module.yml > jobs

- title: Downloading and rescaling wallpapers from the internet
  job:
    type: script
    install: download.sh
```


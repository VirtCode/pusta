# File Job
The `file` job is one of the most important job types. It does what most dotfile managers do, it copies files from your dotfiles directory to their location where they can be used.

By default, the file job copies the files from your module directory to their target location. It does not symlink the files by design. This is because pusta will eventually support variables, which can be referenced by any file and will be filled in during installation. Because of that, copying is the only option to achieve dynamic variables, since the target file will be different from the source. It is still easy to update the file at the target location by just running the update command. If you require symlinks tough, there is a link option as explained below.

## Definition and Properties
The definition is akin to the definition of every other job type. The file job uses the type identifier `file`. The specific properties are listed below:
```yml
# module.yml > jobs

- job:
    type: file
    
    file: [path] # the file to copy
    location: [path] # the target location of the file
    
    root: [boolean] # optional - perform the copying as root
    link: [boolean] # optional - symlink instead of copy
```

- `file` - File name of the file inside the module directory to copy.
- `location` - Target location to copy the file to. This has to be a path to a file and not the parent directory. `~` is supported for specifying the home directory.
- `root` (optional) - Perform the copying and everything as root. This has to be used if copying to somewhere you need root privileges, because you **never run pusta as root**.
- `link` (optional) - Whether to link the file to the target location instead of copying it. Linked files will not support dynamic content like variables.

## Internals
For easier troubleshooting or better understanding of the file job, here are a few points about how this job works:
- If not existing, parent directories will be created at the target location.
- The job overwrites files at the target location. However, it supports caching. This means that the overwritten file will be saved in the cache and can be restored when the job is uninstalled again. Please note though that this is not guaranteed, since a cache can be lost for example when changing the order of the `jobs` array.
- When updating, the job will detect changes on the source file and initiate an update. Changing the definition is not necessary.

## Example
In this example, a file is copied to a directory only accessible with root.
```yml
# module.yml > jobs

- title: set greetd config to use custom greeter
  job:
    type: file
    file: config.toml
    location: /etc/greetd/config.toml
    root: true
```
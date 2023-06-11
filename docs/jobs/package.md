# Package Job
The `package` job a very specific job. It installs packages over your system package manager. Almost every module will use a package job, since most modules configure a software which first needs to be installed. On the inside, this is just a glorified command job, with the command being composed dynamically based on the pusta configuration.

## Definition and Property
The package job uses the type identifier `package`. The one property of the package job is listed below:

```yml
# module.yml > jobs

- job:
    type: package
    
    names: [string1 string2 string3 ...] # a list of names of the packages to install
```

- `names` - A list of names of the packages that are installed. Each package name is seperated from the previous with a space.

## Configuration
Other than the other jobs, this job always requires custom configuration. As you can guess, pusta needs to know what package manager you like to use. Thus, you first have to set the [package manager configuration](config#package-manager) in the pusta config file. Also make sure to adjust the root elevator, if your package manager runs as root.

As a consequence are repositories from different users or systems not always compatible. If they use different packages, which do use different names or have a different selection of packages altogether, the pusta configurations from these two sources may not be compatible. So make sure that you use the same packages as are used in a foreign repository before installing modules from it.

## Example
This example job installs three packages at once.
```yml
# module.yml > jobs

- title: Installing PipeWire integrations with other backends
  job:
    type: package
    names: pipewire-alsa pipewire-pulse pipewire-jack
```

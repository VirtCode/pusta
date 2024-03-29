# Jobs
In Pusta, changes on the System are done through different Jobs. Each Job represents a simple Action. There are many different Jobs which do different things. This page focusses on common elements between all, but also links to the different types.

## Definition

Jobs are defined under the `jobs` attribute in a `module.yml` file. This attribute is an array of Jobs.

```yml
# module.yml

jobs:
  - [job1]
  - [job2]
  - [job3]
  ...
```

Upon installation, these Jobs are executed from top to bottom. So put a Job that depends on another after that dependency. When the installation of a job fails, the whole installation is cancelled. In this case, the previous installed jobs are being uninstalled in reverse order. 

## Properties
The definition of a Job comprises two parts, a general part, which are properties which are present on every job, and specific properties which depend on the job type. In general, a job supports the following properties.

```yml
# module.yml > jobs

# General Properties
- title: [string] # optional - title displayed on installation
  optional: [boolean] # optional - set the job to be optional

  job:
  
    # Specific Properties
    type: [job-type]
```

The general properties which are job type independent are:
- `title` (optional) - A title displayed during installation clarifying the jobs purpose. If not provided, one is generated by Pusta based on the job type and its specific properties.
- `optional` (optional) - Set the job to be optional. If an optional job fails during installation, the installation still continues instead of being cancelled. By default, a job is not optional.
- `job` - Holds the specific properties which are based on the job's type.

Specific Properties are different for every job, based on its job type. Specific properties are always specified under the `job` property. There is only one property shared between every type:
- `type` - Specifies the type of job (see [below](#types))

## Types
As mentioned, there are different types of jobs which do different things. Of which type a job is, is specified as seen by the specific `type` property. Each type does have a different function and thus requires different specific properties. Currently the job types include:
- [`file`](jobs/file.md) - Copies a file from the module to a specific location
- [`package`](jobs/package.md) - Installs a specific package on the system using the configured package manager
- [`script`](jobs/script.md) - Executes a script from the module on installation
- [`command`](jobs/command.md) - Runs a custom command upon installation

## Example
In this example we download and install a rust toolchain. First we install a package and give it a more descriptive title. Afterward, we install a toolchain, also give that a title, and set that to optional, since this step is not mandatory.
```yml
# module.yml

jobs:
  - title: Installing the rust toolchain manager
    job:
      type: package
      names: rustup

  - title: Installing latest nightly toolchain
    optional: true 
    job:
      type: command
      install: rustup toolchain install nightly
      show_output: true
```
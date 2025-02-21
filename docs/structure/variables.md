# Variables
One of the most powerful features of Pusta is its variable system. This allows the abstraction of a config, making it adaptable to the system it is installed on. This can be very powerful when you, for example, intend to use on multiple different hosts, or want to share it with other people. It also makes it possible to share properties between modules, so that you can control those centrally, making it easy to change many things at once.

When installing a job, Pusta evaluates the values of the variables, replacing their definition with the evaluated value in the string or file they were put in. Variables are processed in most files used in jobs (except when the file is linked), and in strings like commands that are run on the host system.

## Configuration
The values for variables are loaded from one big, json-like structure. This structure is composed of variable sources, all declared with yaml syntax.

This means that you can use the following primitive types:
- `String`: a UTF-8 encoded String
- `Number`: a double precision floating-point number
- `Boolean`: `true` or `false`. As they are specified in yaml, one may also use `yes` or `no`.

These values are then identified by a string identifier, and arranged in objects and lists. Accessing a sub value of an object is done with a dot (`.`).

So the definition in yaml with the respective names to access them would be for example:
```yml
my-theme:
  background: red         # mytheme.background
  foreground:
    text: yellow          # mytheme.foreground.text
    size: 42              # mytheme.foreground.size
  transparency: true      # mytheme.transparency
username: feanor          # username
```

## Loading
This variable structure is loaded from different sources which are then combined into one big structure. The following four sources are used for a module inside a repository:
- **Module Variables**: These variables are loaded from the `module.yml` file of the module, see [Module Configuration](module.md#properties).
- **Repository Variables**: These variables are loaded from the `pusta.yml` file of the repository of the module, see [Repository Configuration](repository.md#properties).
- **Injected Variables**: These variables are collected from all installed modules, see [below](#injected-variables).
- **System Variables**: These variables are loaded from a system-specific file, see [below](#system-variables).
- **Magic Variables**: These are variables which are populated by Pusta, depending on the host system.

These different sources are loaded and then merged when installing a module. This merging works like this: If two objects have the same name, they are combined into one big object, where the same properties are merged the same way. If two lists have the same name, they are combined into one list where the second list is appended to the first one. If two values have the same names, the second value is used.

This means that the order in which these variables are merged is important, and it is shown in the list above. This means that the variables are merged in the following order *module*, *repository*, *injected*, *system* and then *magic*. Informally, this means that **module variables are the least important** and **magic variables are the most important**.

This merging is designed this way, such that you can *override* variables of modules in the repository, such that multiple modules use the same values. And you can override in the system variables, if you want something on only one system.

### Injected Variables
Injected variables are variables which are sourced from the `injections` field of all installed modules. This means a module can inject variables into the global variable tree if it is being installed. This means other modules can change their things based on which other modules happen to be installed.

*This is a kind-of experimental feature and might require an update after the installation or removal of a module.*

### System Variables
The system variables are loaded from the file `.config/pusta/variables.yml`. This file is optional and the variables are treated as empty if it is not found. The content of the file is just the structure described above and nothing more.

### Magic Variables
The magic variables are as mentioned variables, which are dynamically defined by pusta itself and cannot be changed. They depend on the host system, pusta is run. The following variables are available:

| Variable Name    | Description of Content                   |
|------------------|------------------------------------------|
| `pusta.hostname` | The hostname of the system it is ran on. |
| `pusta.username` | The username of the user running pusta.  |

## Syntax
Having set some variables, you can now go on to use them in your configuration files. Generally, variables are filled in most files and strings used by pusta, but there are some exceptions. See the page for your specific [job](jobs.md) for more information about that. To now use a variable in a file, you can use the following syntax:
```
%% your.variable.name %%
```
So all variable references, or more generally, all references for the pusta templating system are specified between two ampersands (`%%`). This syntax is uncommon by design, such that it does not interfere with any other common templating system.

Note that when referencing just a variable, the variable **has to be a value** and may not be an object or list.

There are more advanced usages of variables, but those are not described here. See [modifiers](variables/modifiers.md) for how you can modify the contents of your variables before inserting. Or see [control flow](variables/control.md) for information about you can create advanced control flow like if statements.

## Example
An example showing the loading order and insertion. We assume the system has the hostname `example`. This example will specify some variable definitions and then see what they are evaluated to by echoing things to console. Yes, it is a very boring example.

The system variables are defined like this:
```yml
# variables.yml
pusta.hostname: i-will-be-overwritten
theme:
  color: red
  padding: 3
```
In the repository, we have this:
```yml
# pusta.yml
variables:
  theme:
    padding: 42
```

And for our module, we can now use these variables
```yml
# module.yml
# ... module metadata

jobs:
  - job:
      type: command
      install: 'echo "color: %% theme.color %%, padding: %% theme.padding %%"'
  - job:
      type: command
      install: 'echo "am I default? %% default %%"'
  - job:
      type: command
      install: 'echo "on host %% pusta.hostname %%'
variables:
  default: yes
```

Because we have used many echoes, we can now watch what is actually printed on the console:
```
color: red, padding: 42
am I default? true
on host example
```

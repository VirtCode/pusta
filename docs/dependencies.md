# Dependencies
Another way, in which pusta is very similar to a normal package manager, is its dependency system. It has a fully featured dependency system built in. This enables you to structure your configuration modular, but still ensure that the dependencies for a module are available. Note, that using the dependency system is of course optional, you won't even notice it if you don't use it in your configs.

With it, each module can have a list of dependencies, that get installed alongside that module when it is. This is very useful when you have a module that requires something else to run or install, that however is also used by many other modules and does really belong to that module. For example if many of your modules depend on a particular scripting language, they all may have a module as a dependency that installs and configures that scripting language.

Note, that pusta is not at all strict in enforcing the dependency system. If a dependency is not found, the user can just choose to continue anyway. The same way, if a user wants to remove a dependency of a module, he can choose to do so anyway, although that module is being depended on. But as you can see, pusta will always prompt the user and won't allow these sometimes undesired operations to happen unwanted.

## Resolving
As with any dependency system, there are a few properties, which enable you to generalize your dependency relationships. Because of that, the dependency resolving is not solely based on [Qualifiers](module.md#qualifiers), but also on a special property `provides`. This property serves as an alternative alias for each module, special being that you can have many modules which have the same `provides` in one repository.

That means, that there are three options for how a module may qualify as a dependency for a given string:
- **provides** - The module provides this string in its `provides` property.
- **alias** - The module's alias or normal qualifier matches the string.
- **unique qualifier** - The module's unique qualifier matches the string.

If multiple modules qualify for a dependency, Pusta will prompt the user to choose which dependency to install.

## Properties
To configure your module with dependencies, or for it to be able to be leveraged as a dependency by other modules, you can use the following properties in your `module.yml` files:

```yml
# module.yml 

depends: [module1 module2 ...] # optional - list of strings which are the module's dependencies

provides: [string] # optional - string which this module provides
```
- `depends` (optional) - Specifies the dependencies of the module. It is a list of strings for which other may qualify as dependencies. Which modules are meant by one of these strings can be seen in the [Resolving](#resolving) chapter.
- `provides` (optional) - Specifies an additional alias for the module. This string is only used for dependency qualifying, and contrary to the normal alias is not exclusive to only one module per repository. Again, see the [Resolving](#resolving) chapter to see how this impacts resolving.

## Example
The following example shows a scenario, where one module depends on the other. The aliases serve as extra context. The following module is the dependency:
```yml
# module.yml

alias: rustup
provides: rust
```

This module depends on the just specified dependency over that dependencies provider. It itself depends on two modules:
```yml
# module.yml

alias: custom-greeter # written in rust
depends: rust greetd
```

# Getting Started
On this page, we'll be running through the creation and installation of your first module. On the go, you'll also learn a thing or two about each feature and other mechanisms which may be relevant to you when using pusta in a bigger scale.

## System Setup
To use Pusta conveniently, its executable should be available in your path, so you can easily run it from the terminal. If you haven't done that already, take a look back at the [readme](../#installation) for installation instructions. 

To adapt Pusta to your needs and your system, you can configure it over its config file. However, pusta ships with some defaults, which are sufficient for this example. But definitely have a look at the page about [Configuration](custom/config.md) after this introduction.

## Creating your Repository
As you know from reading the readme, Pusta works with your configuration as modules - we'll take a look at those later. Anyway, multiple **modules are typically arranged together in repositories**. These repositories are then added to Pusta so you can install modules form them. So the first step is to create a new repository.

Repositories in Pusta are really simple, they are just a directory in which directories for modules will be located. Each **repository is identified by its alias**, which is typically derived from the directory name or the repository config file. 

Speaking of config file - for a directory to pass as a Pusta repository, it **must contain a repository config file**. So let's create that. Create a directory for your repository and create a new file, ```pusta.yml```, in it.

```yml
# ~/dotfiles/pusta.yml

alias: feanor # optional, overrides the repository alias
```

At this time, we'll only set one option, the `alias` option. With it, you can set the alias of your repository, so it is not derived from the directory name. Change that to something that identifies you, like your name.

> It is generally recommended to set the repository alias explicitly in the configuration, and to set that to something corresponding to your persona. If someone else wants to use your repository, it automatically has a unique alias which is descriptive of its source.

Great, now you have created your first repository. Now, all that's left to do is to **add your repository to pusta**. For that navigate with the terminal into your repository directory and run:

```
pusta source add
```

To verify that you have added the repository successfully, you may run the list command. It lists which repositories you have added and what modules you have installed.

```
pusta list
```

Now you are done with your repository, lets move on to create your first module in it.

## Creating a Module
As mentioned in the readme, a Module is an independent part of your Configuration, which you then can install and remove. It is typically sized to one part of your system or one program. This means you have a module for your window manager, one for your display manager, one for your terminal emulator, you get it. **Every component of your configuration gets a separate module.**

Additionally, a module does not only contain configuration for its component. Let's take the example of the display manager. In this case it would obviously contain the configuration for that software. But it also should contain the package name that the display manager is installed with and the commands which need to be run to enable the service for it to execute on boot. **Thus, a module contains everything to install and configure its component.**

Okay, enough theory, lets start creating your first module. A module is also quite simple, it is **just a directory containing files and most importantly, the module configuration file.** So navigate into your repository and create a new directory. Name the directory according to your component you want to configure, the example here will use the directory `welcome`. Now, create the config file ```module.yml```, inside that directory.

```yml
# ~/dotfiles/welcome/module.yml

name: Welcome to Pusta # display name
description: A very sophisticated description # text description for the module
author: Feanor # optional - basic information about the author
version: 0.1 # a version number or identifier
```

Start your config by giving it some basic Metadata. Use the ```name``` attribute to set a display name for your module. Provide a quick description about the module in the ```description``` attribute. Optionally, you can set an author with the ```author``` attribute. Lastly, give your module a ```version```. **These attributes are just cosmetic, they do not affect the function of the module**, yet they probably are mandatory to keep your configuration organized and understandable.

As you can see, we did not give the module an identifier or something similar. As with the repository, a module has an alias which is derived from its directory or the optional ```alias``` attribute. We'll go with the directory name here.

> For modules, it is recommended to use the directory name as the alias. This makes it easier to tell what each module is for when you just look at the directories of a repository.

Until now, our whole configuration doesn't do anything yet. Let's change that by adding configuration to your module. **A module does do its configuration in jobs.** A Job is a single action that is taken on your system, for example the moving of a file, the execution of a script or the installation of a package. So let's add a Job to our configuration:

```yml
# ~/dotfiles/welcome/module.yml
# metadata as shown above

jobs:
  - title: Creates the welcome file # optional - a descriptive title
    job: # attribute for specific data
      type: file # job type
      file: welcome.txt # file to copy
      location: ~/.welcome # copy location
```

Every job is declared within the ```jobs``` attribute. Start a new job by starting a sub-entry with a dash. Now give your job a title that describes what it does, with the ```title``` attribute. This is optional, Pusta will otherwise try to generate a title for each job, but we'll add one manually here anyway.

Now, start the actual job definition with the ```job``` attribute. This is where it gets interesting. As you can see in the excerpt, the first attribute we define is ```type```. **The ```type``` attribute specifies the type of job we want to run.** There are multiple different types which do different things. Here, we are using the job-type ```file```, which is for copying files to specific locations. Because of that type, we now need to define the ```file``` and ```location``` attribute, specifying which file to copy where. As you can see we reference ```welcome.txt``` under the ```file``` attribute. The path under the file attribute is relative, that means that ```welcome.txt``` refers to the file of that name inside our module directory. So you also need to go on and create this file, and preferably add some content to it. As specified by the ```location``` attribute, that file will then later be copied to ```~/.welcome```.

> **Copying? Not symlinking?**
> Yes. In the current setup, yes, symlinks would be the easier option. But later down the line Pusta will also support variables, which will fill references in your files during this copying process. That is not possible with symlinks. Additionally, updating is easy since Pusta will recopy changed files if you just run one command. If you explicitly want to symlink though, you can still do so by specifying `link: true` under the job attribute for a `file` job.

Okay, now we have defined a module with metadata and jobs to execute. Now we'll move on to installing. Open a new terminal and just run:

```
pusta install welcome
```

Pusta will now scan all added repository for modules with the alias `welcome`. Your output should now show something like this:

```
Scheduled module changes:
   Welcome to Pusta (feanor/welcome-0.1)
?? Do you want to make these changes now? [Y/n]
```

Here, Pusta prompts you whether you really want to install that module. You can confirm the installation by pressing enter.

> Notice that under the module changes, it references our module with `feanor/welcome`. This is called a *unique qualifier* and is comprised of the repository alias, a slash and the module alias. A *unique qualifier* uniquely points to one module and can also be used for the install command. This is useful if you have multiple repositories with modules with the same alias.

If all is configured well, it should say `Module installed successfully` somewhere in the output. Now you can look at your installed configuration.

```
cat ~/.welcome
```

Nice, you created and installed the first part of your configuration.

## Changing Configuration
Most of the time, a configuration is not perfect on first attempt or you'll want it to change anyway some time. 

What if, for example, the welcome file needs to contain something different? Let's edit the source file `welcome.txt` and change it to something different. Now look at `~/.welcome`, it is still what it was before. This is because this file is a copy of the source file, not a link. However, updating is really easy. Just run: 

```
pusta update
```

**Pusta will now check every module for changes and try to apply these.** It does so in a subtle manner, just updating the jobs that have changed, and not reinstalling whole modules if not neccesary. After you have confirmed the module changes, you can look at `~/.welcome` and see the new changes reflected.

But what if you need to add something completely new to your configuration, like a new job. Lets try. Add a new entry into the `jobs` attribute of your `module.yml` file.

```yml
# ~/dotfiles/welcome/module.yml
# metadata and jobs array as shown above

  - job:
      type: command
      install: echo "Hello World from Job" # installation command
      uninstall: echo "Goodbye World from Job" # optional - removal command
```

Contrary to above, we now don't include a title. This means that `job` is the first property and thus has the dash (that's yaml arrays). This time we have a job of type `command`. This job runs a command when installing (`install`) and can run one upon removal (`uninstall`).

Okay, now update your configuration with the update command. As you can see, only that new Job is being run, and `Hello World from Job` is printed to the terminal. In a real scenario, this would obviously be a command that actually does something, like refreshing a cache or something.

That's how easy updating is. Keep in mind that the update command updates all modules. You can select only one module by providing an extra argument.

## Finishing Up
Great! You now know the basics about creating and updating your configuration with the help of Pusta. Of course there are many more features that we have not at all discussed here. And there are many features still to come.

Now go on and create your configuration or port your existing one to Pusta. Have fun! If you notice any bugs or have a feature request, don't hesitate to post an Issue.

Here are a few topics which may now be relevant to you:
- See how to [configure](custom/config.md) Pusta to work with your system.
- Have a look at the [package job](structure/jobs/package.md), which installs system packages.
- Learn how to set the [script](structure/jobs/script.md#definition-and-properties) or [file](structure/jobs/file.md#definition-and-properties) job to operate with root privileges. 
- Create relationships between modules in the form of [dependencies](structure/dependencies.md).
- Unlock the potential of [variables](structure/variables.md) shared between your modules.

And yes, before we forget it, you won't need the `welcome` configuration anymore. To remove it, just run:

```
pusta remove welcome
```

Notice how the uninstall command is being run and how the `~/.welcome` file is no longer present. You can also delete the module directory from the repository.
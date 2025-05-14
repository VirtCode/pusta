# Control Flow
Variables can also used in an even more powerful way. They can be used to introduce some basic control flow into your configuration. This includes if statements based on variables or the enumeration of list entries in a file. They can be really powerful when used correctly, and allows one to make more drastic changes than just including some values.

This control flow generally shares some structure with basic [variable references](../variables.md#syntax), so make sure you have read up on that first.

## Conditionals
The first type of control flow pusta supports are conditionals, or as it should probably be called, *if-else statements*. As that implies, this allows one to include some part of a config if some condition is met, and include nothing or something else if not. Let's have a look at the syntax:

```
%% !if my-variable-with-modifiers %%

insert this if true

%% !else %%

insert this if false

%% !end %%
```

We can see that there are many words prefixed with an exclamation mark (`!`). These words are **keywords** and are *not* variable references. There are only a few of those keywords, here we used `if`, `else` and `end`.

In the first statement, we have an `if` keyword followed by a [variable reference](../variables.md#syntax), **which can contain modifiers**. If this reference evaluates to a `Boolean` and is `true`, the part until the `else` gets inserted. Otherwise the part from the `else` to the `end` is inserted. If the reference does not evaluate to a boolean, an error will be shown before the installation or update. Note, that the `else` part is optional and can be omitted. In this case, the part until the `end` will be inserted if `true` and nothing otherwise.

## List Expansions
You probably recall that the config structure supports lists, but these lists haven't been mentioned ever since. We can use lists to do so called *list expansions* which is fancy speech for "a for-each loop which inserts text for each item". For the following things, remember that lists are merged when [loading the config](../variables.md#loading), so the items of the lists can stem from various sources. Let's look at the syntax:

```
%% !list my-list-variable %%

we can reference the list item with _
e.g. %% _.name %%

%% !end %%
```

We can see that we now use the keyword `list`, followed by a reference to the list we want to use. Inside the following block, we have now access to the special variable `_`, which is the current list item for each iteration. The text is then inserted into the file after each other. Note that the newlines between the two keywords are also inserted, which means that if you don't have any newlines, everything will be on one line.

Using list expansion not only list variables can be expanded but also group variables. When expanding group variables they get transformed to a list of objects with a `key` and a `value` property. The `key` holds the key of the group field and the `value` property holds the value of the group field.

## Examples
Let's see an example for a conditional. In this case, we include some part of the config only if we are on a specific host:
```lua
%% !if pusta.hostname:eq("my-desktop") %%

my_rename_rule = {
  matches = {{
    { "node.name", "equals", "the-stupid-name-for-the-headphone-jack-on-my-motherboard" },
  }},
  apply_properties = {
    ["node.description"] = "Headphones",
  }
}

%% !end %%
```

A more interesting example would be with using lists. Here we have the following in our ssh config, which would expand our list into host definitions.
```
%% !list hosts %%
Host %% _.name %%
    HostName %% _.ip %%
    User %% _.user %%
    Port %% _.port %%
%% !end %%
```

We would now put the following data in our `variables.yml`:
```yml
# variables.yml
hosts:
  - name: my-server
    ip: 172.0.0.1
    port: 22
    user: me
  - name: my-google-server
    ip: 8.8.8.8
    port: 22
    user: root
```


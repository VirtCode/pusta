# Modifiers

As seen in the documentation about variables, variables can be modified using **modifiers** before inserting them,
allowing one to tailor the content of a variable to the file's needs.

All modifiers are inbuilt functions of pusta and cannot be extended. So if you would like to see another modifier
included, don't hesitate to contribute! Pusta already contains a couple of basic modifiers, as seen
under [Available Modifiers](#available).

## Syntax

The syntax of modifiers is pretty straight forward. When specifying a variable, one can append a modifier at the end of
it, separated by a colon (`:`). The arguments are specified inside of braces which can be omitted if there are none:

```
%% pusta.hostname:contains("laptop-") %%
%% pusta.hostname:case-upper %%
%% pusta.hostname:case-snake("case-kebab") %%
```

Modifiers can be chained and take other variables with modifiers as parameters, to create some more interesting results:

```
%% pusta.hostname:eq(hostnames.pc):and(pusta.username:lower-case:eq("feanor")) %%
```

## Available

The following modifiers are currently available. A modifier listed under its *name* is intended to be applied *on type* given with *parameters* as described to achieve a desired *effect*.

| name             | on type     | parameters     | effect                                                                                                                 |
|------------------|-------------|----------------|------------------------------------------------------------------------------------------------------------------------|
| `format-color`   | *string*    | *string*       | formats a color to the desired format, see [a detailed explanation](#color-format)                                     |
| `eq`             | *any*       | *any*          | compares the variable and the parameters, giving a *boolean* value of `true` when they are the same type and content   |
| `if`             | *boolean*   | *any*, *any*   | acts like a ternary operator, if the variable is true, the first parameter is given, otherwise the second one          |
| `contains`       | *string*    | *string*       | gives a *boolean* `true` if the variable contains the parameter                                                        |
| `case-upper`     | *string*    |                | converts the variable to upper case                                                                                    |
| `case-lower`     | *string*    |                | converts the variable to lower case                                                                                    |
| `case-camel`     | *string*    | *string*       | converts the variable from the parameter case to camel case                                                            |
| `case-snake`     | *string*    | *string*       | converts the variable from the parameter case to snake case                                                            |
| `case-pascal`    | *string*    | *string*       | converts the variable from the parameter case to pascal case                                                           |
| `case-kebab`     | *string*    | *string*       | converts the variable from the parameter case to kebab case                                                            |
| `tilde`          | *string*    |                | expands the `~` in the given string to the user's home directory                                                       |
| `parsenum`       | *string*    |                | parses the variable from string to a number                                                                            |
| `not`            | *boolean*   |                | inverts the boolean value of the variable                                                                              |
| `and`            | *boolean*   | *boolean*      | gives the boolean and of the variable and the parameter                                                                |
| `or`             | *boolean*   | *boolean*      | gives the boolean or of the variable and the parameter                                                                 |
| `add`            | *number*    | *number*       | adds the parameter to the variable                                                                                     |
| `sub`            | *number*    | *number*       | subtracts the parameter from the variable                                                                              |
| `mul`            | *number*    | *number*       | multiplies the variable by the parameter                                                                               |
| `div`            | *number*    | *number*       | divides the variable by the parameter                                                                                  |


### `color-format`
This modifier can be used to format colors to suit any possible configuration format. The syntax is inspired by a classic `printf` statement though with special formatters.

Input colors must be strictly formatted as hexadecimal. It can contain an alpha value and start with a slash. If no alpha value is provided the color is fully opaque with an alpha of `1`. These are permitted formats:
```
#RRGGBB, RRGGBB, #RRGGBBAA or RRGGBBAA
```

A single format part starts with a `%`, formats one color component and consists of two parts, e.g:
```
%Xr or %Fg or %Db
```
- The first part is the format. Possible values are:
  - `X` - hexadecimal, two characters long, e.g. `FF`
  - `D` - decimal, between `0` and `255`, e.g. `128`
  - `F` - as a floating point number between `0` and `1`, e.g. `0.75`
- The second part specifies the color component. Possible values are `r` - red, `g` - green, `b` - blue and `a` - alpha.

These format parts can be put together to create a formatter pattern. Here are some examples:
- `#%Xr%Xg%Xb` produces `#RRGGBB`
- `rgba(%Dr, %Dg, %Db, %Fa)` produces `rgba(RRR, GGG, BBB, A.AA)`
- `rgba(%Xr%Xg%Xb%Xa)` produces `rgba(RRGGBBAA)`
- `opacity: %Fa` produces `opacity: A.AA`

## Example
Here are some real-world examples for how modifiers are used at their best:
```ini
# for hyprland's weird color format
col.inactive_border = %% color.border.inactive:format-color("rgba(%Xr%Xg%Xb%Xa)") %%

# using half of a value for some things
gaps_out = %% look.layout.gaps %%
gaps_in = %% look.layout.gaps:div(2) %%

# compare and insert as a one-liner
layout = %% pusta.hostname:eq("desktop"):if("master", "dwindle") %%
```

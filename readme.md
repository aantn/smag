# smag - show me a graph!

Easily create graphs from cli commands and view them in the terminal. Like the `watch` command graphs the output.

**smag was inspired and based on code from the wonderful [gping](https://github.com/orf/gping) tool by Tom Forbes**

![](./images/readme-example.gif)

Table of Contents
=================

   * [Install :cd:](#install-cd)
   * [Usage ](#usage)

# Install :cd:
```bash
git clone https://github.com/aantn/smag.git
cargo install --path .
```

# Usage

Just run `smag [shell_cmd]`. e.g. `smag "ps aux | wc -l"`

```bash
$ smag --help                                                                                                           ✔  2355  17:59:43
smag 0.5.0
Show Me A Graph - Like the `watch` command but with a graph of previous values.

USAGE:
    smag [FLAGS] [OPTIONS] <cmds>...

FLAGS:
    -d, --diff       Graph the diff of subsequent command outputs
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --history <buffer-size>          Specify number of points to 'remember' and graph at once for each commands
                                         [default: 100]
    -n, --interval <polling-interval>    Specify update interval in seconds. [default: 1.0]

ARGS:
    <cmds>...    Command(s) to run
```

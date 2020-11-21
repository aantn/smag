# smag 🚀

Easily create graphs from cli commands and view them in the cli. Like the `watch` command but with a graph.

![](./images/readme-example.gif)

Table of Contents
=================

   * [Install :cd:](#install-cd)
      * [Homebrew (MacOS   Linux)](#homebrew-macos--linux)
      * [Binaries (Windows)](#binaries-windows)
      * [Cargo](#cargo)
   * [Usage :saxophone:](#usage-saxophone)

# Install :cd:

**smag is heavily based on code from the [gping](https://github.com/orf/gping) tool by Tom Forbes**

## Homebrew (MacOS)

```bash
brew install gping
```

## Homebrew (Linux)

```bash
brew install orf/brew/gping
```

## Binaries (Windows)

Download the latest release from [the github releases page](https://github.com/orf/gping/releases). Extract it 
and move it to a directory on your `PATH`.

## Cargo

`cargo install gping`

# Usage :saxophone:

Just run `gping [host]`.

```bash
$ gping --help
gping 0.1.7
Ping, but with a graph.

USAGE:
    gping [OPTIONS] <hosts>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --buffer <buffer>    Determines the number pings to display. [default: 100]

ARGS:
    <hosts>...    Hosts or IPs to ping
```

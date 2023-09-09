# cpi

[![Crates.io](https://img.shields.io/crates/v/cpi.svg)](https://crates.io/crates/cpi)

A command-line tool for copying files without ignored files.

Currently supported ignore-file config is `.gitignore`.

## Install

```sh
cargo install cpi
```

## Usage

```sh
cpi ./project ./project-copy

# disable .gitignore
cpi ./project ./project-copy --no-gitignore

# output as a zip file
cpi ./project ./project-copy.zip

# -f/--force will remove "project-copy" if already existed before copying
cpi ./project ./project-copy -f
```

## To-Do

- [ ] support passing ignore folder in cli

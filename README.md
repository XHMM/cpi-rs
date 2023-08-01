# cpi
A command-line tool for copying files with ignore-files applied.

Currently supported ignore-file is `.gitignore`.

## Install
```sh
cargo install cpi
```

## Usage

```sh
cpi ./project ./project-copy

# disable .gitignore
cpi ./project ./project-copy --no-gitignore
```

## ToDo
[ ] support compressed output  
[ ] support passing ignore folder in cli  
[ ] support overriding dest folder if existed
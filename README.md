# cpi
A cli for copying files with ignore-files applied.

Currently supported ignore-file is `.gitignore`.

## install
```sh
cargo install cpi
```

## usage

```sh
cpi ./project ./project-copy

# disable .gitignore
cpi ./project ./project-copy --no-gitignore
```

## to-do
[ ] support compressed output
[ ] support passing ignore folder in cli
[ ] support overriding dest folder if existed
# cargo-recursive

A cargo subcommand for running a command in all subdirectories recursively.


## Installation

```
cargo install cargo-recursive
```

## Usage

To clean all subdirectories recursively

```
cargo recursive clean
```

Print all selected crates and their versions

```bash
cargo recursive read-manifest | jq '.name + " " + .version'
```

## License

This projest is licensed under [`CC0`](https://creativecommons.org/share-your-work/public-domain/cc0/)

# Scafalra

command-line interface tool for manage scaffolding

## Installation

### Using Cargo

```bash
cargo install scafalra
```

### Using binrary releases

Download the [latest release binary](https://github.com/shixinhuang99/scafalra/releases) for your system

## Prepare

A GitHub personal access token(classic), which does not require any scope for public repositories
and `repo` scope for private repositories

```bash
scafalra token your_token
```

## Usage

```
scafalra is a command-line interface tool for manage scaffold

Usage: scafalra [OPTIONS] [COMMAND]

Commands:
  list    List all scaffolds
  remove  Remove specified scaffolds
  mv      Rename a scaffold
  add     Add scaffolds from GitHub repository
  create  Copy the scaffold folder to the specified directory
  token   Configure or display your GitHub personal access token(classic)
  help    Print this message or the help of the given subcommand(s)

Options:
      --debug          Use debug output
      --token <TOKEN>  Specify the GitHub personal access token(classic)
      --root-dir       Display root dir of scafalra
  -h, --help           Print help
  -V, --version        Print version
```

### proxy support

```bash
# linux/macos
export https_proxy=your_proxy
scafalra add user/repo
```

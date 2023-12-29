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

scafalra requires a PAT(personal access token) to be configured, usually it doesn't require any permissions, but for private repositories it requires a bit of permissions, either `All repositories` or `Only select repositories` for fine-grained PAT, or `repo` scope for classic PAT.

```bash
scafalra token your_token
```

## Usage

```
scafalra is a command-line interface tool for manage scaffold

Usage: scafalra [OPTIONS] [COMMAND]

Commands:
  list       List all scaffolds
  remove     Remove specified scaffolds
  mv         Rename a scaffold
  add        Add scaffolds from GitHub repository
  create     Copy the scaffold folder to the specified directory
  token      Configure or display your GitHub personal access token
  help       Print this message or the help of the given subcommand(s)

Options:
      --debug          Use debug output
      --token <TOKEN>  Specify the GitHub personal access token
      --proj-dir       Display of scafalra's data storage location
  -h, --help           Print help
  -V, --version        Print version
```

### proxy support

```bash
# linux/macos
export https_proxy=your_proxy
scafalra add user/repo
```

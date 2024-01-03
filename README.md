# Scafalra

`scafalra` is a command-line interface tool for manage templates

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
scafalra is a command-line interface tool for manage templates

Usage: scafalra [OPTIONS] [COMMAND]

Commands:
  list    List all templates
  remove  Remove specified templates
  mv      Rename a template
  add     Add templates from GitHub repository
  create  Copy the template folder to the specified directory
  token   Configure or display your GitHub personal access token
  help    Print this message or the help of the given subcommand(s)

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

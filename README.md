# Scafalra(sca)

scafalra is a command-line interface tool for manage templates

## Installation

### Using Cargo

```sh
cargo install scafalra
```

### Using binrary releases

Download the [latest release binary](https://github.com/shixinhuang99/scafalra/releases) for your system

## Token

Scafalra is based on the GitHub api and doesn't force the need for authentication, but if you need higher rate limiting or want to access to private repositories, consider using PAT(personal access token)

```sh
sca token your_token
```

see more info:

<https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api?apiVersion=2022-11-28>

<https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens>

## Usage

```
scafalra(sca) is a command-line interface tool for manage templates

Usage: sca [OPTIONS] [COMMAND]

Commands:
  list    List all templates
  remove  Remove specified templates [aliases: rm]
  rename  Rename a template [aliases: mv]
  add     Add template from GitHub repository
  create  Copy the template folder to the specified directory
  token   Configure or display your GitHub personal access token
  help    Print this message or the help of the given subcommand(s)

Options:
      --debug          Use debug output
      --token <TOKEN>  Specify the GitHub personal access token
      --proj-dir       Display of scafalra's data storage location
  -i, --interactive    Interactive mode
  -h, --help           Print help
  -V, --version        Print version
```

## Example

### Basic

```sh
sca add user/repo

# GitHub url
sca add https://github.com/user/repo.git

# subdir
sca add user/repo --subdir /path/to/dir

# branch
sca add user/repo --branch branch

# tag
sca add user/repo --tag tag

# commit
sca add user/repo --commit e763a43519ea4c209df2452c6e2a5b7dffdfdd3d
```

```sh
sca create repo
```

### Interactive

`create`, `remove`, `rename` can be used in interactive mode

```
? Select a template:
> bar
  baz
[↑↓ to move, enter to select, type to filter]
```

### Sub template

All folders in the `.scafalra` folder in the template root directory are considered as sub-templates, and you can select some of them to create together when using the `create` command

if a template `foo` looks like the following:

```
.
├── dir
│   └── file.txt
└── .scafalra
    ├── dir-1
    └── dir-2
```

```sh
sca create foo -s dir-1 -s dir-2
```

The created template is as follows:

```
.
├── dir
│   └── file.txt
├── dir-1
└── dir-2
```

### Proxy support

```sh
# linux/macos
export https_proxy=your_proxy
sca add user/repo
```

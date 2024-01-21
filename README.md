# Scafalra(sca)

scafalra is a command-line interface tool for manage templates

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
sca token your_token
```

## Usage

```
scafalra(sca) is a command-line interface tool for manage templates

Usage: sca [OPTIONS] [COMMAND]

Commands:
  list    List all templates
  remove  Remove specified templates
  mv      Rename a template
  add     Add template from GitHub repository
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

## Example

### Basic

```bash
sca add user/repo

# subdir
sca add user/repo/path/to/dir

# GitHub url
sca add https://github.com/user/repo.git

# branch
sca add user/repo --branch branch

# tag
sca add user/repo --tag tag

# commit
sca add user/repo --commit e763a43519ea4c209df2452c6e2a5b7dffdfdd3d
```

```bash
sca create repo
```

### Proxy support

```bash
# linux/macos
export https_proxy=your_proxy
sca add user/repo
```

### Repository config

if a repository `foo/bar` looks like the following:

```
.
├── a
│   ├── a1
│   │   └── a1.txt
│   ├── a2
│   │   └── a2.txt
│   └── a3
│       └── a3.txt
└── .scafalra
    ├── common.txt
    ├── copy-all-in-dir
    │   ├── copy-all-in-dir-2
    │   │   └── copy-all-in-dir-2.txt
    │   └── copy-all-in-dir.txt
    ├── copy-dir
    │   ├── copy-dir.txt
    │   └── cpoy-dir-2
    │       └── copy-dir-2.txt
    ├── scafalra.json
    └── shared-a
        └── shared-a.txt
```

And the configuration file looks like this:

```json
{
  "copyOnAdd": {
    "a": ["common.txt", "copy-dir", "copy-all-in-dir/**", "shared-a"]
  }
}
```

With the `sca add foo/bar --depth 1` command, the local cache will look like the following:

```
├── foo
│   └── bar
│       ├── a
│       │   ├── a1
│       │   │   └── a1.txt
│       │   ├── a2
│       │   │   └── a2.txt
│       │   ├── a3
│       │   │   └── a3.txt
│       │   ├── common.txt
│       │   ├── copy-all-in-dir-2
│       │   │   └── copy-all-in-dir-2.txt
│       │   ├── copy-all-in-dir.txt
│       │   └── copy-dir
│       │       ├── copy-dir.txt
│       │       └── cpoy-dir-2
│       │           └── copy-dir-2.txt
│       └── .scafalra
│           ├── common.txt
│           ├── copy-all-in-dir
│           │   ├── copy-all-in-dir-2
│           │   │   └── copy-all-in-dir-2.txt
│           │   └── copy-all-in-dir.txt
│           ├── copy-dir
│           │   ├── copy-dir.txt
│           │   └── cpoy-dir-2
│           │       └── copy-dir-2.txt
│           ├── scafalra.json
│           └── shared-a
│               └── shared-a.txt
```

### The `--with` parameter of the `create` command

The local cache is as follows:

```
├── foo
│   └── bar
│       ├── a
│       │   ├── a1
│       │   │   └── a1.txt
│       │   ├── a2
│       │   │   └── a2.txt
│       │   └── a3
│       │       └── a3.txt
│       └── .scafalra
│           ├── common.txt
│           ├── copy-all-in-dir
│           │   ├── copy-all-in-dir-2
│           │   │   └── copy-all-in-dir-2.txt
│           │   └── copy-all-in-dir.txt
│           ├── copy-dir
│           │   ├── copy-dir.txt
│           │   └── cpoy-dir-2
│           │       └── copy-dir-2.txt
│           ├── scafalra.json
│           └── shared-a
│               └── shared-a.txt
```

```bash
sca create a --with "common.txt,copy-dir,copy-all-in-dir/**"
```

The created project will look like this:

```
├── a
│   ├── a1
│   │   └── a1.txt
│   ├── a2
│   │   └── a2.txt
│   ├── a3
│   │   └── a3.txt
│   ├── common.txt
│   ├── copy-all-in-dir-2
│   │   └── copy-all-in-dir-2.txt
│   ├── copy-all-in-dir.txt
│   └── copy-dir
│       ├── copy-dir.txt
│       └── cpoy-dir-2
│           └── copy-dir-2.txt
```

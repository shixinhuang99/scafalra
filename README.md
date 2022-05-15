# Scaffold Cli - scaffold commannd line tool

This command line tool manages the paths to the project template folders and copies them to specific paths when needed using simple commands.

> _note: require Node 16 or above_

## Installation

```bash
npm install -g @zerowong/scaffold-cli
```

## Example

Suppose we have a folder with templates for different types of projects.

```
projects
├── nodejs
├── react
└── vue
```

We can then use `scaffold add ./projects --depth 1` to add them all to the list.

outputs:

```
+ nodejs    /.../projects/nodejs
+ vue       /.../projects/vue
+ react     /.../projects/react
```

When we want to create a new project based on one of these templates, we can use `scaffold create react ./new-awesome-project` to quickly copy the template.

outputs:

```
INFO: Project created in '/.../new-awesome-project'.
```

Since `v0.2.5`, the `add` command supports adding GitHub repositories.

For example: `scaffold add https://github.com/zerowong/scaffold-cli.git`

outputs:

```
+ scaffold-cli    /.../.scaffold-cli/cache/scaffold-cli
```

The above example will download and extract the archive of the last commit of `scaffold-cli` to the cache directory, then add its local path to the list.

This is much quicker than using `git clone`, because it not downloading the entire git history.

## Usage

### `scaffold [-h|--help] [-v|--version]`

Display help.

```bash
scaffold -h
```

Display version.

```bash
scaffold -v
```

### `scaffold list [-p|--prune]`

List all projects.

```bash
scaffold list
```

Clear the path of items that no longer exist and list them.

```bash
scaffold list --prune
```

### `scaffold add <path ...> [-d|--depth <0|1>]`

Add projects with path of a local folder.

```bash
scaffold add ./path/to/foo ./path/to/bar...
```

The depth defaults to 0, which means that the `add` command treats this folder as one project, and if the depth is 1 then all subfolders under this folder are treated as multiple projects.

```bash
scaffold add ./path/to/projects --depth 1
```

Add projects with url of GitHub repository.

> _note: Using this feature requires that `git` has been installed, because `git ls-remote` is used internally._

```bash
scaffold add https://github.com/user/repo.git
```

HTTPS proxy

```bash
export https_proxy=your_proxy
scaffold add https://github.com/user/repo.git
```

### `scaffold remove <name ...>`

Remove projects from list.

```bash
scaffold remove foo bar baz...
```

### `scaffold create <name> [<directory>] [-o|--overwrite]`

Copy the templates folder to the current working directory.

> note: `DS_Store`, `node_modules`and`.git` folders will be ignored when copying the project

```bash
scaffold create foo
```

Copy the templates folder to the specified path.

```bash
scaffold create foo ./path/to/bar
```

The specified path can be overwritten with `--overwrite` if it already exists.

> note: try not to use it unless you are aware of the risks.

```bash
scaffold create foo ./path/to/bar --overwrite
```

If the project references a GitHub repository, it will check its latest commit hash when it is created, and if the hash changes, it will be re-downloaded and cached

```bash
scaffold create remote-repo
```

### `scaffold mv <oldName> <newName>`

Rename a project.

```bash
scaffold mv foo bar
```

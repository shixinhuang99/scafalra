# Scafalra - command-line interface tool for manage scaffolding

`scaffold-cli` is a command-line interface tool for managing and creating scaffold projects via GitHub repository.

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

In addition to local paths, the `add` command also supports adding GitHub repositories.

For example: `scaffold add https://github.com/zerowong/scaffold-cli.git`

outputs:

```
+ scaffold-cli    /.../.scaffold-cli/cache/scaffold-cli
```

The above example will download and extract the archive of the last commit of `scaffold-cli` to the cache directory, then add its local path to the list.

This is much quicker than using `git clone`, because it not downloading the entire git history.

When we want to create a new project based on one of these templates, we can use `scaffold create projectName ./new-awesome-project` to quickly copy the template.

outputs:

```
INFO: Project created in '/.../new-awesome-project'.
```

The `create` command also supports one-time creation of projects from local paths or GitHub URL, which is useful when using tools like `npx` or `dlx`.

```bash
npx scaffold create https://github.com/zerowong/scaffold-cli.git /path/to/somewhere
```

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

### `scaffold add <path|url ...> [-d|--depth <0|1>]`

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

### `scaffold create <name|path|url> [<directory>] [-o|--overwrite]`

Create a project to the current working directory.

> note: The `DS_Store`, `node_modules` and `.git` folders will be ignored when creating projects from the local path or project list.

```bash
scaffold create foo
```

Create a project to the specified path.

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

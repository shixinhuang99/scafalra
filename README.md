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

We can then use `scaffold-cli add ./projects --depth 1` to add them all to the list.

```
New projects:

+ nodejs    /.../projects/nodejs
+ vue       /.../projects/vue
+ react     /.../projects/react
```

When we want to create a new project based on one of these templates, we can use `scaffold-cli create react ./new-awesome-project` to quickly copy the template.

```
Project created in '/.../new-awesome-project'.
```

## Usage

### `scaffold-cli [-h|--help] [-v|--version]`

Display help.

```bash
scaffold-cli -h
```

Display version.

```bash
scaffold-cli -v
```

### `scaffold-cli list [-p|--prune]`

List all projects.

```bash
scaffold-cli list
```

Clear the path of items that no longer exist and list them.

```bash
scaffold-cli list --purge
```

### `scaffold-cli add <path ...> [-d|--depth <0|1>]`

Add projects with path of a local folder.

```bash
scaffold-cli add ./path/to/foo ./path/to/bar...
```

The depth defaults to 0, which means that the `add` command treats this folder as one project, and if the depth is 1 then all subfolders under this folder are treated as multiple projects.

```bash
scaffold-cli add ./path/to/projects --depth 1
```

### `scaffold-cli remove <name ...>`

Remove projects from list.

```bash
scaffold-cli remove foo bar baz...
```

### `scaffold-cli create <name> [<directory>] [-o|--overwrite]`

Copy the templates folder to the current working directory.

> note: `DS_Store`, `node_modules`and`.git` folders will be ignored when copying the project

```bash
scaffold-cli create foo
```

Copy the templates folder to the specified path.

```bash
scaffold-cli create foo ./path/to/bar
```

The specified path can be overwritten with `-overwrite` if it already exists.

> note: try not to use it unless you are aware of the risks.

```bash
scaffold-cli create foo ./path/to/bar --overwrite
```

## Todo

- [ ] support github repo

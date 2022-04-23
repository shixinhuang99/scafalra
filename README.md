<h1 align="center">Scaffold Cli</h1>

<h2 align="center">scaffold commannd line tool</h2>

_note: require Node 16 or above_

## Installation

```bash
npm install -g @zerowong/scaffold-cli
```

## Usage

### `scaffold-cli [-h|--help] [-v|--version]`

Get help

```bash
scaffold-cli -h
```

Get version

```bash
scaffold-cli -v
```

---

### `scaffold-cli list [-p|--prune]`

List all projects.

```bash
scaffold-cli list
```

Clear path invalid project and list all projects.

```bash
scaffold-cli list --purge
```

---

### `scaffold-cli add <path ...> [-d|--depth <0|1>]`

Add projects with path of a local folder.

```bash
scaffold-cli add './path/to/foo'
```

The depth defaults to 0, which means that the `add` command treats this folder as one project, and if the depth is 1 then all subfolders under this folder are treated as multiple projects.

```bash
scaffold-cli add './path/to/projects' --depth 1
```

---

### `scaffold-cli remove <name ...>`

Remove projects.

```bash
scaffold-cli remove foo
```

---

### `scaffold-cli create <name> [<directory>] [-o|--overwrite]`

Create a project from list.

```bash
scaffold-cli create foo bar
```

Force overwrite target folder.

```bash
scaffold-cli create foo bar --overwrite
```

## Todo

- [] support remote repo

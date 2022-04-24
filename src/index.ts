import os from 'node:os'
import path from 'node:path'
import fs from 'node:fs/promises'
import mri from 'mri'
import chalk from 'chalk'

interface SystemError extends Error {
  code: string
  syscall: string
  path: string
}

function isSystemError(err: unknown): err is SystemError {
  return err instanceof Error && 'syscall' in err
}

function isENOENT(err: unknown): err is SystemError {
  return isSystemError(err) && err.code === 'ENOENT'
}

function isEEXIST(err: unknown): err is SystemError {
  return isSystemError(err) && err.code === 'EEXIST'
}

const log = {
  error(msg: string) {
    console.error(`${chalk.red('scaffold')}: ${msg}`)
  },
  usage(msg: string) {
    console.error(`${chalk.cyan('usage')}: ${msg}`)
  },
  grid(msgs: [string, string][], space = 4) {
    let max = 0
    for (let i = 0, l = msgs.length; i < l; i++) {
      max = Math.max(msgs[i][0].length, max)
    }
    let res = ''
    for (let i = 0, l = msgs.length; i < l; i++) {
      const left = msgs[i][0]
      res += `${left}${' '.repeat(max - left.length + space)}${msgs[i][1]}${
        i === l - 1 ? '' : '\n'
      }`
    }
    console.log(res)
  },
}

function rmrf(target: string) {
  return fs.rm(target, { force: true, recursive: true })
}

// fsPromises.cp is experimental
async function cp(source: string, target: string) {
  const ignore = ['.git', '.DS_Store', 'node_modules']
  const sourceDir = await fs.opendir(source)
  await fs.mkdir(target)
  for await (const dirent of sourceDir) {
    if (ignore.includes(dirent.name)) {
      continue
    }
    const s = path.join(source, dirent.name)
    const t = path.join(target, dirent.name)
    if (dirent.isDirectory()) {
      await cp(s, t)
    } else if (dirent.isFile()) {
      await fs.copyFile(s, t)
    }
  }
}

function hasInvalidFlag(allows: string[], input: string[]) {
  if (input.length === 0) {
    return false
  }
  return allows.some((item) => !input.includes(item))
}

const base = os.homedir() ?? os.tmpdir()
const configFile =
  process.env.NODE_ENV === 'test' ? '.scaffold-cli-test.json' : '.scaffold-cli.json'
const configPath = path.join(base, configFile)
const cwd = process.cwd()

class ScaffoldCli {
  private version: string
  private config: {
    projects: Record<string, string>
  }
  private changes: Record<string, string>

  constructor() {
    this.version = '0.1.3'
    this.config = {
      projects: {},
    }
    this.changes = {}
  }

  private async readConfig() {
    try {
      const content = await fs.readFile(configPath, { encoding: 'utf8' })
      this.config = JSON.parse(content)
    } catch (error) {
      if (isENOENT(error)) {
        await this.writeConfig()
      }
    }
  }

  private async writeConfig() {
    const content = JSON.stringify(this.config, null, 2)
    await fs.writeFile(configPath, content)
  }

  private addProject(name: string, absPath: string) {
    this.config.projects[name] = absPath
    this.changes[`${chalk.green('+')} ${name}`] = absPath
  }

  private removeProject(name: string) {
    this.changes[`${chalk.red('-')} ${name}`] = this.config.projects[name]
    delete this.config.projects[name]
  }

  private logChanges() {
    log.grid(Object.entries(this.changes))
  }

  private none(flag?: string) {
    if (!flag) {
      return log.usage('scaffold [-h|--help] [-v|--version]')
    }
    if (flag === 'h') {
      log.grid(
        [
          ['scaffold', '[-h|--help] [-v|--version]'],
          ['', '<command> [<flags>]'],
        ],
        1
      )
      console.log('\nAvailable commands are as follows:\n')
      log.grid([
        ['list [-p|--prune]', 'List all projects.'],
        [
          'add <path ...> [-d|--depth <0|1>]',
          'Add projects with path of a local folder.',
        ],
        ['remove <name ...>', 'Remove projects from list.'],
        [
          'create <name> [<directory>] [-o|--overwrite]',
          'Create a project by copying the templates folder.',
        ],
      ])
    }
    if (flag === 'v') {
      console.log(this.version)
    }
  }

  private async list(prune = false) {
    if (typeof prune !== 'boolean') {
      return
    }
    const entries = Object.entries(this.config.projects)
    if (prune) {
      for (const [name, value] of entries) {
        try {
          await fs.access(value)
        } catch (error) {
          delete this.config.projects[name]
        }
      }
    }
    log.grid(
      Object.entries(this.config.projects).map((item) => {
        return [chalk.green(item[0]), item[1]]
      })
    )
  }

  private async add(paths: string[], depth = 0) {
    if (paths.length === 0 || typeof depth !== 'number' || depth === -1) {
      return log.usage('scaffold add <path ...> [-d|--depth <0|1>]')
    }
    for (const item of paths) {
      const absPath = path.isAbsolute(item) ? item : path.resolve(cwd, item)
      try {
        if (depth === 0) {
          const target = await fs.stat(absPath)
          if (!target.isDirectory()) {
            return log.error(`'${absPath}' is not a directory.`)
          }
          this.addProject(path.basename(absPath), absPath)
        } else if (depth === 1) {
          const dir = await fs.opendir(absPath)
          for await (const dirent of dir) {
            if (dirent.isDirectory() && dirent.name[0] !== '.') {
              this.addProject(dirent.name, path.join(absPath, dirent.name))
            }
          }
        }
      } catch (error) {
        if (isENOENT(error)) {
          return log.error(`Can't find directory '${error.path}'.`)
        }
      }
    }
    await this.writeConfig()
    console.log('New projects:\n')
    this.logChanges()
  }

  private async remove(names: string[]) {
    if (names.length === 0) {
      return log.usage('scaffold remove <name ...>')
    }
    for (const name of names) {
      if (!(name in this.config.projects)) {
        return log.error(`No such project: '${name}'.`)
      }
      this.removeProject(name)
    }
    await this.writeConfig()
    console.log('Removed projects:\n')
    this.logChanges()
  }

  private async create(name?: string, directory?: string, overwrite = false) {
    if (!name) {
      return log.usage('scaffold create <name> [<directory>] [-o|--overwrite]')
    }
    const source = this.config.projects[name]
    const target = path.resolve(cwd, directory ?? name)
    if (!source) {
      return log.error(`Can't find project '${name}'.`)
    }
    if (target === source) {
      return log.error(`Source path and target paths cannot be the same.`)
    }
    if (overwrite) {
      await rmrf(target)
    }
    try {
      await cp(source, target)
      console.log(`Project created in '${target}'.`)
    } catch (error) {
      if (isEEXIST(error)) {
        log.error(`Directory '${error.path}' already exists.`)
      }
      if (isENOENT(error)) {
        log.error(`Can't find directory '${error.path}'.`)
      }
    }
  }

  async main(argv?: string[]) {
    await this.readConfig()
    const _argv = mri(argv ?? process.argv.slice(2), {
      alias: {
        d: 'depth',
        h: 'help',
        v: 'version',
        o: 'overwrite',
        p: 'prune',
      },
      unknown(flag) {
        log.error(`'${flag}' is not a valid flag. See 'scaffold --help'.`)
      },
    })

    if (!_argv) {
      return
    }

    const {
      _: [action = '', ...args],
      ...flags
    } = _argv

    const flagArr = Object.keys(flags)
    const firstFlag = flagArr[0]?.[0]

    switch (action) {
      case '': {
        this.none(firstFlag)
        break
      }
      case 'list': {
        if (hasInvalidFlag(['p', 'prune'], flagArr)) {
          return log.usage('scaffold list [-p|--prune]')
        }
        this.list(flags.p)
        break
      }
      case 'add': {
        const depth = firstFlag ? (firstFlag === 'd' ? flags[firstFlag] : -1) : 0
        this.add(args, depth)
        break
      }
      case 'remove': {
        this.remove(args)
        break
      }
      case 'create': {
        const overwrite = firstFlag === 'o'
        this.create(args[0], args[1], overwrite)
        break
      }
      default: {
        log.error(`'${action}' is not a valid command. See 'scaffold --help'.`)
        break
      }
    }
  }
}

export default ScaffoldCli

#!/usr/bin/env node
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
    console.error(`${chalk.red('scaffold-cli')}: ${msg}`)
  },
  usage(msg: string) {
    console.log(`${chalk.blue('usage')}: ${msg}`)
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
  try {
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
  } catch (error) {
    if (isEEXIST(error)) {
      log.error(
        `Directory '${error.path}' already exists. Overwrite the directory with '--overwrite'?`
      )
    }
    if (isENOENT(error)) {
      log.error(`Can't find directory '${error.path}'.`)
    }
  }
}

const configPath = path.join(os.homedir() ?? os.tmpdir(), '.scaffold_cli.json')
const cwd = process.cwd()

class ScaffoldCli {
  private version: string
  private config: {
    projects: Record<string, string>
  }
  private changes: [string, string][]

  constructor() {
    this.version = '0.0.1'
    this.config = {
      projects: {},
    }
    this.changes = []
  }

  private addProject(name: string, absPath: string) {
    this.config.projects[name] = absPath
    this.changes.push([name, absPath])
  }

  private removeProject(name: string) {
    this.changes.push([name, this.config.projects[name]])
    delete this.config.projects[name]
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

  private none(flag?: string) {
    if (!flag) {
      return log.usage('scaffold-cli [-h|--help] [-v|--version]')
    }
    if (flag === 'h') {
      log.grid(
        [
          ['scaffold-cli', '[-h|--help] [-v|--version]'],
          ['', '<command> [<flags>]'],
        ],
        1
      )
      console.log('\nAvailable commands are as follows:\n')
      log.grid([
        ['list', 'List all projects.'],
        [
          'add <path ...> [-d|--depth <0|1>]',
          'Add projects with path of a local folder.',
        ],
        ['remove <name ...>', 'Remove projects.'],
        ['create <name> [<directory>] [-o|--overwrite]', 'Create a project from list.'],
      ])
    }
    if (flag === 'v') {
      console.log(this.version)
    }
  }

  /**
   * @todo purge
   */
  private list() {
    log.grid(Object.entries(this.config.projects))
  }

  private async add(paths: string[], depth = 0) {
    if (paths.length === 0 || typeof depth !== 'number' || depth === -1) {
      return log.usage('scaffold-cli add <path ...> [-d|--depth <0|1>]')
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
    log.grid(this.changes)
  }

  private async remove(names: string[]) {
    if (names.length === 0) {
      return log.usage('scaffold-cli remove <name ...>')
    }
    for (const name of names) {
      this.removeProject(name)
    }
    await this.writeConfig()
    console.log('Removed projects:\n')
    log.grid(this.changes)
  }

  private async create(name?: string, directory?: string, overwrite = false) {
    if (!name) {
      return log.usage('scaffold-cli create <name> [<directory>] [-o|--overwrite]')
    }
    const source = this.config.projects[name]
    const target = path.resolve(cwd, directory ?? name)
    if (!source) {
      return log.error(`Can't find project '${name}'.`)
    }
    if (overwrite) {
      await rmrf(target)
    }
    await cp(source, target)
    console.log(`Project created.`)
    if (target !== cwd) {
      console.log('\nNow run:\n')
      console.log(`  cd ${path.relative(cwd, target)}\n`)
    }
  }

  async main() {
    await this.readConfig()
    const argv = mri(process.argv.slice(2), {
      alias: {
        d: 'depth',
        h: 'help',
        v: 'version',
        o: 'overwrite',
      },
      unknown(flag) {
        log.error(`'${flag}' is not a valid flag. See 'scaffold-cli --help'.`)
      },
    })

    if (!argv) {
      return
    }

    const {
      _: [action = '', ...args],
      ...flags
    } = argv

    const flagArr = Object.keys(flags)
    const firstFlag = flagArr[0]?.[0]

    switch (action) {
      case '': {
        this.none(firstFlag)
        break
      }
      case 'list': {
        this.list()
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
        log.error(`'${action}' is not a valid command. See 'scaffold-cli --help'.`)
        break
      }
    }
  }
}

const cli = new ScaffoldCli()

cli.main()

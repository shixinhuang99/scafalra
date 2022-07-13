import os from 'node:os'
import path from 'node:path'
import fsp from 'node:fs/promises'
import chalk from 'chalk'
import {
  exception,
  log,
  rmrf,
  cp,
  exists,
  parse,
  argvParser,
  isURL,
  Key,
  hasOwn,
  uniq,
  type Repo,
  fetchRepo,
} from './utils.js'

export interface Project {
  path: string
  remote?: string
  hash?: string
}

const cwd = process.cwd()
const key = new Key()

export class ScaffoldCli {
  private configDir = path.join(
    os.homedir() ?? os.tmpdir(),
    process.env.NODE_ENV === 'test' ? '.scaffold-cli-test' : '.scaffold-cli'
  )
  private storeFile = path.join(this.configDir, 'store.json')
  private cacheDir = path.join(this.configDir, 'cache')
  private store: Record<string, Project> = {}
  private changes: Record<string, Project> = {}

  private async init() {
    if (await exists(this.configDir)) {
      const raw = await fsp.readFile(this.storeFile, { encoding: 'utf8' })
      this.store = JSON.parse(raw)
    } else {
      await fsp.mkdir(this.configDir)
      await fsp.mkdir(this.cacheDir)
      await this.save()
    }
  }

  private async save() {
    await fsp.writeFile(this.storeFile, JSON.stringify(this.store, null, 2))
  }

  private addProject(
    name: string,
    proj: Project,
    options: { replace?: boolean } = { replace: false }
  ) {
    const realName =
      !options.replace && hasOwn(this.store, name) ? `${name}-${key.gen(name)}` : name
    this.store[realName] = proj
    this.changes[`${chalk.green('+')} ${realName}`] = proj
  }

  private async removeProject(name: string) {
    const proj = this.store[name]
    if (!proj) {
      throw new Error(`No such project: '${name}'.`)
    }
    if (proj.remote) {
      await rmrf(proj.path)
    }
    this.changes[`${chalk.red('-')} ${name}`] = this.store[name]
    delete this.store[name]
  }

  private logChanges() {
    log.grid(
      Object.entries(this.changes).map(([name, proj]) => {
        return [name, proj.path]
      })
    )
  }

  private cacheRepo(repo: Repo) {
    return fetchRepo(this.cacheDir, repo)
  }

  private async none(flags: { h?: boolean; v?: boolean }) {
    if (flags.h || Object.keys(flags).length === 0) {
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
          'add <path|url ...> [-d|--depth <0|1>]',
          'Add projects with path of a local folder.',
        ],
        ['remove <name ...>', 'Remove projects from list.'],
        [
          'create <name|path|url> [<directory>] [-o|--overwrite]',
          'Create a project by copying the templates folder.',
        ],
        ['mv <oldName> <newName>', 'Rename a project.'],
      ])
    }
    if (flags.v) {
      const raw = await fsp.readFile(new URL('../package.json', import.meta.url), {
        encoding: 'utf8',
      })
      const pkg = JSON.parse(raw)
      console.log(pkg.version)
    }
  }

  private async list(prune: boolean) {
    if (prune) {
      await Promise.all(
        Object.entries(this.store).map(async ([name, proj]) => {
          if (!(await exists(proj.path))) {
            delete this.store[name]
          }
        })
      )
      await this.save()
    }
    log.grid(
      Object.entries(this.store).map(([name, proj]) => {
        return [chalk.green(name), proj.path]
      })
    )
  }

  private async addLocal(src: string, depth = 0) {
    const absPath = path.isAbsolute(src) ? src : path.resolve(cwd, src)
    if (depth === 0) {
      if (!(await fsp.stat(absPath)).isDirectory()) {
        throw new Error(`'${absPath}' is not a directory.`)
      }
      this.addProject(path.basename(absPath), { path: absPath })
    } else if (depth === 1) {
      const dir = await fsp.readdir(absPath, { withFileTypes: true })
      for (const dirent of dir) {
        if (dirent.isDirectory() && dirent.name[0] !== '.') {
          this.addProject(dirent.name, { path: path.join(absPath, dirent.name) })
        }
      }
    }
  }

  private async addRemote(src: string, depth = 0) {
    const repo = await parse(src)
    const repoDir = await this.cacheRepo(repo)
    if (depth === 0) {
      this.addProject(repo.name, { path: repoDir, remote: src, hash: repo.hash })
    } else if (depth === 1) {
      const dir = await fsp.readdir(repoDir, { withFileTypes: true })
      for (const dirent of dir) {
        if (dirent.isDirectory() && dirent.name[0] !== '.') {
          this.addProject(dirent.name, {
            path: path.join(repoDir, dirent.name),
            remote: src,
            hash: repo.hash,
          })
        }
      }
    }
  }

  private async add(paths: string[], depth = 0) {
    const { locals, remotes } = paths.reduce<{ locals: string[]; remotes: string[] }>(
      (prev, curr) => {
        if (isURL(curr)) {
          prev.remotes.push(curr)
        } else {
          prev.locals.push(curr)
        }
        return prev
      },
      { locals: [], remotes: [] }
    )
    const result: PromiseSettledResult<void>[] = []
    result.push(
      ...(await Promise.allSettled(locals.map((src) => this.addLocal(src, depth))))
    )
    if (remotes.length > 0) {
      log.write('Downloading...')
      result.push(
        ...(await Promise.allSettled(remotes.map((src) => this.addRemote(src, depth))))
      )
      log.clear()
    }
    await this.save()
    this.logChanges()
    log.result(result)
  }

  private async remove(names: string[]) {
    const result = await Promise.allSettled(
      names.map((name) => {
        return this.removeProject(name)
      })
    )
    await this.save()
    this.logChanges()
    log.result(result)
  }

  private async create(src: string, directory?: string, overwrite = false) {
    if (isURL(src)) {
      const repo = await parse(src)
      const parentDir = directory ? path.resolve(cwd, directory) : cwd
      const directoryPath = path.join(parentDir, repo.name)
      if (!overwrite && (await exists(directoryPath))) {
        return log.error(`Directory '${directoryPath}' already exists.`)
      }
      log.write('Downloading...')
      try {
        const fullPath = await fetchRepo(parentDir, repo)
        log.clear()
        return log.info(`Project created in '${fullPath}'.`)
      } catch (err) {
        log.clear()
        if (err instanceof Error) {
          return log.error(err.message)
        }
        throw err
      }
    }
    let source = ''
    const proj = this.store[src]
    if (!proj) {
      const absPath = path.isAbsolute(src) ? src : path.resolve(cwd, src)
      try {
        if (!(await fsp.stat(absPath)).isDirectory()) {
          return log.error(`'${absPath}' is not a directory.`)
        }
        source = absPath
      } catch (err) {
        if (exception.isENOENT(err)) {
          return log.error(`Can't find directory '${err.path}'.`)
        }
        throw err
      }
    } else {
      if (proj.remote) {
        const repo = await parse(proj.remote).catch(() => null)
        if (repo && repo.hash !== proj.hash) {
          log.write('The cache needs to be updated, downloading...')
          await this.cacheRepo(repo)
          this.addProject(
            src,
            {
              path: proj.path,
              remote: proj.remote,
              hash: repo.hash,
            },
            { replace: true }
          )
          await this.save()
          log.clear()
        }
      }
      source = proj.path
    }
    if (!source) {
      return log.error('Unknonw source.')
    }
    const target = path.resolve(cwd, directory ?? src)
    if (target === source) {
      return log.error(`Source path and target paths cannot be the same.`)
    }
    if (overwrite) {
      await rmrf(target)
    }
    try {
      await cp(source, target)
      log.info(`Project created in '${target}'.`)
    } catch (err) {
      if (exception.isEEXIST(err)) {
        return log.error(`Directory '${err.path}' already exists.`)
      }
      if (exception.isENOENT(err)) {
        return log.error(`Can't find directory '${err.path}'.`)
      }
      throw err
    }
  }

  private async mv(oldName: string, newName: string) {
    if (!hasOwn(this.store, oldName) || oldName === newName) {
      return
    }
    if (hasOwn(this.store, newName)) {
      return log.error(`'${newName}' already exists.`)
    }
    this.store[newName] = this.store[oldName]
    delete this.store[oldName]
    await this.save()
  }

  async main() {
    await this.init()
    const argv = argvParser()

    if (!argv) {
      return
    }

    const { action, args, flags, checker } = argv

    switch (action) {
      case '': {
        if (checker(['v', 'h'])) {
          return log.usage('scaffold [-h|--help] [-v|--version]')
        }
        await this.none(flags)
        break
      }
      case 'list': {
        if (checker(['p'])) {
          return log.usage('scaffold list [-p|--prune]')
        }
        await this.list(flags.p)
        break
      }
      case 'add': {
        if (args.length === 0 || checker({ flag: 'd', options: [0, 1] })) {
          return log.usage('scaffold add <path|url ...> [-d|--depth <0|1>]')
        }
        await this.add(uniq(args), flags.d)
        break
      }
      case 'remove': {
        if (args.length === 0) {
          return log.usage('scaffold remove <name ...>')
        }
        await this.remove(uniq(args))
        break
      }
      case 'create': {
        if (args.length < 1 || checker(['o'])) {
          return log.usage(
            'scaffold create <name|path|url> [<directory>] [-o|--overwrite]'
          )
        }
        await this.create(args[0], args[1], flags.o)
        break
      }
      case 'mv': {
        if (args.length !== 2) {
          return log.usage('scaffold mv <oldName> <newName>')
        }
        await this.mv(args[0], args[1])
        break
      }
      default: {
        log.error(`'${action}' is not a valid command. See 'scaffold --help'.`)
        break
      }
    }
  }
}

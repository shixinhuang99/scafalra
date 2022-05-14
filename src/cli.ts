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
  fetchHeadHash,
  joinArchiveUrl,
  download,
  unzip,
  argsParser,
  isURL,
  Key,
  hasOwn,
  uniq,
  logTaskResult,
} from './utils'

export interface Project {
  path: string
  remote?: string
  hash?: string
}

const cwd = process.cwd()
const key = new Key()

export default class ScaffoldCli {
  private configDir = path.join(
    os.homedir(),
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

  private addProject(name: string, proj: Project) {
    const realName = hasOwn(this.store, name) ? `${name}-${key.gen(name)}` : name
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
    if (flags.v) {
      const raw = await fsp.readFile(path.join(__dirname, '../package.json'), {
        encoding: 'utf8',
      })
      const pkg = JSON.parse(raw)
      console.log(pkg.version)
    }
  }

  private async list(prune: boolean) {
    const entries = Object.entries(this.store)
    if (prune) {
      for (const [name, proj] of entries) {
        if (!(await exists(proj.path))) {
          delete this.store[name]
        }
      }
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
      const target = await fsp.stat(absPath)
      if (!target.isDirectory()) {
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
    const repo = parse(src)
    const hash = await fetchHeadHash(src)
    const url = joinArchiveUrl(hash, repo)
    const archiveFile = path.join(this.cacheDir, `${repo.name}-${hash}.zip`)
    await download(url, archiveFile, { proxy: process.env.https_proxy })
    const unzipedDir = await unzip(archiveFile)
    if (depth === 0) {
      this.addProject(repo.name, { path: unzipedDir, remote: src, hash })
    } else if (depth === 1) {
      const dir = await fsp.readdir(unzipedDir, { withFileTypes: true })
      for (const dirent of dir) {
        if (dirent.isDirectory() && dirent.name[0] !== '.') {
          this.addProject(dirent.name, {
            path: path.join(unzipedDir, dirent.name),
            hash,
            remote: src,
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
    logTaskResult(result)
  }

  private async remove(names: string[]) {
    const result = await Promise.allSettled(
      names.map((name) => {
        return this.removeProject(name)
      })
    )
    await this.save()
    this.logChanges()
    logTaskResult(result)
  }

  private async create(name: string, directory?: string, overwrite = false) {
    const proj = this.store[name]
    if (!proj) {
      return log.error(`Can't find project '${name}'.`)
    }
    if (proj.remote) {
      const newHash = await fetchHeadHash(proj.remote).catch(() => null)
      if (!newHash) {
        log.warn('Could not find commit hash of HEAD, Local cache will be used.')
      } else {
        const repo = parse(proj.remote)
        if (newHash !== proj.hash) {
          log.write('The cache needs to be updated, downloading...')
          const url = joinArchiveUrl(newHash, repo)
          const archiveFile = path.join(this.cacheDir, `${repo.name}-${newHash}.zip`)
          await download(url, archiveFile, { proxy: process.env.https_proxy })
          const unzipedDir = await unzip(archiveFile)
          this.addProject(name, {
            path: unzipedDir,
            remote: proj.remote,
            hash: newHash,
          })
          await this.save()
          log.clear()
        }
      }
    }
    const source = proj.path
    const target = path.resolve(cwd, directory ?? name)
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

  async main() {
    await this.init()
    const argv = argsParser()

    if (!argv) {
      return
    }

    const { action, args, flags, checker } = argv

    switch (action) {
      case '': {
        if (checker(['v', 'h'])) {
          return log.usage('scaffold [-h|--help] [-v|--version]')
        }
        this.none(flags)
        break
      }
      case 'list': {
        if (checker(['p'])) {
          return log.usage('scaffold list [-p|--prune]')
        }
        this.list(flags.p)
        break
      }
      case 'add': {
        if (args.length === 0 || checker({ flag: 'd', options: [0, 1] })) {
          return log.usage('scaffold add <path ...> [-d|--depth <0|1>]')
        }
        this.add(uniq(args), flags.d)
        break
      }
      case 'remove': {
        if (args.length === 0) {
          return log.usage('scaffold remove <name ...>')
        }
        this.remove(uniq(args))
        break
      }
      case 'create': {
        if (args.length < 1 || checker(['o'])) {
          return log.usage('scaffold create <name> [<directory>] [-o|--overwrite]')
        }
        this.create(args[0], args[1], flags.o)
        break
      }
      default: {
        log.error(`'${action}' is not a valid command. See 'scaffold --help'.`)
        break
      }
    }
  }
}

import { join } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, rmSync, readdirSync } from 'node:fs'
import { execFile } from 'node:child_process'
import { homedir } from 'node:os'
import {
  describe,
  test,
  expect,
  beforeAll,
  afterAll,
  beforeEach,
  afterEach,
} from 'vitest'
import type { Project } from '../src/cli'
import { fetchHeadHash } from '../src/utils'

const cwd = process.cwd()
function joinCwd(path: string) {
  return join(cwd, path)
}

const cliPath = joinCwd('bin/index.js')
const configDir = join(homedir(), '.scaffold-cli-test')
const storeFile = join(configDir, 'store.json')
const cacheDir = join(configDir, 'cache')

async function run(command: string, args: string[] = []) {
  return new Promise<{ stdout: string; stderr: string }>((resolve, reject) => {
    execFile('node', [cliPath, command, ...args], (err, stdout, stderr) => {
      if (err) {
        return reject(err)
      }
      return resolve({ stdout: stdout.trimEnd(), stderr: stderr.trimEnd() })
    })
  })
}

const log = {
  error(msg: string) {
    return `ERROR: ${msg}`
  },
  usage(msg: string) {
    return `USAGE: ${msg}`
  },
  info(msg: string) {
    return `INFO: ${msg}`
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
    return res
  },
}

const testSourceDir = 'ZHploHewql'
const testTargetDir = 'rDm1eCgGj2'
function toTestPath(path?: string, source = true) {
  const dir = source ? testSourceDir : testTargetDir
  if (!path) {
    return join(cwd, dir)
  }
  return join(cwd, dir, path)
}

const constants = {
  notExists: 'not-exists',
  isAFile: 'is-a-file.txt',
  addUsage: 'scaffold add <path ...> [-d|--depth <0|1>]',
  testGitHubRepo: 'https://github.com/zerowong/scaffold-cli.git',
  timeout: 10000,
}

function save(store: Record<string, Project> = {}) {
  writeFileSync(storeFile, JSON.stringify(store))
}

function rmrf(target: string) {
  return rmSync(target, { force: true, recursive: true })
}

// .
// ├── .ignore-dir
// ├── 0
// │   ├── .DS_Store
// │   ├── .git
// │   ├── 0
// │   │   ├── 0.txt
// │   │   ├── 1.txt
// │   │   └── 2.txt
// │   ├── 0.txt
// │   ├── 1
// │   │   ├── 0.txt
// │   │   ├── 1.txt
// │   │   └── 2.txt
// │   ├── 1.txt
// │   ├── 2
// │   │   ├── 0.txt
// │   │   ├── 1.txt
// │   │   └── 2.txt
// │   ├── 2.txt
// │   └── node_modules
// ├── 1
// │   ...
// ├── 2
// │   ...
// └── is-a-file.txt
function genTestDir() {
  mkdirSync(toTestPath())
  mkdirSync(toTestPath('.ignore-dir'))
  writeFileSync(toTestPath(constants.isAFile), '')
  for (let i = 0; i < 3; i++) {
    mkdirSync(toTestPath(i.toString()))
    writeFileSync(toTestPath(`${i}/.DS_Store`), '')
    mkdirSync(toTestPath(`${i}/node_modules`))
    mkdirSync(toTestPath(`${i}/.git`))
    for (let j = 0; j < 3; j++) {
      mkdirSync(toTestPath(`${i}/${j}`))
      writeFileSync(toTestPath(`${i}/${j}.txt`), '')
      for (let k = 0; k < 3; k++) {
        writeFileSync(toTestPath(`${i}/${j}/${k}.txt`), '')
      }
    }
  }
}

function cleanup() {
  rmrf(toTestPath())
  rmrf(toTestPath(undefined, false))
  rmrf(configDir)
}

beforeAll(() => {
  rmrf(configDir)
  mkdirSync(configDir)
  save()
  genTestDir()
})

afterAll(cleanup)

process.on('uncaughtException', (err) => {
  cleanup()
  throw err
})

describe('none command', () => {
  const help =
    log.grid(
      [
        ['scaffold', '[-h|--help] [-v|--version]'],
        ['', '<command> [<flags>]'],
      ],
      1
    ) +
    '\n\nAvailable commands are as follows:\n\n' +
    log.grid([
      ['list [-p|--prune]', 'List all projects.'],
      ['add <path ...> [-d|--depth <0|1>]', 'Add projects with path of a local folder.'],
      ['remove <name ...>', 'Remove projects from list.'],
      [
        'create <name> [<directory>] [-o|--overwrite]',
        'Create a project by copying the templates folder.',
      ],
    ])

  test('none', async () => {
    const { stdout } = await run('')
    expect(stdout).toBe(help)
  })

  test('invalid flag', async () => {
    const { stderr } = await run('', ['-z'])
    expect(stderr).toBe(log.error(`'-z' is not a valid flag. See 'scaffold --help'.`))
  })

  test('version', async () => {
    const { stdout } = await run('-v')
    const pkg = JSON.parse(
      readFileSync(join(__dirname, '../package.json'), { encoding: 'utf-8' })
    )
    expect(stdout).toBe(pkg.version)
  })

  test('help', async () => {
    const { stdout } = await run('-h')
    expect(stdout).toBe(help)
  })
})

describe('list', () => {
  beforeAll(() => {
    save({ foo: { path: toTestPath(constants.notExists) } })
  })

  afterAll(() => {
    save()
  })

  test('no flag', async () => {
    const { stdout } = await run('list')
    expect(stdout).toBe(log.grid([['foo', toTestPath(constants.notExists)]]))
  })

  test('prune', async () => {
    const { stdout } = await run('list', ['--prune'])
    expect(stdout).toBe('')
    const { stdout: stdout2 } = await run('list')
    expect(stdout2).toBe('')
  })
})

describe('add local directory', () => {
  test('no args', async () => {
    const { stderr } = await run('add')
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('depth not in valid range', async () => {
    const { stderr } = await run('add', ['--depth', '999'])
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('depth is not a number', async () => {
    const { stderr } = await run('add', ['--depth', 'a'])
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('first flag is not depth', async () => {
    const { stderr } = await run('add', ['--help'])
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('depth is 0 and path does not point to a folder', async () => {
    const path = toTestPath(constants.isAFile)
    const { stderr, stdout } = await run('add', [path])
    expect(stderr).toBe(log.error(`'${path}' is not a directory.`))
    expect(stdout).toBe('')
  })

  test('path is not exists', async () => {
    const path = toTestPath(constants.notExists)
    const { stderr, stdout } = await run('add', [path])
    expect(stderr).toBe(log.error(`Can't find directory '${path}'.`))
    expect(stdout).toBe('')
  })

  test('absolute path is valid', async () => {
    const path = toTestPath('0')
    const { stdout } = await run('add', [path])
    expect(stdout).toBe(log.grid([['+ 0', path]]))
  })

  test('relative path is valid', async () => {
    const path = `./${testSourceDir}/0`
    const { stdout } = await run('add', [path])
    expect(stdout).toBe(log.grid([['+ 0', toTestPath('0')]]))
  })

  test('absolute path is valid and depth is 1', async () => {
    const path = toTestPath()
    const { stdout } = await run('add', [path, '--depth', '1'])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(list)
  })

  test('relative path is valid and depth is 1', async () => {
    const path = `./${testSourceDir}`
    const { stdout } = await run('add', [path, '--depth', '1'])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(list)
  })

  test('multiple path and depth is 0', async () => {
    const paths = ['0', '1', '2'].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(list)
  })

  test('multiple path and depth is 0, but one of them is invalid', async () => {
    const invalidPath = 'invalid'
    const paths = ['0', '1', invalidPath].map((val) => toTestPath(val))
    const { stderr, stdout } = await run('add', [...paths])
    expect(stderr).toBe(log.error(`Can't find directory '${toTestPath(invalidPath)}'.`))
    expect(stdout).toBe('')
  })

  test('multiple path and depth is 1', async () => {
    const paths = ['0', '1', '2'].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths, '--depth', '1'])
    const list = log.grid(
      ['node_modules', '0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(`2/${val}`)]
      })
    )
    expect(stdout).toBe(list)
  })

  test('multiple path and depth is 1, but one of them is invalid', async () => {
    const invalidPath = 'invalid'
    const paths = ['0', '1', invalidPath].map((val) => toTestPath(val))
    // @todo not pass --depth 1
    const { stderr, stdout } = await run('add', [...paths])
    expect(stderr).toBe(log.error(`Can't find directory '${toTestPath(invalidPath)}'.`))
    expect(stdout).toBe('')
  })
})

describe('add GitHub repository', () => {
  beforeEach(() => {
    mkdirSync(cacheDir)
  })

  afterEach(() => {
    rmrf(cacheDir)
  })

  test('invalid url', async () => {
    const { stdout, stderr } = await run('add', [
      'https://github.com/zerowong/scaffold-cli',
    ])
    expect(stderr).toBe(log.error('Invalid GitHub url'))
    expect(stdout).toBe('')
  })

  test(
    'valid url',
    async () => {
      const { stdout } = await run('add', [constants.testGitHubRepo])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(1)
      expect(dirs[0].isDirectory()).toBeTruthy()
      expect(stdout).toBe(log.grid([['+ scaffold-cli', join(cacheDir, dirs[0].name)]]))
    },
    constants.timeout
  )

  test(
    'depth is 1',
    async () => {
      const { stdout } = await run('add', [constants.testGitHubRepo, '--depth', '1'])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(1)
      expect(dirs[0].isDirectory()).toBeTruthy()
      expect(stdout).toBe(
        log.grid([
          ['+ test', join(cacheDir, dirs[0].name, 'test')],
          ['+ bin', join(cacheDir, dirs[0].name, 'bin')],
          ['+ scripts', join(cacheDir, dirs[0].name, 'scripts')],
          ['+ src', join(cacheDir, dirs[0].name, 'src')],
        ])
      )
    },
    constants.timeout
  )

  test.todo('multiple github repo')
  test.todo('multiple github repo and depth is 1')
  test.todo('multiple github repo but one of them throw the error')
})

describe('remove', () => {
  const store = {
    foo: { path: '/path/to/foo' },
    bar: { path: '/path/to/bar' },
  }

  beforeEach(() => {
    save(store)
  })

  test('no args', async () => {
    const { stderr } = await run('remove')
    expect(stderr).toBe(log.usage('scaffold remove <name ...>'))
  })

  test('valid name', async () => {
    const { stdout } = await run('remove', ['foo'])
    const list = log.grid([['- foo', store.foo.path]])
    expect(stdout).toBe(list)
  })

  test('invalid name', async () => {
    const { stderr, stdout } = await run('remove', ['baz'])
    expect(stderr).toBe(log.error(`No such project: 'baz'.`))
    expect(stdout).toBe('')
  })

  test('multiple name', async () => {
    const { stdout } = await run('remove', ['foo', 'bar'])
    const list = log.grid([
      ['- foo', store.foo.path],
      ['- bar', store.bar.path],
    ])
    expect(stdout).toBe(list)
  })

  test('multiple name but one of them is not exists', async () => {
    const { stderr, stdout } = await run('remove', ['foo', 'bar', 'baz'])
    expect(stderr).toBe(log.error(`No such project: 'baz'.`))
    expect(stdout).toBe('')
  })
})

describe('create from local dir', () => {
  const store = {
    [testSourceDir]: { path: toTestPath('0') },
    foo: { path: toTestPath(constants.notExists) },
  }
  const targetPath = toTestPath(undefined, false)

  const expectTargetDir = (target: string) => {
    const dirs = readdirSync(target)
    const allSubDirs = ['0', '1', '2'].map((val) => {
      return readdirSync(`${target}/${val}`)
    })
    expect(dirs).toHaveLength(6)
    for (const item of ['.git', '.DS_Store', 'node_modules']) {
      expect(dirs).not.toContain(item)
    }
    for (const item of ['0', '1', '2']) {
      expect(dirs).toContain(item)
      expect(dirs).toContain(`${item}.txt`)
    }
    allSubDirs.forEach((item) => {
      expect(item).toEqual(['0.txt', '1.txt', '2.txt'])
    })
  }

  beforeAll(() => {
    save(store)
  })

  afterAll(() => {
    rmrf(targetPath)
  })

  test('no args', async () => {
    const { stderr } = await run('create')
    expect(stderr).toBe(
      log.usage('scaffold create <name> [<directory>] [-o|--overwrite]')
    )
  })

  test('project is not exists', async () => {
    const { stderr, stdout } = await run('create', ['not-exists'])
    expect(stderr).toBe(log.error(`Can't find project 'not-exists'.`))
    expect(stdout).toBe('')
  })

  test('project is exists but the real path is not exists', async () => {
    const { stderr, stdout } = await run('create', ['foo'])
    expect(stderr).toBe(log.error(`Can't find directory '${store.foo.path}'.`))
    expect(stdout).toBe('')
  })

  test('source equal target', async () => {
    const { stderr, stdout } = await run('create', [
      testSourceDir,
      `./${testSourceDir}/0`,
    ])
    expect(stderr).toBe(log.error('Source path and target paths cannot be the same.'))
    expect(stdout).toBe('')
  })

  test('create successfully', async () => {
    const { stdout } = await run('create', [testSourceDir, testTargetDir])
    expect(stdout).toBe(log.info(`Project created in '${targetPath}'.`))
    expectTargetDir(targetPath)
    rmrf(targetPath)
  })

  test('target is exists', async () => {
    mkdirSync(targetPath)
    const { stderr, stdout } = await run('create', [testSourceDir, `./${testTargetDir}`])
    rmrf(targetPath)
    expect(stderr).toBe(log.error(`Directory '${targetPath}' already exists.`))
    expect(stdout).toBe('')
  })

  test('target is exists and has overwirte flag', async () => {
    mkdirSync(targetPath)
    const { stdout } = await run('create', [testSourceDir, `./${testTargetDir}`, '-o'])
    expect(stdout).toBe(log.info(`Project created in '${targetPath}'.`))
    expectTargetDir(targetPath)
    rmrf(targetPath)
  })
})

describe('create from GitHub repository', () => {
  const targetPath = toTestPath(undefined, false)

  beforeAll(async () => {
    const lastestHash = await fetchHeadHash(constants.testGitHubRepo)
    if (!lastestHash) {
      throw new Error('could not find hash')
    }
    mkdirSync(cacheDir)
    const foo = join(cacheDir, 'foo')
    mkdirSync(foo)
    writeFileSync(join(foo, 'foo.txt'), '')
    save({
      foo: {
        path: foo,
        remote: constants.testGitHubRepo,
        hash: lastestHash,
      },
      bar: {
        path: join(cacheDir, 'scaffold-cli'),
        remote: constants.testGitHubRepo,
        hash: '83db959639ebc2f212eca5dc01bbdf9375b21419',
      },
      baz: {
        path: join(cacheDir, 'scaffold-cli', 'bin'),
        remote: constants.testGitHubRepo,
        hash: '83db959639ebc2f212eca5dc01bbdf9375b21419',
      },
    })
  })

  beforeEach(() => {
    mkdirSync(targetPath)
  })

  afterEach(() => {
    rmrf(targetPath)
  })

  test(
    'use local cache',
    async () => {
      const target = join(targetPath, 'foo')
      const { stdout } = await run('create', ['foo', target])
      expect(stdout).toBe(log.info(`Project created in '${target}'.`))
      const files = readdirSync(target)
      expect(files).toHaveLength(1)
      expect(files[0]).toBe('foo.txt')
    },
    constants.timeout
  )

  test(
    'update cache',
    async () => {
      const target = join(targetPath, 'bar')
      const { stdout } = await run('create', ['bar', target])
      expect(stdout).toBe(log.info(`Project created in '${target}'.`))
      const files = readdirSync(target)
      expect(files.length).toBeGreaterThan(1)
    },
    constants.timeout
  )

  test(
    'update cache for subdir',
    async () => {
      const target = join(targetPath, 'baz')
      const { stdout } = await run('create', ['baz', target])
      expect(stdout).toBe(log.info(`Project created in '${target}'.`))
      const files = readdirSync(target)
      expect(files).toEqual(['index.js'])
    },
    constants.timeout
  )
})

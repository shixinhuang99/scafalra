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
import type { Project } from '../src/cli.js'
import { parse } from '../src/utils.js'

const cwd = process.cwd()
function joinCwd(path: string) {
  return join(cwd, path)
}

const cliPath = joinCwd('bin/index.js')
const configDir = join(homedir(), '.scaffold-cli-test')
const storeFile = join(configDir, 'store.json')
const cacheDir = join(configDir, 'cache')
const joinCacheDir = (...paths: string[]) => join(cacheDir, ...paths)

async function run(command: string, args: string[] = []) {
  return new Promise<{ stdout: string }>((resolve, reject) => {
    execFile('node', [cliPath, command, ...args], (err, stdout) => {
      if (err) {
        return reject(err)
      }
      return resolve({ stdout: stdout.trimEnd() })
    })
  })
}

const log = {
  info(msg: string) {
    return `INFO: ${msg}`
  },
  error(msg: string) {
    return `ERROR: ${msg}`
  },
  usage(msg: string) {
    return `USAGE: ${msg}`
  },
  grid(msgs: [string, string][], space = 4) {
    const max = msgs.reduce((prev, curr) => {
      return Math.max(curr[0].length, prev)
    }, 0)
    const res = msgs.reduce((perv, curr, i) => {
      return (
        perv +
        `${curr[0]}${' '.repeat(max - curr[0].length + space)}${curr[1]}${
          i === msgs.length - 1 ? '' : '\n'
        }`
      )
    }, '')
    return res
  },
  result(success: number, failed: string[]) {
    const str = failed.reduce(
      (prev, curr) => `${prev}\nERROR: ${curr}`,
      `INFO: ${success} success, ${failed.length} fail.`
    )
    return str
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
  addUsage: 'scaffold add <path|url ...> [-d|--depth <0|1>]',
  testGitHubRepo: 'https://github.com/zerowong/scaffold-cli.git',
  testGitHubRepo2: 'https://github.com/zerowong/zerowong.github.io.git',
  timeout: 20000,
  debugTimeout: 3600000,
}

interface Store {
  [key: string]: Project
}

function save(store: Store = {}) {
  writeFileSync(storeFile, JSON.stringify(store))
}

function getStore() {
  const raw = readFileSync(storeFile, { encoding: 'utf8' })
  return JSON.parse(raw) as Store
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

let lastestHash: string | null = null

beforeAll(async () => {
  cleanup()
  mkdirSync(configDir)
  save()
  genTestDir()
  lastestHash = (await parse(constants.testGitHubRepo)).hash
}, constants.timeout)

afterAll(cleanup)

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

  test('none', async () => {
    const { stdout } = await run('')
    expect(stdout).toBe(help)
  })

  test('invalid flag', async () => {
    const { stdout } = await run('', ['-z'])
    expect(stdout).toBe(log.error(`'-z' is not a valid flag. See 'scaffold --help'.`))
  })

  test('version', async () => {
    const { stdout } = await run('-v')
    const pkg = JSON.parse(
      readFileSync(new URL('../package.json', import.meta.url), { encoding: 'utf-8' })
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
  beforeEach(() => {
    save()
  })

  afterAll(() => {
    save()
  })

  test('no args', async () => {
    const { stdout } = await run('add')
    expect(stdout).toBe(log.usage(constants.addUsage))
  })

  test('depth not in valid range', async () => {
    const { stdout } = await run('add', ['--depth', '999'])
    expect(stdout).toBe(log.usage(constants.addUsage))
  })

  test('depth is not a number', async () => {
    const { stdout } = await run('add', ['--depth', 'a'])
    expect(stdout).toBe(log.usage(constants.addUsage))
  })

  test('first flag is not depth', async () => {
    const { stdout } = await run('add', ['--help'])
    expect(stdout).toBe(log.usage(constants.addUsage))
  })

  test('depth is 0 and path does not point to a folder', async () => {
    const path = toTestPath(constants.isAFile)
    const { stdout } = await run('add', [path])
    expect(getStore()).toEqual({})
    expect(stdout).toBe(log.result(0, [`'${path}' is not a directory.`]))
  })

  test('path is not exists', async () => {
    const path = toTestPath(constants.notExists)
    const { stdout } = await run('add', [path])
    expect(getStore()).toEqual({})
    expect(stdout).toBe(log.result(0, [`Can't find directory '${path}'.`]))
  })

  test('absolute path is valid', async () => {
    const path = toTestPath('0')
    const { stdout } = await run('add', [path])
    expect(getStore()).toEqual({ '0': { path } })
    expect(stdout).toBe(`${log.grid([['+ 0', path]])}\n${log.result(1, [])}`)
  })

  test('relative path is valid', async () => {
    const path = `./${testSourceDir}/0`
    const { stdout } = await run('add', [path])
    expect(getStore()).toEqual({ '0': { path: toTestPath('0') } })
    expect(stdout).toBe(`${log.grid([['+ 0', toTestPath('0')]])}\n${log.result(1, [])}`)
  })

  test('absolute path is valid and depth is 1', async () => {
    const path = toTestPath()
    const { stdout } = await run('add', [path, '--depth', '1'])
    const obj: Store = {}
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        const t = toTestPath(val)
        obj[val] = { path: t }
        return [`+ ${val}`, t]
      })
    )
    expect(getStore()).toEqual(obj)
    expect(stdout).toBe(`${list}\n${log.result(1, [])}`)
  })

  test('relative path is valid and depth is 1', async () => {
    const path = `./${testSourceDir}`
    const { stdout } = await run('add', [path, '--depth', '1'])
    const obj: Store = {}
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        const t = toTestPath(val)
        obj[val] = { path: t }
        return [`+ ${val}`, t]
      })
    )
    expect(getStore()).toEqual(obj)
    expect(stdout).toBe(`${list}\n${log.result(1, [])}`)
  })

  test('multiple path and depth is 0', async () => {
    const paths = ['0', '1', '2'].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths])
    const store = getStore()
    const obj = ['0', '1', '2'].reduce<Store>((prev, curr) => {
      prev[curr] = { path: toTestPath(curr) }
      return prev
    }, {})
    expect(store).toEqual(obj)
    expect(stdout.split('\n').at(-1)).toBe(log.result(3, []))
  })

  test('multiple path and depth is 0, but one of them is invalid', async () => {
    const invalidPath = 'invalid'
    const paths = ['0', '1', invalidPath].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths])
    const store = getStore()
    const obj = ['0', '1'].reduce<Store>((prev, curr) => {
      prev[curr] = { path: toTestPath(curr) }
      return prev
    }, {})
    expect(store).toEqual(obj)
    expect(stdout.split('\n').slice(-2).join('\n')).toBe(
      log.result(2, [`Can't find directory '${toTestPath(invalidPath)}'.`])
    )
  })

  test('multiple path and depth is 1', async () => {
    const paths = ['0', '1', '2'].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths, '--depth', '1'])
    const store = getStore()
    const storeKeys = Object.keys(store)
    const keys = [
      '0',
      '1',
      '2',
      'node_modules',
      '0-1',
      '1-1',
      '2-1',
      'node_modules-1',
      '0-2',
      '1-2',
      '2-2',
      'node_modules-2',
    ]
    expect(storeKeys.length).toBe(keys.length)
    expect(storeKeys.every((val) => keys.indexOf(val) !== -1)).toBeTruthy()
    expect(stdout.split('\n').at(-1)).toBe(log.result(3, []))
  })

  test('multiple path and depth is 1, but one of them is invalid', async () => {
    const invalidPath = 'invalid'
    const paths = ['0', '1', invalidPath].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths, '--depth', '1'])
    const store = getStore()
    const names = ['0', '1', '2', 'node_modules', '0-1', '1-1', '2-1', 'node_modules-1']
    expect(Object.keys(store).every((val) => names.indexOf(val) !== -1)).toBeTruthy()
    expect(stdout.split('\n').slice(-2).join('\n')).toBe(
      log.result(2, [`Can't find directory '${toTestPath(invalidPath)}'.`])
    )
  })
})

describe('add GitHub repository', () => {
  const baseProj = {
    hash: '',
    remote: constants.testGitHubRepo,
  }

  const baseProj2 = {
    hash: '26e5548e008df18ecd4aa02fe69b1afef476c06a',
    remote: constants.testGitHubRepo2,
  }

  beforeAll(() => {
    if (!lastestHash) {
      throw new Error('no hash')
    }
    baseProj.hash = lastestHash
  })

  afterAll(() => {
    save()
  })

  beforeEach(() => {
    mkdirSync(cacheDir)
    save()
  })

  afterEach(() => {
    rmrf(cacheDir)
  })

  test('invalid url', async () => {
    const { stdout } = await run('add', ['https://github.com/zerowong/scaffold-cli'])
    expect(stdout).toBe(log.result(0, ['Invalid GitHub url']))
  })

  test(
    'valid url',
    async () => {
      const { stdout } = await run('add', [constants.testGitHubRepo])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(1)
      expect(dirs[0].isDirectory()).toBeTruthy()
      expect(getStore()).toEqual({
        'scaffold-cli': { path: joinCacheDir(dirs[0].name), ...baseProj },
      })
      expect(stdout.split('\n').at(-1)).toBe(log.result(1, []))
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
      const projName = dirs[0].name
      expect(getStore()).toEqual({
        'bin': { path: joinCacheDir(projName, 'bin'), ...baseProj },
        'scripts': { path: joinCacheDir(projName, 'scripts'), ...baseProj },
        'src': { path: joinCacheDir(projName, 'src'), ...baseProj },
        'test': { path: joinCacheDir(projName, 'test'), ...baseProj },
      })
      expect(stdout.split('\n').at(-1)).toBe(log.result(1, []))
    },
    constants.timeout
  )

  test(
    'multiple github repo',
    async () => {
      const { stdout } = await run('add', [
        constants.testGitHubRepo,
        constants.testGitHubRepo2,
      ])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(2)
      expect(dirs.every((t) => t.isDirectory())).toBeTruthy()
      expect(getStore()).toEqual({
        'scaffold-cli': { path: joinCacheDir(dirs[0].name), ...baseProj },
        'zerowong.github.io': { path: joinCacheDir(dirs[1].name), ...baseProj2 },
      })
      expect(stdout.split('\n').at(-1)).toBe(log.result(2, []))
    },
    constants.timeout
  )

  test(
    'multiple github repo and depth is 1',
    async () => {
      const { stdout } = await run('add', [
        constants.testGitHubRepo,
        constants.testGitHubRepo2,
        '--depth',
        '1',
      ])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(2)
      expect(dirs.every((t) => t.isDirectory())).toBeTruthy()
      const store = getStore()
      const storeKeys = Object.keys(store)
      const keys = ['bin', 'scripts', 'src', 'test', 'public', 'src-1']
      expect(storeKeys.length).toBe(keys.length)
      expect(storeKeys.every((val) => keys.indexOf(val) !== -1)).toBeTruthy()
      expect(stdout.split('\n').at(-1)).toBe(log.result(2, []))
    },
    constants.timeout
  )

  test(
    'multiple github repo but one of them failed',
    async () => {
      const { stdout } = await run('add', [
        constants.testGitHubRepo,
        'https://github.com/zerowong/scaffold-cli',
      ])
      const dirs = readdirSync(cacheDir, { withFileTypes: true })
      expect(dirs).toHaveLength(1)
      expect(dirs[0].isDirectory).toBeTruthy()
      expect(getStore()).toEqual({
        'scaffold-cli': { path: joinCacheDir(dirs[0].name), ...baseProj },
      })
      expect(stdout.split('\n').slice(-2).join('\n')).toBe(
        log.result(1, ['Invalid GitHub url'])
      )
    },
    constants.timeout
  )
})

describe('remove', () => {
  const store = {
    foo: { path: '/path/to/foo' },
    bar: { path: '/path/to/bar' },
  }

  beforeEach(() => {
    save(store)
  })

  afterAll(() => {
    save()
  })

  test('no args', async () => {
    const { stdout } = await run('remove')
    expect(stdout).toBe(log.usage('scaffold remove <name ...>'))
  })

  test('valid name', async () => {
    const { stdout } = await run('remove', ['foo'])
    const list = log.grid([['- foo', store.foo.path]])
    expect(stdout).toBe(`${list}\n${log.result(1, [])}`)
  })

  test('invalid name', async () => {
    const { stdout } = await run('remove', ['baz'])
    expect(stdout).toBe(log.result(0, ["No such project: 'baz'."]))
  })

  test('multiple name', async () => {
    const { stdout } = await run('remove', ['foo', 'bar'])
    const list = log.grid([
      ['- foo', store.foo.path],
      ['- bar', store.bar.path],
    ])
    expect(stdout).toBe(`${list}\n${log.result(2, [])}`)
  })

  test('multiple name but one of them is not exists', async () => {
    const { stdout } = await run('remove', ['foo', 'bar', 'baz'])
    const list = log.grid([
      ['- foo', store.foo.path],
      ['- bar', store.bar.path],
    ])
    expect(stdout).toBe(`${list}\n${log.result(2, ["No such project: 'baz'."])}`)
  })

  test('remove remote project', async () => {
    mkdirSync(cacheDir)
    const remoteProjPath = joinCacheDir('remote')
    mkdirSync(remoteProjPath)
    save({ remote: { path: remoteProjPath, remote: '_', hash: '' } })
    const { stdout } = await run('remove', ['remote'])
    const list = log.grid([['- remote', remoteProjPath]])
    expect(stdout).toBe(`${list}\n${log.result(1, [])}`)
    const dirs = readdirSync(cacheDir)
    expect(dirs).toHaveLength(0)
    rmrf(cacheDir)
  })
})

describe('create from local dir', () => {
  const store = {
    [testSourceDir]: { path: toTestPath('0') },
    foo: { path: toTestPath(constants.notExists) },
  }
  const targetPath = toTestPath(undefined, false)

  const expectTargetDir = () => {
    const dirs = readdirSync(targetPath)
    expect(dirs).toHaveLength(6)
    for (const item of ['.git', '.DS_Store', 'node_modules']) {
      expect(dirs).not.toContain(item)
    }
    for (const item of ['0', '1', '2']) {
      expect(dirs).toContain(item)
      expect(dirs).toContain(`${item}.txt`)
    }
    const allSubDirs = ['0', '1', '2'].map((val) => {
      return readdirSync(`${targetPath}/${val}`)
    })
    allSubDirs.forEach((item) => {
      expect(item).toEqual(['0.txt', '1.txt', '2.txt'])
    })
  }

  beforeAll(() => {
    save(store)
  })

  afterEach(() => {
    rmrf(targetPath)
  })

  test('no args', async () => {
    const { stdout } = await run('create')
    expect(stdout).toBe(
      log.usage('scaffold create <name|path|url> [<directory>] [-o|--overwrite]')
    )
  })

  test('project is not exists', async () => {
    const { stdout } = await run('create', ['not-exists'])
    expect(stdout).toBe(
      log.error(`Can't find directory '${joinCwd(constants.notExists)}'.`)
    )
  })

  test('project is exists but the real path is not exists', async () => {
    const { stdout } = await run('create', ['foo'])
    expect(stdout).toBe(log.error(`Can't find directory '${store.foo.path}'.`))
  })

  test('source equal target', async () => {
    const { stdout } = await run('create', [testSourceDir, `./${testSourceDir}/0`])
    expect(stdout).toBe(log.error('Source path and target paths cannot be the same.'))
  })

  test('create successfully', async () => {
    const { stdout } = await run('create', [testSourceDir, testTargetDir])
    expect(stdout).toBe(log.info(`Project created in '${targetPath}'.`))
    expectTargetDir()
  })

  test('target is exists', async () => {
    mkdirSync(targetPath)
    const { stdout } = await run('create', [testSourceDir, `./${testTargetDir}`])
    expect(stdout).toBe(log.error(`Directory '${targetPath}' already exists.`))
  })

  test('target is exists and has overwirte flag', async () => {
    mkdirSync(targetPath)
    const { stdout } = await run('create', [testSourceDir, `./${testTargetDir}`, '-o'])
    expect(stdout).toBe(log.info(`Project created in '${targetPath}'.`))
    expectTargetDir()
  })

  test('create a project directly from the local path', async () => {
    save()
    const { stdout } = await run('create', [`${testSourceDir}/0`, testTargetDir])
    expect(stdout).toBe(log.info(`Project created in '${targetPath}'.`))
    expectTargetDir()
  })
})

describe('create from GitHub repository', () => {
  const targetPath = toTestPath(undefined, false)

  beforeAll(() => {
    if (!lastestHash) {
      throw new Error('no hash')
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
      expect(files).toHaveLength(16)
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

  test(
    'create a project directly from a GitHub URL',
    async () => {
      const target = toTestPath('scaffold-cli', false)
      const { stdout } = await run('create', [constants.testGitHubRepo, targetPath])
      expect(stdout).toBe(log.info(`Project created in '${target}'.`))
      const files = readdirSync(target)
      expect(files).toHaveLength(16)
    },
    constants.timeout
  )

  test(
    'create a project directly from a GitHub URL but dest already exists',
    async () => {
      const target = toTestPath('scaffold-cli', false)
      mkdirSync(target)
      const { stdout } = await run('create', [constants.testGitHubRepo, targetPath])
      expect(stdout).toBe(log.error(`Directory '${target}' already exists.`))
    },
    constants.timeout
  )

  test(
    'create a project directly from a GitHub URL with overwirte',
    async () => {
      const target = toTestPath('scaffold-cli', false)
      mkdirSync(target)
      const { stdout } = await run('create', [
        constants.testGitHubRepo,
        targetPath,
        '--overwrite',
      ])
      expect(stdout).toBe(log.info(`Project created in '${target}'.`))
      const files = readdirSync(target)
      expect(files).toHaveLength(16)
    },
    constants.timeout
  )
})

describe('mv', () => {
  beforeAll(() => {
    save({ foo: { path: '_' }, bar: { path: '_' } })
  })

  afterAll(() => {
    save()
  })

  test('invalid or missing arguments', async () => {
    const { stdout } = await run('mv', ['foo'])
    expect(stdout).toBe(log.usage('scaffold mv <oldName> <newName>'))
  })

  test('project not exists', async () => {
    const { stdout } = await run('mv', ['baz', 'bar'])
    expect(stdout).toBe('')
  })

  test('the new name is the same as the old name', async () => {
    const { stdout } = await run('mv', ['foo', 'foo'])
    expect(stdout).toBe('')
  })

  test('new name exists', async () => {
    const { stdout } = await run('mv', ['foo', 'bar'])
    expect(stdout).toBe(log.error("'bar' already exists."))
  })

  test('rename successfully', async () => {
    const { stdout } = await run('mv', ['foo', 'baz'])
    expect(stdout).toBe('')
    expect(getStore()).toEqual({ bar: { path: '_' }, baz: { path: '_' } })
  })
})

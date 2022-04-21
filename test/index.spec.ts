import { join } from 'node:path'
import { readFileSync, writeFileSync, mkdirSync, rmSync, readdirSync } from 'node:fs'
import { exec } from 'node:child_process'
import { homedir, tmpdir } from 'node:os'
import { describe, test, expect, beforeAll, afterAll, beforeEach } from 'vitest'

const cwd = process.cwd()
function joinCwd(path: string) {
  return join(cwd, path)
}

const cliPath = joinCwd('bin/index.js')
const pkg = JSON.parse(readFileSync(joinCwd('package.json'), { encoding: 'utf8' }))
const configPath = join(homedir() ?? tmpdir(), '.scaffold-cli-test.json')

async function run(command: string, args?: string[]) {
  return new Promise<{ stdout: string; stderr: string }>((resolve, reject) => {
    let commandStr = `node ${cliPath} ${command}`
    if (args && args.length > 0) {
      commandStr += ` ${args.join(' ')}`
    }
    exec(commandStr, (err, stdout, stderr) => {
      if (err) {
        return reject(err)
      }
      return resolve({ stdout: stdout.trimEnd(), stderr: stderr.trimEnd() })
    })
  })
}

const log = {
  error(msg: string) {
    return `scaffold-cli: ${msg}`
  },
  usage(msg: string) {
    return `usage: ${msg}`
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
  commonUsage: 'scaffold-cli [-h|--help] [-v|--version]',
  addUsage: 'scaffold-cli add <path ...> [-d|--depth <0|1>]',
  newProjects: 'New projects:',
  removedProjects: 'Removed projects:',
}

function writeConfig(obj: { projects: Record<string, string> } = { projects: {} }) {
  writeFileSync(configPath, JSON.stringify(obj))
}

function rmrf(target: string) {
  return rmSync(target, { force: true, recursive: true })
}

/**
 * .
 * ├── .ignore-dir
 * ├── 0
 * │   ├── .DS_Store
 * │   ├── .git
 * │   ├── 0
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 0.txt
 * │   ├── 1
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 1.txt
 * │   ├── 2
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 2.txt
 * │   └── node_modules
 * ├── 1
 * │   ├── .DS_Store
 * │   ├── .git
 * │   ├── 0
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 0.txt
 * │   ├── 1
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 1.txt
 * │   ├── 2
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 2.txt
 * │   └── node_modules
 * ├── 2
 * │   ├── .DS_Store
 * │   ├── .git
 * │   ├── 0
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 0.txt
 * │   ├── 1
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 1.txt
 * │   ├── 2
 * │   │   ├── 0.txt
 * │   │   ├── 1.txt
 * │   │   └── 2.txt
 * │   ├── 2.txt
 * │   └── node_modules
 * └── is-a-file.txt
 */

beforeAll(() => {
  writeConfig()
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
})

afterAll(() => {
  rmrf(toTestPath())
  rmSync(configPath)
})

describe('none command', () => {
  test('none', async () => {
    const { stderr } = await run('')
    expect(stderr).toBe(log.usage(constants.commonUsage))
  })

  test('invalid flag', async () => {
    const { stderr } = await run('', ['-z'])
    expect(stderr).toBe(log.error(`'-z' is not a valid flag. See 'scaffold-cli --help'.`))
  })

  test('version', async () => {
    const { stdout } = await run('-v')
    expect(stdout).toBe(pkg.version)
  })

  test('help', async () => {
    const { stdout } = await run('-h')
    let res = ''
    res += log.grid(
      [
        ['scaffold-cli', '[-h|--help] [-v|--version]'],
        ['', '<command> [<flags>]'],
      ],
      1
    )
    res += '\n\nAvailable commands are as follows:\n\n'
    res += log.grid([
      ['list', 'List all projects.'],
      ['add <path ...> [-d|--depth <0|1>]', 'Add projects with path of a local folder.'],
      ['remove <name ...>', 'Remove projects.'],
      ['create <name> [<directory>] [-o|--overwrite]', 'Create a project from list.'],
    ])
    expect(stdout).toBe(res)
  })
})

describe('list', () => {
  beforeAll(() => {
    writeConfig({
      projects: {
        foo: toTestPath(constants.notExists),
      },
    })
  })

  afterAll(() => {
    writeConfig()
  })

  test('no flag', async () => {
    const { stdout } = await run('list')
    expect(stdout).toBe(log.grid([['foo', toTestPath(constants.notExists)]]))
  })
})

describe('add', () => {
  beforeEach(() => {
    writeConfig()
  })

  test('no args', async () => {
    const { stderr } = await run('add')
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('depth not in valid range', async () => {
    const { stderr } = await run('add', ['', '--depth', '999'])
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('depth is not a number', async () => {
    const { stderr } = await run('add', ['', '--depth', 'a'])
    expect(stderr).toBe(log.usage(constants.addUsage))
  })

  test('first flag is not depth', async () => {
    const { stderr } = await run('add', ['', '--help'])
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
    expect(stdout).toBe(`${constants.newProjects}\n\n${log.grid([['+ 0', path]])}`)
  })

  test('relative path is valid', async () => {
    const path = `./${testSourceDir}/0`
    const { stdout } = await run('add', [path])
    expect(stdout).toBe(
      `${constants.newProjects}\n\n${log.grid([['+ 0', toTestPath('0')]])}`
    )
  })

  test('absolute path is valid and depth is 1', async () => {
    const path = toTestPath()
    const { stdout } = await run('add', [path, '--depth', '1'])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(`${constants.newProjects}\n\n${list}`)
  })

  test('relative path is valid and depth is 1', async () => {
    const path = `./${testSourceDir}`
    const { stdout } = await run('add', [path, '--depth', '1'])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(`${constants.newProjects}\n\n${list}`)
  })

  test('multiple path and depth is 0', async () => {
    const paths = ['0', '1', '2'].map((val) => toTestPath(val))
    const { stdout } = await run('add', [...paths])
    const list = log.grid(
      ['0', '1', '2'].map((val) => {
        return [`+ ${val}`, toTestPath(val)]
      })
    )
    expect(stdout).toBe(`${constants.newProjects}\n\n${list}`)
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
    expect(stdout).toBe(`${constants.newProjects}\n\n${list}`)
  })

  test('multiple path and depth is 1, but one of them is invalid', async () => {
    const invalidPath = 'invalid'
    const paths = ['0', '1', invalidPath].map((val) => toTestPath(val))
    const { stderr, stdout } = await run('add', [...paths])
    expect(stderr).toBe(log.error(`Can't find directory '${toTestPath(invalidPath)}'.`))
    expect(stdout).toBe('')
  })
})

describe('remove', () => {
  const projects = {
    foo: '/path/to/foo',
    bar: '/path/to/bar',
  }

  beforeEach(() => {
    writeConfig({ projects })
  })

  test('no args', async () => {
    const { stderr } = await run('remove')
    expect(stderr).toBe(log.usage('scaffold-cli remove <name ...>'))
  })

  test('valid name', async () => {
    const { stdout } = await run('remove', ['foo'])
    const list = log.grid([['- foo', projects.foo]])
    expect(stdout).toBe(`${constants.removedProjects}\n\n${list}`)
  })

  test('invalid name', async () => {
    const { stderr, stdout } = await run('remove', ['baz'])
    expect(stderr).toBe(log.error(`No such project: 'baz'.`))
    expect(stdout).toBe('')
  })

  test('multiple name', async () => {
    const { stdout } = await run('remove', ['foo', 'bar'])
    const list = log.grid([
      ['- foo', projects.foo],
      ['- bar', projects.bar],
    ])
    expect(stdout).toBe(`${constants.removedProjects}\n\n${list}`)
  })

  test('multiple name but one of them is not exists', async () => {
    const { stderr, stdout } = await run('remove', ['foo', 'bar', 'baz'])
    expect(stderr).toBe(log.error(`No such project: 'baz'.`))
    expect(stdout).toBe('')
  })
})

function expectTargetDir(target: string, expectRef: typeof expect) {
  const dirs = readdirSync(target)
  const allSubDirs = ['0', '1', '2'].map((val) => {
    return readdirSync(`${target}/${val}`)
  })
  expectRef(dirs).toHaveLength(6)
  ;['.git', '.DS_Store', 'node_modules'].forEach((item) => {
    expectRef(dirs).not.toContain(item)
  })
  ;['0', '1', '2'].forEach((item) => {
    expectRef(dirs).toContain(item)
    expectRef(dirs).toContain(`${item}.txt`)
  })
  allSubDirs.forEach((item) => {
    expectRef(item).toEqual(['0.txt', '1.txt', '2.txt'])
  })
}

describe('create', () => {
  const projects = {
    [testSourceDir]: toTestPath('0'),
    foo: toTestPath(constants.notExists),
  }
  const targetPath = toTestPath(undefined, false)

  beforeAll(() => {
    writeConfig({ projects })
  })

  test('no args', async () => {
    const { stderr } = await run('create')
    expect(stderr).toBe(
      log.usage('scaffold-cli create <name> [<directory>] [-o|--overwrite]')
    )
  })

  test('project is not exists', async () => {
    const { stderr, stdout } = await run('create', ['not-exists'])
    expect(stderr).toBe(log.error(`Can't find project 'not-exists'.`))
    expect(stdout).toBe('')
  })

  test('project is exists but the real path is not exists', async () => {
    const { stderr, stdout } = await run('create', ['foo'])
    expect(stderr).toBe(log.error(`Can't find directory '${projects.foo}'.`))
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
    expect(stdout).toBe(`Project created in '${targetPath}'.`)
    expectTargetDir(targetPath, expect)
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
    expect(stdout).toBe(`Project created in '${targetPath}'.`)
    expectTargetDir(targetPath, expect)
    rmrf(targetPath)
  })
})

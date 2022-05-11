import path from 'node:path'
import fsp from 'node:fs/promises'
import child_process from 'node:child_process'
import https from 'node:https'
import fs from 'node:fs'
import chalk from 'chalk'
import createHttpsProxyAgent from 'https-proxy-agent'
import StreamZip from 'node-stream-zip'
import mri from 'mri'

interface SystemError extends Error {
  code: string
  syscall: string
  path: string
}

export interface Result {
  success: number
  failed: string[]
}

const isTTY = process.stdout.isTTY

function isSystemError(err: unknown): err is SystemError {
  return err instanceof Error && 'syscall' in err
}

export const exception = {
  isENOENT(err: unknown): err is SystemError {
    return isSystemError(err) && err.code === 'ENOENT'
  },
  isEEXIST(err: unknown): err is SystemError {
    return isSystemError(err) && err.code === 'EEXIST'
  },
}

export const log = {
  info(msg: string) {
    console.log(`${chalk.bold.cyan('INFO')}: ${msg}`)
  },
  error(msg: string) {
    console.log(`${chalk.bold.red('ERROR')}: ${msg}`)
  },
  usage(msg: string) {
    console.log(`${chalk.bold.cyan('USAGE')}: ${msg}`)
  },
  warn(msg: string) {
    console.log(`${chalk.bold.yellow('WARN')}: ${msg}`)
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
    if (!res) {
      return
    }
    console.log(res)
  },
  write(msg: string) {
    if (isTTY) {
      process.stdout.write(msg)
    }
  },
  clear() {
    if (isTTY) {
      process.stdout.clearLine(0)
      process.stdout.cursorTo(0)
    }
  },
  result(res: Result) {
    const str = res.failed.reduce(
      (prev, curr) => `${prev}\n${chalk.bold.red('ERROR')}: ${curr}`,
      `${chalk.bold.cyan('INFO')}: ${chalk.green(res.success)} success, ${chalk.red(
        res.failed.length
      )} fail.`
    )
    console.log(str)
  },
}

export function rmrf(target: string) {
  return fsp.rm(target, { force: true, recursive: true })
}

// fsPromises.cp is experimental
export async function cp(source: string, target: string) {
  const ignore = ['.git', '.DS_Store', 'node_modules']
  const sourceDir = await fsp.readdir(source, { withFileTypes: true })
  await fsp.mkdir(target)
  for (const dirent of sourceDir) {
    if (ignore.includes(dirent.name)) {
      continue
    }
    const s = path.join(source, dirent.name)
    const t = path.join(target, dirent.name)
    if (dirent.isDirectory()) {
      await cp(s, t)
    } else if (dirent.isFile()) {
      await fsp.copyFile(s, t)
    }
  }
}

export async function exists(target: string) {
  try {
    await fsp.access(target)
  } catch {
    return false
  }
  return true
}

export function parse(src: string) {
  const regexp = /^https:\/\/github.com\/([^/\s]+)\/([^/\s]+)\.git$/
  const match = src.match(regexp)
  if (!match) {
    return null
  }
  const user = match[1]
  const name = match[2]
  const url = `https://github.com/${user}/${name}`
  return { user, name, url }
}

async function git(args: string[]) {
  return new Promise<{ stdout: string; stderr: string }>((resolve, reject) => {
    child_process.execFile('git', args, (err, stdout, stderr) => {
      if (err) {
        return reject(err)
      }
      return resolve({ stdout: stdout.trimEnd(), stderr: stderr.trimEnd() })
    })
  })
}

export async function fetchHeadHash(src: string) {
  const { stdout } = await git(['ls-remote', src])
  const blank = stdout.indexOf('\t')
  if (blank !== -1) {
    return stdout.slice(0, blank)
  }
  return null
}

export function joinGithubArchiveUrl(url: string, hash: string) {
  return path.join(url, 'archive', `${hash}.zip`)
}

export function download(url: string, dest: string, options: { proxy?: string } = {}) {
  const { proxy } = options
  const agent = proxy ? createHttpsProxyAgent(proxy) : undefined
  return new Promise<void>((resolve, reject) => {
    https
      .get(url, { agent }, (res) => {
        const { statusCode, statusMessage } = res
        if (!statusCode) {
          return reject('No response.')
        }
        if (statusCode < 300 && statusCode >= 200) {
          res.pipe(fs.createWriteStream(dest)).on('finish', resolve).on('error', reject)
        } else if (statusCode < 400 && statusCode >= 300 && res.headers.location) {
          download(res.headers.location, dest, { proxy }).then(resolve, reject)
        } else {
          reject(`${statusCode}: ${statusMessage}.`)
        }
      })
      .on('error', reject)
  })
}

/**
 * in-place unzip and rename
 * @param src archive zip file that name must be with hash
 * @returns the full directory path path of unziped dir
 */
export async function unzip(src: string) {
  const { dir, name } = path.parse(src)
  const zip = new StreamZip.async({ file: src })
  await zip.extract(null, dir)
  await zip.close()
  await fsp.rm(src)
  const index = name.lastIndexOf('-')
  if (index === -1) {
    throw new Error('Cannot remove hash.')
  }
  const nameWithoutHash = name.slice(0, index)
  const unzipedDir = path.join(dir, nameWithoutHash)
  await rmrf(unzipedDir)
  await fsp.rename(path.join(dir, name), unzipedDir)
  return unzipedDir
}

export type Validate = string[] | { flag: string; options: number[] }

export function argsParser() {
  const mriArgv = mri(process.argv.slice(2), {
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

  if (!mriArgv) {
    return null
  }

  const {
    _: [action = '', ...args],
    ...flags
  } = mriArgv

  /**
   * @returns `true` when no pass
   */
  const checker = (validate: Validate) => {
    if (Array.isArray(validate)) {
      const res = validate.filter(
        (flag) => flags[flag] !== undefined && typeof flags[flag] === 'boolean'
      )
      return !(res.length === 0 || res.length === 1)
    }
    const { flag, options } = validate
    const value = flags[flag]
    return !!value && options.indexOf(value) === -1
  }

  return { action, args, flags, checker }
}

export function isURL(arg: string) {
  if (/^(?:https?:\/\/)?(?:[\da-z.-]+)\.(?:[a-z.]{2,6})(?:[/\w .-]*)*\/?$/.test(arg)) {
    return true
  }
  return false
}

export class Key {
  private value: Record<string, number>

  constructor() {
    this.value = {}
  }

  gen(name: string) {
    if (!(name in this.value)) {
      this.value[name] = 1
    }
    return this.value[name]++
  }
}
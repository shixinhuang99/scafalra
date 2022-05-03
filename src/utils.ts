import path from 'node:path'
import fsp from 'node:fs/promises'
import child_process from 'node:child_process'
import https from 'node:https'
import fs from 'node:fs'
import chalk from 'chalk'
import createHttpsProxyAgent from 'https-proxy-agent'
import StreamZip from 'node-stream-zip'

interface SystemError extends Error {
  code: string
  syscall: string
  path: string
}

function isSystemError(err: unknown): err is SystemError {
  return err instanceof Error && 'syscall' in err
}

export const error = {
  isENOENT(err: unknown): err is SystemError {
    return isSystemError(err) && err.code === 'ENOENT'
  },
  isEEXIST(err: unknown): err is SystemError {
    return isSystemError(err) && err.code === 'EEXIST'
  },
}

export const log = {
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

export function rmrf(target: string) {
  return fsp.rm(target, { force: true, recursive: true })
}

// fsPromises.cp is experimental
export async function cp(source: string, target: string) {
  const ignore = ['.git', '.DS_Store', 'node_modules']
  const sourceDir = await fsp.opendir(source)
  await fsp.mkdir(target)
  for await (const dirent of sourceDir) {
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

export function hasInvalidFlag(allows: string[], flags: string[]) {
  if (flags.length === 0) {
    return false
  }
  return allows.some((item) => !flags.includes(item))
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
        if (statusCode && statusCode < 300 && statusCode >= 200) {
          res.pipe(fs.createWriteStream(dest)).on('finish', resolve).on('error', reject)
        } else if (
          statusCode &&
          statusCode < 400 &&
          statusCode >= 300 &&
          res.headers.location
        ) {
          res.resume()
          download(res.headers.location, dest, { proxy }).then(resolve, reject)
        } else {
          res.resume()
          reject(new Error(`${statusCode}: ${statusMessage}`))
        }
      })
      .on('error', reject)
  })
}

/**
 * in-place unzip and rename
 * @param src archive zip file that name must be with hash
 * @returns path of unziped dir
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

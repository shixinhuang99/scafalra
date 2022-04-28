import { join } from 'node:path'
import { rm, opendir, mkdir, copyFile } from 'node:fs/promises'
import chalk from 'chalk'

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
  return rm(target, { force: true, recursive: true })
}

// fsPromises.cp is experimental
export async function cp(source: string, target: string) {
  const ignore = ['.git', '.DS_Store', 'node_modules']
  const sourceDir = await opendir(source)
  await mkdir(target)
  for await (const dirent of sourceDir) {
    if (ignore.includes(dirent.name)) {
      continue
    }
    const s = join(source, dirent.name)
    const t = join(target, dirent.name)
    if (dirent.isDirectory()) {
      await cp(s, t)
    } else if (dirent.isFile()) {
      await copyFile(s, t)
    }
  }
}

export function hasInvalidFlag(allows: string[], flags: string[]) {
  if (flags.length === 0) {
    return false
  }
  return allows.some((item) => !flags.includes(item))
}

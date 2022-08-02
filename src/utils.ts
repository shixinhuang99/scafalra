import * as fsp from 'node:fs/promises'
import * as path from 'node:path'
import * as os from 'node:os'
import { randomBytes } from 'node:crypto'
import { inspect } from 'node:util'
import createHttpsProxyAgent from 'https-proxy-agent'

export function rmrf(target: string) {
  return fsp.rm(target, { force: true, recursive: true })
}

const ignoreFileOrDirs = new Set(['.git', '.DS_Store', 'node_modules'])

export async function cp(source: string, target: string) {
  const dirents = await fsp.readdir(source, { withFileTypes: true })
  await fsp.mkdir(target)
  for (const dirent of dirents) {
    if (ignoreFileOrDirs.has(dirent.name)) {
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

export function uniq(arr: string[]) {
  return Array.from(new Set(arr))
}

export const scafalraPath = path.join(
  os.homedir() ?? os.tmpdir(),
  process.env.NODE_ENV ? `.scafalra-${process.env.NODE_ENV}` : '.scafalra'
)

export const agent = (() => {
  const { https_proxy, http_proxy } = process.env
  const proxy = https_proxy ?? http_proxy
  return proxy ? createHttpsProxyAgent(proxy) : undefined
})()

export function randomString() {
  return randomBytes(3).toString('hex')
}

export function hasOwn(obj: object, key: string) {
  return Object.prototype.hasOwnProperty.call(obj, key)
}

export function printObject(obj: object) {
  return inspect(obj, { colors: true, compact: false, depth: 1 })
}

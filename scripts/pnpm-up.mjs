import { spawn } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

function print(data) {
  process.stdout.write(data)
}

/**
 * @param {string} version
 * @returns {Promise<number>}
 */
function runCorepack(version) {
  return new Promise((resolve, reject) => {
    const corepack = spawn('corepack', ['prepare', `pnpm@${version}`, '--activate'])
    corepack.stdout.on('data', print)
    corepack.stderr.on('data', print)
    corepack.on('error', (err) => reject(err))
    corepack.on('exit', (code) => resolve(code))
  })
}

/**
 * @param {string} version
 */
async function wirtePackageJson(version) {
  const pkgPath = join(process.cwd(), './package.json')
  const pkgJson = await readFile(pkgPath, 'utf-8')
  const end = pkgJson.lastIndexOf('}')
  const emptyLine = pkgJson.slice(end + 1)
  const pkgObj = JSON.parse(pkgJson)
  pkgObj.packageManager = `pnpm@${version}`
  await writeFile(pkgPath, JSON.stringify(pkgObj, null, 2) + emptyLine)
}

async function main() {
  const version = process.argv[2]
  if (!version) {
    return console.log('usage: pnpm-up <version>')
  }
  const code = await runCorepack(version)
  if (code !== 0) {
    return
  }
  await wirtePackageJson(version)
}

main()

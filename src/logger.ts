import chalk from 'chalk'

export type Usage = 'ADD' | 'REMOVE' | 'CREATE' | 'MV' | 'LIST' | 'CONFIG'

export class Logger {
  private static readonly notTTY = !process.stdout.isTTY

  static readonly usageMap: Record<Usage, string> = {
    LIST: 'list [--show-more]',
    ADD: 'add <repo> [--depth <0|1>] [--name <name>]',
    REMOVE: 'remove <name ...>',
    CREATE: 'create <name|repo> [<directory>]',
    MV: 'mv <old-name> <new-name>',
    CONFIG: 'config <token> [<value>]',
  }

  static readonly commandsTable: [string, string][] = [
    ['[-h|--help]', ''],
    ['[-v|--version]', ''],
    [Logger.usageMap.LIST, 'List all projects.'],
    [Logger.usageMap.ADD, 'Add projects with GitHub repository.'],
    [Logger.usageMap.REMOVE, 'Remove projects'],
    [Logger.usageMap.CREATE, 'Create a project from list'],
    [Logger.usageMap.MV, 'Rename a project.'],
    [Logger.usageMap.CONFIG, 'Command-line configuration'],
  ]

  static info(msg: string) {
    console.log(`${chalk.bold.cyan('INFO')}: ${msg}`)
  }

  static error(msg: string) {
    console.log(`${chalk.bold.red('ERROR')}: ${msg}`)
  }

  static usage(usage: Usage) {
    console.log(`${chalk.bold.cyan('USAGE')}: scafalra ${Logger.usageMap[usage]}`)
  }

  static grid(msgs: [string, string][], space = 4) {
    const max = Math.max(...msgs.map((val) => val[0].length))
    const output = msgs
      .map(([left, right]) => {
        if (!right) {
          return left
        }
        return `${left.padEnd(max + space)}${right}`
      })
      .join('\n')
    console.log(output)
  }

  static write(msg: string) {
    if (Logger.notTTY) {
      return
    }
    process.stdout.write(msg)
  }

  static clear() {
    if (Logger.notTTY) {
      return
    }
    process.stdout.clearLine(0)
    process.stdout.cursorTo(0)
  }
}

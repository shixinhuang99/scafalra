#!/usr/bin/env node
import { cac } from 'cac';
import { Logger } from './logger.js';
import { ScafalraError } from './error.js';
import { Scafalra } from './scafalra.js';

async function main() {
  const cli = cac('scafalra');
  const scafalra = new Scafalra();

  cli
    .command('list', 'List all projects')
    .alias('ls')
    .option('--show-more', 'List all projects with detail')
    .action((options) => scafalra.list(options));

  cli
    .command('remove <...names>', 'Remove projects')
    .alias('rm')
    .action((names) => scafalra.remove(names));

  cli
    .command('mv <name> <new-name>', 'Rename a project')
    .action((name, newName) => scafalra.mv(name, newName));

  cli
    .command('config [key] [value]', 'Configuration(available: token)')
    .example('`config token your-token` set a new token')
    .example('`config token` display the token')
    .example('`config` diplay all of configuration')
    .action((key, value) => scafalra.config(key, value));

  cli
    .command('add <repo>', 'Add projects with GitHub repository')
    .option(
      '-D, --depth <depth>',
      'The depth to go when recursing repo(only support 0 or 1)',
      { default: 0 },
    )
    .option(
      '-N, --name <name>',
      'Project name, use the repo name by default(only for depth 0)',
    )
    .action((repo, options) => scafalra.add(repo, options));

  cli
    .command(
      'create <name-or-repo> [directory]',
      'Create a project by name or repo to the specified directory(defaults to the current directory)',
    )
    .action((input, directory) => scafalra.create(input, directory));

  cli.help();
  cli.version(Scafalra.version);

  if (process.argv.length === 2) {
    return cli.outputHelp();
  }

  cli.parse(process.argv, { run: false });

  try {
    await scafalra.init();
    await cli.runMatchedCommand();
  } catch (err) {
    Logger.clear();
    if (
      err instanceof ScafalraError ||
      (err instanceof Error && err.name === 'CACError')
    ) {
      return Logger.error(err.message);
    }
    process.exitCode = 1;
    console.error(err);
  }
}

main();

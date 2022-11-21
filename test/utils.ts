import { execFile } from 'node:child_process';
import { join } from 'node:path';
import * as fsp from 'node:fs/promises';
import { type Dirent } from 'node:fs';
import { scafalraPath, rmrf, printObject } from '../src/utils.js';
import { Logger as BaseLogger, type Usage } from '../src/logger.js';
import { Store as BaseStore, type ScafalraItem } from '../src/store.js';
import { UserConfig as BaseUserConfig } from '../src/user-config.js';

type Command =
  | 'list'
  | 'add'
  | 'create'
  | 'remove'
  | 'mv'
  | 'config'
  | ''
  | 'foo';

const bin = join(process.cwd(), 'dist', 'cli.js');

export function cli(command: Command, options: string[] = []) {
  return new Promise<string>((resolve, reject) => {
    const args = command ? [bin, command, ...options] : [bin, ...options];
    execFile('node', args, (err, stdout, stderr) => {
      if (err) {
        return reject(err);
      }
      if (stderr) {
        return reject(new Error(stderr));
      }
      resolve(stdout.trim());
    });
  });
}

export class Logger extends BaseLogger {
  static info(msg: string) {
    return `INFO: ${msg}`;
  }

  static error(msg: string) {
    return `ERROR: ${msg}`;
  }

  static usage(usage: Usage) {
    return `USAGE: scafalra ${Logger.usageMap[usage]}`;
  }

  static grid(msgs: [string, string][], space = 4) {
    if (!msgs.length) {
      throw new Error('empty msgs');
    }
    const max = Math.max(...msgs.map((val) => val[0].length));
    const output = msgs
      .map(([left, right]) => {
        if (!right) {
          return left;
        }
        return `${left.padEnd(max + space)}${right}`;
      })
      .join('\n');
    return output;
  }
}

export class Store extends BaseStore {
  constructor() {
    super();
  }

  async set(content: [string, ScafalraItem][]) {
    this.content = new Map(content);
    await this.save();
  }

  async clear() {
    this.content = new Map();
    await this.save();
  }

  async getContent() {
    await this.init();
    return this.content;
  }
}

export class CacheController {
  private readonly path = join(scafalraPath, 'cache');

  init() {
    return fsp.mkdir(this.path);
  }

  join(...paths: string[]) {
    return join(this.path, ...paths);
  }

  mkdirs(...paths: string[]) {
    return Promise.all(paths.map((val) => fsp.mkdir(join(this.path, val))));
  }

  readdir(withFileTypes: true): Promise<Dirent[]>;
  readdir(): Promise<string[]>;
  readdir(withFileTypes?: boolean) {
    if (withFileTypes) {
      return fsp.readdir(this.path, { withFileTypes: true });
    }
    return fsp.readdir(this.path);
  }

  async clear() {
    await rmrf(this.path);
    await fsp.mkdir(this.path);
  }
}

export class UserConfig extends BaseUserConfig {
  constructor() {
    super();
  }

  async clear() {
    await this.set('token', '').save();
  }

  list(): string {
    return printObject(this.content);
  }
}

import * as fsp from 'node:fs/promises';
import * as path from 'node:path';
import { existsSync } from 'node:fs';
import chalk from 'chalk';
import { rmrf, scafalraRootDir, printObject } from './utils.js';
import { Logger } from './logger.js';
import { ScafalraError } from './error.js';

export interface ScafalraItem {
  input: string;
  url: string;
  sha: string;
  local: string;
}

const logSymbols = {
  add: chalk.green('+'),
  remove: chalk.red('-'),
  error: chalk.red('✖'),
} as const;

export class Store {
  private readonly path = path.join(scafalraRootDir, 'store.json');

  protected content: Map<string, ScafalraItem> = new Map();

  private readonly changes: Map<string, string> = new Map();

  async init() {
    if (!existsSync(this.path)) {
      await this.save();
      return;
    }
    const raw = await fsp.readFile(this.path, 'utf-8');
    const content: [string, ScafalraItem][] = JSON.parse(raw);
    this.content = new Map(content);
  }

  async save() {
    const content = [...this.content];
    await fsp.writeFile(this.path, JSON.stringify(content, null, 2));
  }

  add(name: string, itemInit: ScafalraItem) {
    this.content.set(name, itemInit);
    this.changes.set(`${logSymbols.add} ${name}`, '');
  }

  async remove(name: string) {
    const item = this.content.get(name);
    if (!item) {
      this.changes.set(
        `${logSymbols.error} ${name}`,
        ScafalraError.itemNotExists(name).message,
      );
      return;
    }
    await rmrf(item.local);
    this.changes.set(`${logSymbols.remove} ${name}`, '');
    this.content.delete(name);
  }

  async removeOld(name: string, oldLocalPath: string) {
    await rmrf(oldLocalPath);
    this.changes.set(`${logSymbols.remove} ${name}(${chalk.gray('old')})`, '');
  }

  rename(name: string, newName: string) {
    const item = this.content.get(name);
    if (!item) {
      throw ScafalraError.itemNotExists(name);
    }
    if (this.content.has(newName)) {
      throw ScafalraError.itemExists(newName);
    }
    this.content.set(newName, item);
    this.content.delete(name);
  }

  printChanges() {
    Logger.grid([...this.changes]);
  }

  printList(showMore?: boolean) {
    if (showMore) {
      Logger.grid(
        [...this.content].map(([name, item]) => {
          return [name, printObject(item)];
        }),
      );
      return;
    }
    Logger.grid(
      [...this.content.keys()].map((name) => {
        return [name, ''];
      }),
    );
  }

  has(name: string) {
    return this.content.has(name);
  }

  get(name: string) {
    return this.content.get(name);
  }
}

import * as fsp from 'node:fs/promises';
import * as path from 'node:path';
import { existsSync } from 'node:fs';
import { scafalraPath, hasOwn, printObject } from './utils.js';

interface UserConfigContent {
  token: string;
}

export class UserConfig {
  private readonly path = path.join(scafalraPath, 'config.json');

  protected content: UserConfigContent = { token: '' };

  async init() {
    if (!existsSync(this.path)) {
      await fsp.writeFile(this.path, JSON.stringify(this.content, null, 2));
      return;
    }
    const rawJson = await fsp.readFile(this.path, 'utf-8');
    this.content = JSON.parse(rawJson);
  }

  private validate(key: string): key is keyof UserConfigContent {
    return hasOwn(this.content, key);
  }

  set(key: string, value: string) {
    if (this.validate(key)) {
      this.content[key] = value;
    }
    return this;
  }

  get(key: string) {
    if (!this.validate(key)) {
      return null;
    }
    return this.content[key];
  }

  list() {
    console.log(printObject(this.content));
  }

  async save() {
    await fsp.writeFile(this.path, JSON.stringify(this.content, null, 2));
  }

  get token() {
    return this.content.token;
  }
}

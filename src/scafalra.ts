import * as path from 'node:path';
import * as fsp from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { cp, scafalraRootDir, uniq } from './utils.js';
import { Logger } from './logger.js';
import { ScafalraError } from './error.js';
import { Store } from './store.js';
import { Repository } from './repository.js';
import { GitHubGraphQLApi } from './github-graphql-api.js';
import { UserConfig } from './user-config.js';

export class Scafalra {
  private readonly cacheDir = path.join(scafalraRootDir, 'cache');
  private readonly store = new Store();
  private readonly userConfig = new UserConfig();
  private readonly githubApi = new GitHubGraphQLApi();

  static get version() {
    return '0.5.0';
  }

  async init() {
    if (!existsSync(scafalraRootDir)) {
      await fsp.mkdir(scafalraRootDir);
      await fsp.mkdir(this.cacheDir);
    }
    await this.userConfig.init();
    await this.store.init();
    this.githubApi.setToken(this.userConfig.token);
  }

  async list(options: { showMore?: boolean }) {
    this.store.printList(options.showMore);
  }

  async add(input: string, options: { depth?: number; name?: string }) {
    const { depth = 0, name } = options;
    Logger.write('Download...');
    const repo = new Repository(input);
    const apiRes = await this.githubApi.get(repo);
    const finalName =
      name ?? (repo.subdir ? path.basename(repo.subdir) : repo.name);
    const oldLocalPath = this.store.get(finalName)?.local;
    const finalPath = await repo.download(
      this.cacheDir,
      apiRes.zipballUrl,
      finalName,
    );
    const scafalraItem = { input, url: apiRes.url, sha: apiRes.oid };
    if (depth === 0) {
      this.store.add(finalName, { ...scafalraItem, local: finalPath });
    }
    if (depth === 1) {
      const dirents = await fsp.readdir(finalPath, { withFileTypes: true });
      for (const dirent of dirents) {
        if (
          dirent.isDirectory() &&
          dirent.name[0] !== '.' &&
          dirent.name !== 'node_modules'
        ) {
          this.store.add(dirent.name, {
            ...scafalraItem,
            local: path.join(finalPath, dirent.name),
          });
        }
      }
    }
    if (oldLocalPath) {
      await this.store.removeOld(finalName, oldLocalPath);
    }
    Logger.clear();
    await this.store.save();
    this.store.printChanges();
  }

  async remove(names: string[]) {
    await Promise.all(uniq(names).map((name) => this.store.remove(name)));
    await this.store.save();
    this.store.printChanges();
  }

  async create(input: string, directory?: string) {
    const cwd = process.cwd();
    const printResult = (finalPath: string) => {
      Logger.clear();
      Logger.info(`Project created in '${finalPath}'.`);
    };
    Logger.write('Creating project...');
    if (Repository.isRepo(input)) {
      const repo = new Repository(input);
      const apiRes = await this.githubApi.get(repo);
      const targetPath = (() => {
        if (directory) {
          return path.isAbsolute(directory)
            ? directory
            : path.join(cwd, directory);
        }
        return path.join(cwd, repo.name);
      })();
      const finalPath = await repo.download(
        path.dirname(targetPath),
        apiRes.zipballUrl,
        path.basename(targetPath),
      );
      printResult(finalPath);
    } else {
      const item = this.store.get(input);
      if (!item) {
        Logger.clear();
        return Logger.error(ScafalraError.itemNotExists(input).message);
      }
      const finalPath = (() => {
        if (directory) {
          return path.isAbsolute(directory)
            ? directory
            : path.join(cwd, directory);
        }
        return path.join(cwd, input);
      })();
      await cp(item.local, finalPath);
      printResult(finalPath);
    }
  }

  async mv(name: string, newName: string) {
    if (name === newName) {
      return;
    }
    this.store.rename(name, newName);
    await this.store.save();
  }

  async config(key: string | undefined, value: string | undefined) {
    if (!key) {
      return this.userConfig.list();
    }
    if (value) {
      await this.userConfig.set(key, value).save();
      return;
    }
    const currentValue = this.userConfig.get(key);
    if (currentValue) {
      console.log(currentValue);
    }
  }
}

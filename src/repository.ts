import * as fsp from 'node:fs/promises';
import * as path from 'node:path';
import * as https from 'node:https';
import * as fs from 'node:fs';
import StreamZip from 'node-stream-zip';
import { agent, rmrf, cp, randomString } from './utils.js';
import { ScafalraError } from './error.js';
import type { Query, QueryType } from './types.js';

export class Repository {
  // owner/name/...subdir?query
  private static readonly regexp =
    /^([^/\s]+)\/([^/\s?]+)(?:((?:\/[^/\s?]+)+))?(?:\?(branch|tag|commit)=(.+))?$/;

  private readonly validQueryTypes = new Set(['branch', 'tag', 'commit']);

  readonly owner: string;

  readonly name: string;

  readonly subdir?: string;

  readonly query?: Query;

  constructor(src: string) {
    const match = src.match(Repository.regexp);
    if (!match) {
      throw new ScafalraError(`Could not parse the input: '${src}'.`);
    }
    this.owner = match[1];
    this.name = match[2];
    this.subdir = match[3];
    const queryType = match[4];
    if (queryType && this.isValidRefType(queryType)) {
      this.query = { type: queryType, value: match[5] };
    }
  }

  private isValidRefType(value: string): value is QueryType {
    return this.validQueryTypes.has(value);
  }

  private dowloadZipball(url: string, zipballFile: string) {
    return new Promise<string>((resolve, reject) => {
      https
        .get(url, { agent }, (res) => {
          const { statusCode, statusMessage } = res;
          if (!statusCode) {
            return reject(new Error('No response.'));
          }
          if (statusCode < 300 && statusCode >= 200) {
            return res
              .pipe(fs.createWriteStream(zipballFile))
              .on('finish', () => resolve(zipballFile))
              .on('error', reject);
          }
          const err = new Error(statusMessage);
          err.name = statusCode.toString();
          return reject(err);
        })
        .on('error', reject);
    });
  }

  private async unzip(zipballFile: string) {
    const { dir, name } = path.parse(zipballFile);
    const zip = new StreamZip.async({ file: zipballFile });
    const tempParentDir = path.join(dir, name);
    await zip.extract(null, tempParentDir);
    await zip.close();
    await fsp.rm(zipballFile);
    return tempParentDir;
  }

  private async move(tempParentDir: string, dirName: string) {
    const [extracted] = await fsp.readdir(tempParentDir);
    const sourcePath = this.subdir
      ? path.join(tempParentDir, extracted, this.subdir)
      : path.join(tempParentDir, extracted);
    if (!fs.existsSync(sourcePath)) {
      throw new ScafalraError(`No such directory: '${sourcePath}'`);
    }
    const finalPath = path.join(tempParentDir, '..', dirName);
    await cp(sourcePath, finalPath);
    await rmrf(tempParentDir);
    return finalPath;
  }

  async download(parentDir: string, zipballUrl: string, dirName: string) {
    const zipballFile = await this.dowloadZipball(
      zipballUrl,
      path.join(parentDir, `${randomString()}.zip`),
    );
    const tempParentDir = await this.unzip(zipballFile);
    const finalPath = await this.move(tempParentDir, dirName);
    return finalPath;
  }

  static isRepo(src: string) {
    return Repository.regexp.test(src);
  }
}

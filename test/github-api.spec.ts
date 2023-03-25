import { describe, test, expect, beforeAll } from 'vitest';
import * as fsp from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import * as path from 'node:path';
import { GitHubApi, type ApiResult } from '../src/github-api.js';

interface Tokens {
  noAnyScope: string;
  withRepoScope: string;
}

function isValidResult(obj: ApiResult) {
  const { oid, zipballUrl, url } = obj;
  return [oid, zipballUrl, url].every((v) => typeof v === 'string');
}

describe('GitHub api', () => {
  let tokens: Tokens | null = null;

  const api = new GitHubApi();

  beforeAll(async () => {
    const tokenFilePath = path.join(
      fileURLToPath(import.meta.url),
      '..',
      'token.json',
    );

    if (!existsSync(tokenFilePath)) {
      return;
    }

    const raw = await fsp.readFile(tokenFilePath, 'utf-8');
    tokens = JSON.parse(raw);
  });

  test.runIf(tokens)('get a public repository', async () => {
    api.setToken(tokens!.noAnyScope);
    const res = await api.get({ owner: 'shixinhuang99', name: 'scafalra' });
    expect(res).toSatisfy(isValidResult);
  });

  test.runIf(tokens)('get a private repository', async () => {
    api.setToken(tokens!.withRepoScope);
    const res = await api.get({
      owner: 'shixinhuang99',
      name: 'scafalra-private-repo-test',
    });
    expect(res).toSatisfy(isValidResult);
  });
});

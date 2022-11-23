import {
  describe,
  test,
  expect,
  beforeAll,
  afterAll,
  beforeEach,
} from 'vitest';
import * as fsp from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { inspect } from 'node:util';
import { scafalraRootDir, rmrf } from '../src/utils.js';
import { type ScafalraItem } from '../src/store.js';
import { cli, Store, CacheController, UserConfig, Logger } from './utils.js';
import { tokens } from './token.js';

const store = new Store();
const cacheController = new CacheController();
const userConfig = new UserConfig();

beforeAll(async () => {
  if (existsSync(scafalraRootDir)) {
    await rmrf(scafalraRootDir);
  }
  await fsp.mkdir(scafalraRootDir);
  await store.init();
  await cacheController.init();
  await userConfig.init();

  return async () => {
    await rmrf(scafalraRootDir);
  };
});

describe('none action', () => {
  test('none', async () => {
    const stdout = await cli('');
    console.log(stdout);
    expect(stdout).not.toBe('');
  });

  test('version', async () => {
    const stdout = await cli('', ['-v']);
    const packageJson = await fsp.readFile(
      new URL('../package.json', import.meta.url),
      'utf-8',
    );
    const packageObj = JSON.parse(packageJson);
    const index = stdout.indexOf(' ');
    expect(stdout.slice(0, index)).toBe(`scafalra/${packageObj.version}`);
  });

  test('help', async () => {
    const stdout = await cli('', ['-h']);
    expect(stdout).not.toBe('');
  });

  test('unknown command', async () => {
    const stdout = await cli('foo');
    expect(stdout).toBe('');
  });

  test('unknown option', async () => {
    const stdout = await cli('', ['--foo']);
    expect(stdout).toBe('');
  });
});

describe('list', () => {
  const list: [string, ScafalraItem][] = [
    ['a', { input: '', url: '', sha: '', local: cacheController.join('a') }],
    ['b', { input: '', url: '', sha: '', local: cacheController.join('b') }],
  ];

  beforeAll(async () => {
    await store.set(list);
    await cacheController.mkdirs('a');

    return async () => {
      await store.clear();
      await cacheController.clear();
    };
  });

  test('no option', async () => {
    const stdout = await cli('list');
    expect(stdout).toBe(Logger.grid(list.map(([name]) => [name, ''])));
  });

  test('show more', async () => {
    const stdout = await cli('list', ['--show-more']);
    expect(stdout).toBe(
      Logger.grid(
        list.map(([name, item]) => {
          return [
            name,
            inspect(item, { colors: true, compact: false, depth: 1 }),
          ];
        }),
      ),
    );
  });
});

describe('config', () => {
  afterAll(async () => {
    await userConfig.clear();
  });

  test('no arguments', async () => {
    const stdout = await cli('config');
    expect(stdout).toBe(userConfig.list());
  });

  test('argument is not token', async () => {
    const stdout = await cli('config', ['foo']);
    expect(stdout).toBe('');
  });

  test('check the token but nothing', async () => {
    const stdout = await cli('config', ['token']);
    expect(stdout).toBe('');
  });

  test('set token and check the token', async () => {
    const stdout = await cli('config', ['token', tokens.noAnyScope]);
    expect(stdout).toBe('');
    const stdout2 = await cli('config', ['token']);
    expect(stdout2).toBe(tokens.noAnyScope);
  });
});

const repoForTest = {
  owner: 'shixinhuang99',
  name: 'scafalra-private-repo-test',
  input: 'shixinhuang99/scafalra-private-repo-test',
  url: 'https://github.com/shixinhuang99/scafalra-private-repo-test',
  shaOfDefaultBranch: '9d529472b77f9b211795659cba453ca5a726d46d',
  shaOfCommit: '83054010d567fc4f61cfe9652206b760ac768c64',
  shaOfAnotherBranch: '38dc4a33a13f9a89e4353d4497dcc66e6d51265a',
  shaOfTag: 'fd24a92268d758a6ca2a400521d09ad35264f044',
};

describe('add', () => {
  test('no argument', async () => {
    const stdout = await cli('add');
    expect(stdout).toBe(
      Logger.error('missing required args for command `add <repo>`'),
    );
  });

  test('invalid repo', async () => {
    const stdout = await cli('add', ['foo/bar?foo=bar']);
    expect(stdout).toBe(
      Logger.error(`Could not parse the input: 'foo/bar?foo=bar'.`),
    );
  });

  test('no token', async () => {
    const stdout = await cli('add', [repoForTest.input]);
    expect(stdout).toBe(
      Logger.error('GitHub personal access token is not configured.'),
    );
  });
});

describe('add with token(no any scope)', () => {
  beforeAll(async () => {
    await userConfig.set('token', tokens.noAnyScope).save();

    return async () => {
      await userConfig.clear();
    };
  });

  test('add a private repository', async () => {
    expect(() => cli('add', [repoForTest.input])).rejects.toThrowError();
  });
});

describe('add with token(repo scope)', () => {
  beforeAll(async () => {
    await userConfig.set('token', tokens.withRepoScope).save();

    return async () => {
      await userConfig.clear();
    };
  });

  beforeEach(async () => {
    await Promise.all([store.clear(), cacheController.clear()]);
  });

  test('add repository with no options', async () => {
    const { input, name, url, shaOfDefaultBranch } = repoForTest;
    const stdout = await cli('add', [input]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const dirs = await cacheController.readdir();
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get(name);
    expect(item).not.toBeUndefined();
    expect(item).toStrictEqual({
      input,
      url,
      sha: shaOfDefaultBranch,
      local: cacheController.join(local),
    } as ScafalraItem);
  });

  test('with same name', async () => {
    const { input, name } = repoForTest;
    const stdout = await cli('add', [input]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    expect(cacheController.readdir()).resolves.toHaveLength(1);
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const { ...item } = storeContent.get(name);
    const stdout2 = await cli('add', [input]);
    expect(stdout2).toBe(
      Logger.grid([
        [`+ ${name}`, ''],
        [`- ${name}(old)`, ''],
      ]),
    );
    expect(cacheController.readdir()).resolves.toHaveLength(1);
    const storeContent2 = await store.getContent();
    expect(storeContent2.size).toBe(1);
    const item2 = storeContent2.get(name);
    expect(item.local).not.toBe(item2?.local);
  });

  test('with --name', async () => {
    const name = 'foo';
    const stdout = await cli('add', [repoForTest.input, '--name', name]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const storeContent = await store.getContent();
    expect(storeContent.has(name)).toBe(true);
    expect(cacheController.readdir()).resolves.toHaveLength(1);
  });

  test('with depth 1', async () => {
    const { input, url, shaOfDefaultBranch } = repoForTest;
    const stdout = await cli('add', [input, '--depth', '1']);
    expect(stdout).toBe(
      Logger.grid(['a', 'b', 'c'].map((val) => [`+ ${val}`, ''])),
    );
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(3);
    const a = storeContent.get('a');
    const b = storeContent.get('b');
    const c = storeContent.get('c');
    const comparable = {
      input: repoForTest.input,
      url,
      sha: shaOfDefaultBranch,
    };
    expect(a).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'a'),
    });
    expect(b).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'b'),
    });
    expect(c).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'c'),
    });
  });

  test('with subdir', async () => {
    const { input, url, shaOfDefaultBranch } = repoForTest;
    const stdout = await cli('add', [`${input}/a`]);
    expect(stdout).toBe(Logger.grid([['+ a', '']]));
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get('a');
    const comparable = {
      input: `${input}/a`,
      url,
      sha: shaOfDefaultBranch,
      local: cacheController.join(local),
    };
    expect(item).toStrictEqual(comparable);
  });

  test('with subdir and depth 1', async () => {
    const { input, url, shaOfDefaultBranch } = repoForTest;
    const stdout = await cli('add', [`${input}/a`, '--depth', '1']);
    expect(stdout).toBe(
      Logger.grid(['a1', 'a2', 'a3'].map((val) => [`+ ${val}`, ''])),
    );
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(3);
    const a1 = storeContent.get('a1');
    const a2 = storeContent.get('a2');
    const a3 = storeContent.get('a3');
    const comparable = { input: `${input}/a`, url, sha: shaOfDefaultBranch };
    expect(a1).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'a1'),
    });
    expect(a2).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'a2'),
    });
    expect(a3).toStrictEqual({
      ...comparable,
      local: cacheController.join(local, 'a3'),
    });
  });

  test('with subdir and --name', async () => {
    const { input, url, shaOfDefaultBranch } = repoForTest;
    const name = 'foo';
    const stdout = await cli('add', [`${input}/a`, '--name', name]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get(name);
    const comparable = {
      input: `${input}/a`,
      url,
      sha: shaOfDefaultBranch,
      local: cacheController.join(local),
    };
    expect(item).toStrictEqual(comparable);
  });

  test('with branch', async () => {
    const { input, name, url, shaOfAnotherBranch } = repoForTest;
    const stdout = await cli('add', [`${input}?branch=another-branch`]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get(name);
    const comparable = {
      input: `${input}?branch=another-branch`,
      url,
      sha: shaOfAnotherBranch,
      local: cacheController.join(local),
    };
    expect(item).toStrictEqual(comparable);
  });

  test('with tag', async () => {
    const { input, name, url, shaOfTag } = repoForTest;
    const stdout = await cli('add', [`${input}?tag=v1.0.0`]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get(name);
    const comparable = {
      input: `${input}?tag=v1.0.0`,
      url,
      sha: shaOfTag,
      local: cacheController.join(local),
    };
    expect(item).toStrictEqual(comparable);
  });

  test('with commit', async () => {
    const { input, name, url, shaOfCommit } = repoForTest;
    const stdout = await cli('add', [`${input}?commit=${shaOfCommit}`]);
    expect(stdout).toBe(Logger.grid([[`+ ${name}`, '']]));
    const dirs = await cacheController.readdir();
    expect(dirs).toHaveLength(1);
    const local = dirs[0];
    const storeContent = await store.getContent();
    expect(storeContent.size).toBe(1);
    const item = storeContent.get(name);
    const comparable = {
      input: `${input}?commit=${shaOfCommit}`,
      url,
      sha: shaOfCommit,
      local: cacheController.join(local),
    };
    expect(item).toStrictEqual(comparable);
  });
});

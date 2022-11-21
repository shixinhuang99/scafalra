import * as fsp from 'node:fs/promises';
import * as path from 'node:path';

async function main() {
  const parentDirPath = process.argv[2];
  if (!parentDirPath) {
    throw new Error('no argument');
  }
  await fsp.mkdir(path.join(parentDirPath, 'node_modules'));
  await fsp.writeFile(
    path.join(parentDirPath, 'node_modules', 'foo.txt'),
    'foo',
  );
  const depth1 = ['a', 'b', 'c', '.d'];
  await Promise.all(
    depth1.map(async (e1) => {
      await fsp.mkdir(path.join(parentDirPath, e1));
      const depth2 = [`${e1}1`, `${e1}2`, `${e1}3`];
      await Promise.all(
        depth2.map(async (e2) => {
          await fsp.mkdir(path.join(parentDirPath, e1, e2));
          await fsp.writeFile(
            path.join(parentDirPath, e1, e2, `${e2}.txt`),
            'foo',
          );
        }),
      );
    }),
  );
}

main();

import { rm } from 'node:fs/promises';
import { join } from 'node:path';

function rmrf(filename) {
  return rm(join(process.cwd(), filename), {
    recursive: true,
    force: true,
  });
}

await Promise.all(process.argv.slice(2).map(rmrf));

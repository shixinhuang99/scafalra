import { rm } from 'node:fs/promises';

function rmrf(filename) {
  return rm(new URL(filename, import.meta.url), {
    recursive: true,
    force: true,
  });
}

await rmrf('../dist');

await rmrf('../debug');

await rmrf('../coverage');

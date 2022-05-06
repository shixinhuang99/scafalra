import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['src/cli.ts', 'src/utils.ts'],
  clean: true,
  minify: true,
  format: ['cjs'],
  target: 'node16',
})

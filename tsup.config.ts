import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['src/index.ts', 'src/utils.ts'],
  clean: true,
  minify: true,
  format: ['cjs'],
  target: 'node16',
})

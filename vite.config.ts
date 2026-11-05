/// <reference types="vitest/config" />
import { defineConfig } from 'vite';

// GitHub Pages serves the project from /dhool/; everywhere else it lives at /.
const base = process.env.GITHUB_PAGES === 'true' ? '/dhool/' : '/';

export default defineConfig({
  root: 'web',
  base,
  publicDir: 'public',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    // Never inline the .wasm as a data: URL; instantiateStreaming wants a real fetch.
    assetsInlineLimit: 0,
  },
  test: {
    root: '.',
    include: ['web/src/**/*.test.ts'],
    environment: 'node',
  },
});

// Builds the Rust core for wasm32 and copies the module next to the front end,
// where Vite picks it up as a hashed asset. Plain cargo: no wasm-bindgen, no wasm-pack.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const target = 'wasm32-unknown-unknown';

execFileSync('cargo', ['build', '--release', '--target', target, '-p', 'dhool-sim'], {
  cwd: root,
  stdio: 'inherit',
});

const from = join(root, 'target', target, 'release', 'dhool_sim.wasm');
const to = join(root, 'web', 'src', 'wasm', 'dhool.wasm');
mkdirSync(dirname(to), { recursive: true });
copyFileSync(from, to);
console.log(`wasm: ${(statSync(to).size / 1024).toFixed(1)} KiB -> web/src/wasm/dhool.wasm`);

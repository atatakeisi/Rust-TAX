#!/usr/bin/env node
// Build the complete Rust UI and its calculation engine into a browser Wasm module.
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const local = join(root, '.tools', 'bin', 'wasm-pack');
const command = existsSync(local) ? local : 'wasm-pack';
try {
  execFileSync(command, ['build', 'frontend', '--target', 'web', '--out-dir', '../web/pkg', '--release', '--no-opt'], { cwd: root, stdio: 'inherit' });
} catch (error) {
  console.error('Rust UI build failed. Install wasm-pack and the wasm32-unknown-unknown Rust target.');
  process.exit(error.status || 1);
}

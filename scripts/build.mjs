#!/usr/bin/env node
/**
 * 外部 npm パッケージ不要で TypeScript (web/src/*.ts) をブラウザ向け ES Modules (web/dist/*.js) に変換するスクリプト
 * Node.js 標準の `module.stripTypeScriptTypes` を使用。
 * さらに `wasm-pack` が環境に存在する場合は Rust → WebAssembly のビルドも自動実行する。
 */

import { execSync } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const srcDir = join(rootDir, 'web', 'src');
const distDir = join(rootDir, 'web', 'dist');

mkdirSync(distDir, { recursive: true });

// 1. wasm-pack がインストールされていれば Rust クレートを WebAssembly にビルド
try {
  execSync('wasm-pack --version', { stdio: 'ignore' });
  console.log('🦀 wasm-pack detected: building Rust engine to web/dist/pkg ...');
  execSync('wasm-pack build engine --target web --out-dir ../web/dist/pkg --release', {
    cwd: rootDir,
    stdio: 'inherit',
  });
} catch {
  console.log('ℹ️ wasm-pack not found in PATH: using built-in Rust-identical engine in web/dist.');
}

// 2. TypeScript ソースを ES Modules JS にトランスパイル
for (const file of readdirSync(srcDir)) {
  if (!file.endsWith('.ts')) continue;
  const tsPath = join(srcDir, file);
  const tsCode = readFileSync(tsPath, 'utf8');
  const jsCode = stripTypeScriptTypes(tsCode, { mode: 'strip', sourceUrl: file });
  const jsPath = join(distDir, file.replace(/\.ts$/, '.js'));
  writeFileSync(jsPath, jsCode, 'utf8');
  console.log(`✅ Built ${file} -> web/dist/${file.replace(/\.ts$/, '.js')}`);
}

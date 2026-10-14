#!/usr/bin/env node
/**
 * Install a square PNG as Neural Nova's permanent icon source and ask Tauri
 * to regenerate the desktop/mobile icons. Use --push to commit ONLY the icon
 * files and automatically trigger the existing GitHub Actions app workflow.
 * No neural-code, checkpoint, signing-key or visualizer changes.
 */
import { readFileSync, mkdirSync, copyFileSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const app = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(app, '..');
const png = join(app, 'assets', 'nova-brain-icon.png');
const args = process.argv.slice(2);
const push = args.includes('--push');
const filenames = args.filter(value => value !== '--push');
if (filenames.length !== 1) {
  console.error('Usage: npm run icons:apply -- "C:\\path\\to\\brain-logo.png" [--push]');
  process.exit(2);
}
function run(command, arguments_, cwd) {
  const result = spawnSync(command, arguments_, { cwd, stdio: 'inherit', shell: process.platform === 'win32' });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} ${arguments_.join(' ')} failed: ${result.error?.message ?? result.status}`);
  }
}
function capture(command, arguments_, cwd) {
  const result = spawnSync(command, arguments_, { cwd, encoding: 'utf8', shell: process.platform === 'win32' });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} ${arguments_.join(' ')} failed: ${result.stderr || result.error?.message || result.status}`);
  }
  return result.stdout.trim();
}
const file = resolve(process.cwd(), filenames[0]);
const bytes = readFileSync(file);
if (bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a' || bytes.length < 24) {
  throw new Error('The source must be a genuine PNG file; convert other formats to PNG first.');
}
const width = bytes.readUInt32BE(16), height = bytes.readUInt32BE(20);
if (width !== height || width < 512) {
  throw new Error(`The icon must be square and at least 512x512 pixels (received ${width}x${height}).`);
}
// Check branch BEFORE writing or committing, so --push never changes main.
if (push) {
  const branch = capture('git', ['branch', '--show-current'], repo);
  if (branch !== 'experimental/neural-nova') throw new Error(`Expected experimental/neural-nova, on ${branch}`);
  if (capture('git', ['diff', '--cached', '--name-only'], repo)) {
    throw new Error('Commit or unstage other staged files before using --push.');
  }
}
mkdirSync(dirname(png), { recursive: true });
if (resolve(file) !== resolve(png)) copyFileSync(file, png);
console.log(`Using ${width}x${height} PNG: ${png}`);
run('npm', ['run', 'icons:generate'], app);
if (push) {
  run('git', ['add', 'app/assets/nova-brain-icon.png', 'app/src-tauri/icons'], repo);
  const changes = capture('git', ['diff', '--cached', '--name-only'], repo);
  if (!changes) { console.log('Icons already committed.'); process.exit(0); }
  run('git', ['commit', '-m', 'chore(nova): generate launcher icons from dual-hemisphere artwork'], repo);
  run('git', ['push', 'origin', 'experimental/neural-nova'], repo);
  console.log('Pushed new icon. The Neural Nova experimental Windows/Android workflow has been triggered.');
} else {
  console.log('Generated icons locally. Use --push to commit them and trigger the GitHub Actions workflow.');
}

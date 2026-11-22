import { lstat, mkdir, readlink, symlink, unlink, access } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.platform !== 'darwin' && process.platform !== 'linux') {
  throw new Error('Automatic MCP command installation currently supports macOS and Linux.');
}

const project = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const defaultExecutable = process.platform === 'darwin'
  ? join(project, 'src-tauri', 'target', 'release', 'bundle', 'macos', 'Issue Tracker.app', 'Contents', 'MacOS', 'issue_tracker')
  : join(project, 'src-tauri', 'target', 'release', 'issue_tracker');
const supplied = process.argv[2] ? resolve(process.argv[2]) : null;
const executable = supplied?.endsWith('.app')
  ? join(supplied, 'Contents', 'MacOS', 'issue_tracker')
  : supplied || defaultExecutable;
await access(executable);

const binDir = process.env.ISSUE_TRACKER_MCP_INSTALL_DIR || join(homedir(), '.local', 'bin');
const launcher = join(binDir, 'issue-tracker-mcp');
await mkdir(binDir, { recursive: true });
try {
  const current = await lstat(launcher);
  if (!current.isSymbolicLink()) throw new Error(`${launcher} already exists and is not a symlink.`);
  if (await readlink(launcher) === executable) {
    process.stdout.write(`MCP command ready: ${launcher}\n`);
    process.exit(0);
  }
  await unlink(launcher);
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
}
await symlink(executable, launcher);
process.stdout.write(`Installed MCP command: ${launcher}\n`);
if (!process.env.PATH?.split(process.platform === 'win32' ? ';' : ':').includes(binDir)) {
  process.stdout.write(`Add ${binDir} to PATH before configuring your coding agent.\n`);
}

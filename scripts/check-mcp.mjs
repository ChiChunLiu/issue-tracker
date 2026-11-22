import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const child = spawn('issue-tracker-mcp', [], { stdio: ['pipe', 'pipe', 'pipe'] });
const pending = new Map();
let nextId = 1;
let stderr = '';
child.stderr.on('data', chunk => { stderr += chunk.toString(); });
child.on('error', error => { for (const { reject } of pending.values()) reject(error); pending.clear(); });
void (async () => {
  for await (const line of createInterface({ input: child.stdout })) {
    const message = JSON.parse(line);
    if (message.id === undefined) continue;
    const waiter = pending.get(message.id);
    if (!waiter) continue;
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
  }
})();

function request(method, params = {}) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out: ${stderr}`)); }, 10000);
    pending.set(id, {
      resolve: value => { clearTimeout(timeout); resolve(value); },
      reject: error => { clearTimeout(timeout); reject(error); }
    });
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });
}

function call(name, args) {
  return request('tools/call', { name, arguments: args }).then(result => {
    if (result.isError) throw new Error(result.content?.[0]?.text || `${name} failed`);
    return result.structuredContent;
  });
}

try {
  await request('initialize', {
    protocolVersion: '2025-11-25', capabilities: {},
    clientInfo: { name: 'issue-tracker-check', version: '1.0.0' }
  });
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
  const listed = await request('tools/list');
  const names = listed.tools.map(tool => tool.name).sort();
  const expected = ['get_issue', 'get_report', 'list_projects', 'search_issues', 'search_reports'];
  if (JSON.stringify(names) !== JSON.stringify(expected)) throw new Error(`Unexpected tools: ${names}`);
  if (listed.tools.some(tool => tool.annotations?.readOnlyHint !== true)) throw new Error('A tool is not marked read-only.');
  const { projects } = await call('list_projects', {});
  if (!Array.isArray(projects)) throw new Error('Projects response missing.');
  for (const project of projects) {
    const issues = await call('search_issues', { project_id: project.id });
    const reports = await call('search_reports', { project_id: project.id, limit: 1 });
    if (!Array.isArray(issues.issues) || !Array.isArray(reports.reports)) throw new Error('Search response missing.');
    for (const summary of issues.issues.slice(0, 1)) {
      const detail = await call('get_issue', { issue_id: summary.id });
      if (detail.issue.id !== summary.id) throw new Error('Issue detail mismatch.');
      for (const linked of detail.linked_reports.slice(0, 1)) {
        const report = await call('get_report', { report_id: linked.id });
        if (report.report.issue_id !== summary.id || report.issue?.id !== summary.id) {
          throw new Error('Bidirectional report link mismatch.');
        }
      }
    }
  }
  process.stdout.write(`MCP check passed: ${names.length} tools, ${projects.length} projects.\n`);
} finally {
  child.stdin.end();
  child.kill();
}

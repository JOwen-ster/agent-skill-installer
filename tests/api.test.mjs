import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import ts from 'typescript';

const { outputText } = ts.transpileModule(
  readFileSync(new URL('../src/lib/api.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
);

function installer(invoke) {
  const exports = {};
  runInNewContext(outputText, { exports, require: () => ({ invoke }) });
  return exports.installGithubSkills;
}

test('reports each result while the next skill is still pending', async () => {
  const requests = [{ candidate: { name: 'first' } }, { candidate: { name: 'second' } }];
  const pending = [];
  const progress = [];
  const install = installer((command, args) => {
    assert.equal(command, 'install_github_skills');
    assert.equal(args.agent, 'claude');
    assert.equal(args.requests.length, 1);
    assert.equal(args.requests[0], requests[pending.length]);
    return new Promise((resolve) => pending.push(resolve));
  });
  const done = install(requests, 'claude', (result, index) => progress.push({ result, index }));
  assert.equal(pending.length, 1);
  assert.equal(progress.length, 0);

  const first = { candidate_name: 'first', success: true };
  pending[0]([first]);
  await new Promise(setImmediate);
  assert.deepEqual(progress, [{ result: first, index: 0 }]);
  assert.equal(pending.length, 2);

  const second = { candidate_name: 'second', success: false, error: 'Download failed' };
  pending[1]([second]);
  const results = await done;
  assert.equal(results[0], first);
  assert.equal(results[1], second);
  assert.deepEqual(progress, [{ result: first, index: 0 }, { result: second, index: 1 }]);
});

test('propagates worker failures and does not start later skills', async () => {
  let calls = 0;
  const failure = new Error('Install worker failed');
  const install = installer(async () => {
    calls++;
    throw failure;
  });
  await assert.rejects(install([{}, {}], 'codex'), (error) => error === failure);
  assert.equal(calls, 1);
});

test('rejects a missing result instead of reporting success', async () => {
  const install = installer(async () => []);
  await assert.rejects(install([{}], 'codex'), /Installer returned no result/);
});

test('an empty queue does not invoke the backend', async () => {
  const install = installer(() => assert.fail('Unexpected invocation'));
  assert.equal((await install([], 'codex')).length, 0);
});

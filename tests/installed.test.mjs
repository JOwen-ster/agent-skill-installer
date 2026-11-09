import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import ts from 'typescript';

const { outputText } = ts.transpileModule(
  readFileSync(new URL('../src/lib/installed.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
);
const exports = {};
runInNewContext(outputText, { exports, URL });
const { findInstalledGithubSkill: find } = exports;

test('matches the actual GitHub folder name and owner-prefixed installs', () => {
  const plain = { name: 'pdf', source_repo: 'anthropics/skills' };
  const prefixed = { name: 'anthropics-pdf', source_repo: 'anthropics/skills' };
  assert.equal(find([plain], 'https://github.com/anthropics/skills', 'pdf'), plain);
  assert.equal(find([prefixed], 'https://github.com/anthropics/skills', 'pdf'), prefixed);
});

test('does not block another repository with the same skill name', () => {
  const installed = { name: 'pdf', source_repo: 'openai/skills' };
  assert.equal(find([installed], 'https://github.com/anthropics/skills', 'pdf'), undefined);
  assert.equal(find([installed], 'https://github.com/openai/another-repo', 'pdf'), undefined);
});

test('matches folders without source metadata and normalizes spaces and case', () => {
  const installed = { name: 'Anthropics-My-Skill', source_repo: null };
  assert.equal(find([installed], 'https://github.com/Anthropics/skills', 'my skill'), installed);
});

test('accepts full source URLs and repository URLs with branches', () => {
  const installed = { name: 'pdf', source_repo: 'https://github.com/Anthropics/skills/tree/main/skills/pdf' };
  assert.equal(find([installed], 'https://github.com/anthropics/skills.git/tree/main', 'pdf'), installed);
});

test('does not confuse similar folder names, prefixes, or other hosts', () => {
  const installed = [{ name: 'pdf-tools', source_repo: null }, { name: 'openai-pdf', source_repo: null }];
  assert.equal(find(installed, 'https://github.com/anthropics/skills', 'pdf'), undefined);
  assert.equal(find([{ name: 'pdf', source_repo: 'https://other.example/anthropics/skills' }], 'https://github.com/anthropics/skills', 'pdf'), undefined);
  assert.equal(find([{ name: 'pdf', source_repo: null }], 'https://other.example/anthropics/skills', 'pdf'), undefined);
  assert.equal(find([{ name: 'pdf', source_repo: null }], 'bad URL', 'pdf'), undefined);
});

test('ignores source records for folders absent from the live installed list', () => {
  assert.equal(find([], 'https://github.com/anthropics/skills', 'pdf'), undefined);
});

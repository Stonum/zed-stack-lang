// Validate Zed queries against the same parser and representative corpus as syntax tests.
const {spawnSync} = require('node:child_process');
const path = require('node:path');
const cli = path.resolve('node_modules/tree-sitter-cli', process.platform === 'win32' ? 'tree-sitter.exe' : 'tree-sitter');
for (const query of ['highlights', 'brackets', 'indents', 'outline']) {
  const result = spawnSync(cli, ['query', `../../languages/stack/${query}.scm`, '-n', '1', '--quiet'], {
    cwd: path.resolve('grammars/stack'), encoding: 'utf8',
  });
  if (result.status !== 0) {
    process.stderr.write(result.stderr || result.error?.message || 'Query validation failed');
    process.exit(1);
  }
  console.log(`${query}: OK`);
}

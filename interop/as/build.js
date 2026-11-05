// Compile every assembly/*.ts fixture to ../build/as_<name>.wasm with the
// same asc settings as @massalabs/sc-project-initializer.
import { readdirSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

mkdirSync('../build', { recursive: true });
for (const file of readdirSync('assembly').filter((f) => f.endsWith('.ts') && !f.startsWith('_'))) {
  const name = file.replace(/\.ts$/, '');
  const out = `../build/as_${name}.wasm`;
  execFileSync('npx', ['asc', `assembly/${file}`, '-o', out, '--target', 'release', '--config', 'asconfig.json'], {
    stdio: 'inherit',
  });
  console.log(`built ${out}`);
}

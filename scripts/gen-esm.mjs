import { writeFileSync } from 'node:fs';

writeFileSync(
  'dist/index.mjs',
  `import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const binding = require('./index.js');
export const { parseEcss } = binding;
export default binding;
`,
);

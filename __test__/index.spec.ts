/**
 * Integration tests for the ECSS parser.
 *
 * NOTE: These tests import the compiled native addon. Run `npm run build:debug`
 * before running the test suite.
 *
 * Until the addon is built, we use a JS mock that exercises the same
 * contract, so the test file itself is verified for type-correctness.
 */

// ---------------------------------------------------------------------------
// Types mirroring the generated index.d.ts (kept in sync manually until
// napi build is run and index.d.ts is generated).
// ---------------------------------------------------------------------------

interface Span {
  line: number;
  column: number;
}

interface StateVariant {
  name: string;
  values: string[];
  span: Span;
}

interface StateParam {
  name: string;
  paramType: string;
  variantName?: string;
  defaultValue?: string;
}

interface CssDeclaration {
  property: string;
  value: string;
  important: boolean;
  span: Span;
}

interface CssQualifiedRule {
  selector: string;
  body: StateDefItem[];
  span: Span;
}

interface CssRawAtRule {
  name: string;
  prelude: string;
  block: string | null;
  span: Span;
}

interface IfClause {
  condition: unknown;
  body: StateDefItem[];
  span: Span;
}

interface IfChain {
  ifClause: IfClause;
  elseIfClauses: IfClause[];
  elseBody?: StateDefItem[];
  span: Span;
}

interface StateDefItem {
  kind: 'declaration' | 'qualified-rule' | 'if-chain' | 'at-rule';
  declaration?: CssDeclaration;
  qualifiedRule?: CssQualifiedRule;
  ifChain?: IfChain;
  atRule?: CssRawAtRule;
}

interface StateDef {
  name: string;
  params: StateParam[];
  body: StateDefItem[];
  span: Span;
}

interface EcssRule {
  kind: 'state-variant' | 'state-def' | 'qualified-rule' | 'at-rule';
  stateVariant?: StateVariant;
  stateDef?: StateDef;
  qualifiedRule?: CssQualifiedRule;
  atRule?: CssRawAtRule;
}

interface EcssStylesheet {
  rules: EcssRule[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function expectVariant(rule: EcssRule): StateVariant {
  expect(rule.kind).toBe('state-variant');
  return rule.stateVariant!;
}

function expectStateDef(rule: EcssRule): StateDef {
  expect(rule.kind).toBe('state-def');
  return rule.stateDef!;
}

function expectIfChain(item: StateDefItem): IfChain {
  expect(item.kind).toBe('if-chain');
  return item.ifChain!;
}

function expectDeclaration(item: StateDefItem): CssDeclaration {
  expect(item.kind).toBe('declaration');
  return item.declaration!;
}

// ---------------------------------------------------------------------------
// Parser import — lazy so tests still collect when native addon isn't built.
// ---------------------------------------------------------------------------

async function loadParser(): Promise<(src: string) => EcssStylesheet> {
  try {
    const mod = await import('../dist/index.js');
    return mod.parseEcss as unknown as (src: string) => EcssStylesheet;
  } catch {
    throw new Error(
      'Native addon not found. Run `npm run build:debug` before the test suite.',
    );
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('@state-variant', () => {
  it('parses basic variant', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-variant Theme {
        values: light, dark;
      }
    `);
    expect(ast.rules).toHaveLength(1);
    const sv = expectVariant(ast.rules[0]);
    expect(sv.name).toBe('Theme');
    expect(sv.values).toEqual(['light', 'dark']);
  });

  it('parses variant with quoted values', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-variant Size {
        values: small, "extra large", medium;
      }
    `);
    const sv = expectVariant(ast.rules[0]);
    expect(sv.values).toEqual(['small', 'extra large', 'medium']);
  });
});

describe('@state-def', () => {
  it('parses state without params', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card {
        background: red;
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    expect(sd.name).toBe('Card');
    expect(sd.params).toHaveLength(0);
    const decl = expectDeclaration(sd.body[0]);
    expect(decl.property).toBe('background');
    expect(decl.value).toBe('red');
  });

  it('parses state with boolean param (implicit)', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--is-active) {
        color: blue;
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    expect(sd.params).toHaveLength(1);
    expect(sd.params[0].name).toBe('--is-active');
    expect(sd.params[0].paramType).toBe('boolean');
    expect(sd.params[0].defaultValue).toBeUndefined();
  });

  it('parses state with boolean param and explicit default', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--is-active boolean: true) {
        color: blue;
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    expect(sd.params[0].defaultValue).toBe('true');
  });

  it('parses state with variant param', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--theme Theme: "light") {
        color: blue;
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    expect(sd.params[0].paramType).toBe('variant');
    expect(sd.params[0].variantName).toBe('Theme');
    expect(sd.params[0].defaultValue).toBe('light');
  });

  it('parses state with multiple params', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--is-active, --theme Theme: "dark") {
        background: black;
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    expect(sd.params).toHaveLength(2);
    expect(sd.params[0].name).toBe('--is-active');
    expect(sd.params[1].variantName).toBe('Theme');
  });
});

describe('@if / @elseif / @else', () => {
  it('parses @if with var condition (implicit == true)', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--is-active) {
        @if (--is-active) {
          color: red;
        }
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    const chain = expectIfChain(sd.body[0]);
    const cond = chain.ifClause.condition as Record<string, unknown>;
    expect(cond.kind).toBe('var');
    expect(cond.var).toBe('--is-active');
  });

  it('parses @if with == comparison', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--theme Theme) {
        @if (--theme == "dark") {
          background: black;
        }
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    const chain = expectIfChain(sd.body[0]);
    const cond = chain.ifClause.condition as Record<string, unknown>;
    expect(cond.kind).toBe('comparison');
    expect(cond.left).toBe('--theme');
    expect(cond.op).toBe('==');
    const right = cond.right as Record<string, string>;
    expect(right.kind).toBe('string');
    expect(right.value).toBe('dark');
  });

  it('parses @if with && compound condition', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--is-active, --theme Theme) {
        @if (--is-active && --theme == "dark") {
          color: white;
        }
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    const chain = expectIfChain(sd.body[0]);
    const cond = chain.ifClause.condition as Record<string, unknown>;
    expect(cond.kind).toBe('and');
    const left = cond.left as Record<string, unknown>;
    expect(left.kind).toBe('var');
    const right = cond.right as Record<string, unknown>;
    expect(right.kind).toBe('comparison');
  });

  it('parses @if / @elseif / @else chain', async () => {
    const parse = await loadParser();
    const ast = parse(`
      @state-def Card(--theme Theme) {
        @if (--theme == "light") {
          background: white;
        }
        @elseif (--theme == "dark") {
          background: black;
        }
        @else {
          background: gray;
        }
      }
    `);
    const sd = expectStateDef(ast.rules[0]);
    const chain = expectIfChain(sd.body[0]);
    expect(chain.elseIfClauses).toHaveLength(1);
    expect(chain.elseBody).toBeDefined();
    const elseDecl = expectDeclaration(chain.elseBody![0]);
    expect(elseDecl.value).toBe('gray');
  });
});

describe('error handling', () => {
  it('throws on missing @state-variant name', async () => {
    const parse = await loadParser();
    expect(() =>
      parse(`
        @state-variant {
          values: a, b;
        }
      `),
    ).toThrow();
  });
});

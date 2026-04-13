<div align="center">
  <h1>@ecss/parser</h1>
  <br>
  <img alt="@ecss/parser" src="./assets/logo.svg" height="240">
  <br>
  <br>
  <p style="text-decoration: none">
    <a href="https://www.npmjs.com/package/@ecss/parser">
       <img src="https://img.shields.io/npm/v/@ecss/parser.svg?color=646fe1&labelColor=9B7AEF" alt="npm package" />
    </a>
    <a href="https://github.com/webeach/react-x/actions">
      <img src="https://img.shields.io/github/actions/workflow/status/webeach/ecss-parser/ci.yml?color=646fe1&labelColor=9B7AEF" alt="build" />
    </a>
    <a href="https://www.npmjs.com/package/@ecss/parser">
      <img src="https://img.shields.io/npm/dm/@ecss/parser.svg?color=646fe1&labelColor=9B7AEF" alt="npm downloads" />
    </a>
  </p>
  <p><a href="./README.md">🇺🇸 English version</a> | <a href="./README.ru.md">🇷🇺 Русская версия</a></p>
  <p>High-performance ECSS parser written in Rust (napi-rs). Accepts source text and returns an AST.</p>
  <br>
  <p>
    <a href="https://ecss.webea.ch" style="font-size: 1.5em">📖 Documentation</a> | <a href="https://ecss.webea.ch/reference/spec.html" style="font-size: 1.5em">📋 Specification</a>
  </p>
</div>

---

## 💎 Features

- ⚡ **Written in Rust** — native N-API addon, minimal overhead
- 🌐 **WASM support** — works in Node.js without a native build (`@ecss/parser/wasm`) and in the browser (`@ecss/parser/wasm/browser`)
- 📦 **Dual CJS/ESM** — `require` and `import` out of the box
- 🖥️ **Cross-platform** — macOS, Linux, Windows, plus a WASM target
- 📝 **TypeScript** — types included, auto-generated from Rust code

---

## 📦 Installation

```bash
npm install @ecss/parser
```

or

```bash
pnpm add @ecss/parser
```

or

```bash
yarn add @ecss/parser
```

---

## 🚀 Quick start

```ts
import { parseEcss } from '@ecss/parser';

const ast = parseEcss(`
  @state-variant Theme {
    values: light, dark;
  }

  @state-def Button(--theme Theme: "light", --disabled boolean: false) {
    border-radius: 6px;

    @if (--disabled) {
      opacity: 0.4;
      cursor: not-allowed;
    }

    @if (--theme == "light") {
      background: #fff;
      color: #111;
    }
    @else {
      background: #1e1e1e;
      color: #f0f0f0;
    }
  }
`);

console.log(ast.rules);
```

---

## 🛠 API

### `parseEcss(source: string): EcssStylesheet`

The only exported function. Accepts an ECSS source string and returns the AST.

On parse failure throws a JavaScript `Error` with the source location: `[line:column] description`.

```ts
import { parseEcss } from '@ecss/parser';

try {
  const ast = parseEcss(source);
  // ast: EcssStylesheet
} catch (err) {
  // e.g. "[3:5] Unknown at-rule: @unknown"
  console.error(err.message);
}
```

---

## 📐 AST types

### `EcssStylesheet`

The root node of the tree.

```ts
interface EcssStylesheet {
  rules: EcssRule[];
}
```

### `EcssRule`

A top-level rule. The `kind` discriminant determines which field is populated.

```ts
interface EcssRule {
  kind: 'state-variant' | 'state-def' | 'qualified-rule' | 'at-rule';
  stateVariant?: StateVariant;
  stateDef?: StateDef;
  qualifiedRule?: CssQualifiedRule;
  atRule?: CssRawAtRule;
}
```

### `StateVariant`

An `@state-variant` node.

```ts
interface StateVariant {
  name: string; // enumeration name, e.g. "Theme"
  values: string[]; // ["light", "dark"]
  span: Span;
}
```

### `StateDef`

An `@state-def` node.

```ts
interface StateDef {
  name: string;
  params: StateParam[];
  body: StateDefItem[];
  span: Span;
}

interface StateParam {
  name: string; // "--theme"
  paramType: 'boolean' | string; // "boolean" or a @state-variant name
  variantName?: string; // @state-variant name for variant params
  defaultValue?: string; // "light", "true", "false", etc.
}
```

### `StateDefItem`

An item inside a `@state-def` or `@if` block body.

```ts
interface StateDefItem {
  kind: 'declaration' | 'qualified-rule' | 'if-chain' | 'at-rule';
  declaration?: CssDeclaration;
  qualifiedRule?: CssQualifiedRule;
  ifChain?: IfChain;
  atRule?: CssRawAtRule;
}
```

### `IfChain`

An `@if` / `@elseif` / `@else` node.

```ts
interface IfChain {
  ifClause: IfClause;
  elseIfClauses: IfClause[];
  elseBody?: StateDefItem[];
  span: Span;
}

interface IfClause {
  condition: unknown; // JSON-serialized ConditionExpr
  body: StateDefItem[];
  span: Span;
}
```

The `condition` field contains a `ConditionExpr` serialized to JSON. Shape:

```ts
// { kind: "var",        var: "--name" }
// { kind: "comparison", left: "--name", op: "==" | "!=", right: { kind, value } }
// { kind: "and",        left: ConditionExpr, right: ConditionExpr }
// { kind: "or",         left: ConditionExpr, right: ConditionExpr }
```

### `CssDeclaration`

A CSS declaration (property: value).

```ts
interface CssDeclaration {
  property: string;
  value: string;
  important: boolean;
  span: Span;
}
```

### `CssQualifiedRule`

A CSS rule with a selector (including CSS Nesting inside `@state-def`).

```ts
interface CssQualifiedRule {
  selector: string; // "&:hover", ".class > div", etc.
  body: StateDefItem[];
  span: Span;
}
```

### `CssRawAtRule`

An arbitrary CSS at-rule that is not an ECSS construct.

```ts
interface CssRawAtRule {
  name: string;
  prelude: string;
  block?: string;
  span: Span;
}
```

### `Span`

Source position of a node.

```ts
interface Span {
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}
```

---

## 🌐 WASM

If the native build is unavailable (e.g. containers without N-API support, or the browser), use the WASM variants.

**Node.js / WASI:**

```ts
import { parseEcss } from '@ecss/parser/wasm';
```

**Browser:**

```ts
import { parseEcss } from '@ecss/parser/wasm/browser';
```

> The native addon is preferred automatically; the WASM binding acts as a fallback. To force WASM, set the environment variable `NAPI_RS_FORCE_WASI=1`.

---

## 🖥️ Supported platforms

| Platform      | Architecture | Target                     |
| ------------- | ------------ | -------------------------- |
| macOS         | x64          | `x86_64-apple-darwin`      |
| macOS         | ARM64        | `aarch64-apple-darwin`     |
| Linux (glibc) | x64          | `x86_64-unknown-linux-gnu` |
| Windows       | x64          | `x86_64-pc-windows-msvc`   |
| WASM          | —            | `wasm32-wasip1-threads`    |

---

## 🔧 Development

**Build native addon:**

```bash
pnpm build          # release
pnpm build:debug    # debug
```

**Build WASM:**

```bash
pnpm build:wasm         # release
pnpm build:wasm:debug   # debug
```

**Tests:**

```bash
pnpm test
```

**Type check:**

```bash
pnpm typecheck
```

**Lint and format (JS/TS):**

```bash
pnpm lint         # oxlint
pnpm lint:fix     # oxlint --fix
pnpm fmt          # oxfmt
pnpm fmt:check    # oxfmt --check
```

**Lint and format (Rust):**

```bash
pnpm lint:rs      # cargo clippy -D warnings
pnpm lint:rs:fix  # cargo clippy --fix
pnpm fmt:rs       # cargo fmt
pnpm fmt:rs:check # cargo fmt --check
```

---

## 👨‍💻 Author

Developed and maintained by [Ruslan Martynov](https://github.com/ruslan-mart).

Found a bug or have a suggestion? Open an issue or submit a pull request.

---

## 📄 License

Distributed under the [MIT License](./LICENSE).

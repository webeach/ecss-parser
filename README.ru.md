<div align="center">
  <h1>@ecss/parser</h1>
  <br>
  <img alt="react-x" src="./assets/logo.svg" height="240">
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
  <p>Высокопроизводительный парсер языка ECSS на Rust (napi-rs). Принимает исходный текст и возвращает AST.</p>
  <br>
  <p>
    <a href="https://ecss.webea.ch/ru" style="font-size: 1.5em">📖 Документация</a> | <a href="https://ecss.webea.ch/ru/reference/spec.html" style="font-size: 1.5em">📋 Спецификация</a>
  </p>
</div>

---

## 💎 Особенности

- ⚡ **Написан на Rust** — нативный N-API аддон, минимальные накладные расходы
- 🌐 **WASM-поддержка** — работает в Node.js без нативной сборки (`@ecss/parser/wasm`) и в браузере (`@ecss/parser/wasm/browser`)
- 📦 **Dual CJS/ESM** — `require` и `import` из коробки
- 🖥️ **Кроссплатформенность** — macOS, Linux, Windows, плюс WASM-таргет
- 📝 **TypeScript** — типы в комплекте, генерируются из Rust-кода автоматически

---

## 📦 Установка

```bash
npm install @ecss/parser
```

или

```bash
pnpm add @ecss/parser
```

или

```bash
yarn add @ecss/parser
```

---

## 🚀 Быстрый старт

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

Единственная экспортируемая функция. Принимает строку с ECSS-кодом и возвращает AST.

При ошибке парсинга выбрасывает JavaScript `Error` с указанием позиции: `[line:column] описание`.

```ts
import { parseEcss } from '@ecss/parser';

try {
  const ast = parseEcss(source);
  // ast: EcssStylesheet
} catch (err) {
  // Например: "[3:5] Unknown at-rule: @unknown"
  console.error(err.message);
}
```

---

## 📐 AST-типы

### `EcssStylesheet`

Корневой узел дерева.

```ts
interface EcssStylesheet {
  rules: EcssRule[];
}
```

### `EcssRule`

Правило верхнего уровня. Дискриминант `kind` определяет, какое поле заполнено.

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

Узел `@state-variant`.

```ts
interface StateVariant {
  name: string; // имя перечисления, напр. "Theme"
  values: string[]; // ["light", "dark"]
  span: Span;
}
```

### `StateDef`

Узел `@state-def`.

```ts
interface StateDef {
  name: string;
  params: StateParam[];
  body: StateDefItem[];
  span: Span;
}

interface StateParam {
  name: string; // "--theme"
  paramType: 'boolean' | string; // "boolean" или имя @state-variant
  variantName?: string; // имя @state-variant для variant-параметров
  defaultValue?: string; // "light", "true", "false" и т.д.
}
```

### `StateDefItem`

Элемент тела `@state-def` или `@if`-блока.

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

Узел `@if` / `@elseif` / `@else`.

```ts
interface IfChain {
  ifClause: IfClause;
  elseIfClauses: IfClause[];
  elseBody?: StateDefItem[];
  span: Span;
}

interface IfClause {
  condition: unknown; // JSON-сериализованный ConditionExpr
  body: StateDefItem[];
  span: Span;
}
```

Поле `condition` содержит `ConditionExpr`, сериализованный в JSON. Структура:

```ts
// { kind: "var",        var: "--name" }
// { kind: "comparison", left: "--name", op: "==" | "!=", right: { kind, value } }
// { kind: "and",        left: ConditionExpr, right: ConditionExpr }
// { kind: "or",         left: ConditionExpr, right: ConditionExpr }
```

### `CssDeclaration`

CSS-объявление (свойство: значение).

```ts
interface CssDeclaration {
  property: string;
  value: string;
  important: boolean;
  span: Span;
}
```

### `CssQualifiedRule`

CSS-правило с селектором (включая CSS Nesting внутри `@state-def`).

```ts
interface CssQualifiedRule {
  selector: string; // "&:hover", ".class > div" и т.д.
  body: StateDefItem[];
  span: Span;
}
```

### `CssRawAtRule`

Произвольный CSS at-rule, не являющийся ECSS-конструкцией.

```ts
interface CssRawAtRule {
  name: string;
  prelude: string;
  block?: string;
  span: Span;
}
```

### `Span`

Позиция узла в исходном тексте.

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

Если нативная сборка недоступна (например, в контейнерах без поддержки N-API или в браузере), используйте WASM-варианты.

**Node.js / WASI:**

```ts
import { parseEcss } from '@ecss/parser/wasm';
```

**Браузер:**

```ts
import { parseEcss } from '@ecss/parser/wasm/browser';
```

> Нативный аддон автоматически используется в приоритете, WASM-биндинг — как запасной вариант. Принудительно переключить на WASM можно через переменную окружения `NAPI_RS_FORCE_WASI=1`.

---

## 🖥️ Поддерживаемые платформы

| Платформа     | Архитектура | Таргет                     |
| ------------- | ----------- | -------------------------- |
| macOS         | x64         | `x86_64-apple-darwin`      |
| macOS         | ARM64       | `aarch64-apple-darwin`     |
| Linux (glibc) | x64         | `x86_64-unknown-linux-gnu` |
| Windows       | x64         | `x86_64-pc-windows-msvc`   |
| WASM          | —           | `wasm32-wasip1-threads`    |

---

## 🔧 Разработка

**Сборка нативного аддона:**

```bash
pnpm build          # release
pnpm build:debug    # debug
```

**Сборка WASM:**

```bash
pnpm build:wasm         # release
pnpm build:wasm:debug   # debug
```

**Тесты:**

```bash
pnpm test
```

**Проверка типов:**

```bash
pnpm typecheck
```

**Линтинг и форматирование (JS/TS):**

```bash
pnpm lint         # oxlint
pnpm lint:fix     # oxlint --fix
pnpm fmt          # oxfmt
pnpm fmt:check    # oxfmt --check
```

**Линтинг и форматирование (Rust):**

```bash
pnpm lint:rs      # cargo clippy -D warnings
pnpm lint:rs:fix  # cargo clippy --fix
pnpm fmt:rs       # cargo fmt
pnpm fmt:rs:check # cargo fmt --check
```

---

## 👨‍💻 Автор

Разработка и поддержка: [Руслан Мартынов](https://github.com/ruslan-mart)

Если нашёл баг или есть предложение — открывай issue или отправляй pull request.

---

## 📄 Лицензия

Распространяется под [лицензией MIT](./LICENSE).

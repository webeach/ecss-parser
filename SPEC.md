# ECSS Language Specification

**Version:** 0.1.0-draft  
**Status:** Draft  
**Date:** 2026-04-12

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Conformance](#2-conformance)
3. [Lexical Structure](#3-lexical-structure)
4. [Grammar](#4-grammar)
   - 4.1 [Stylesheet](#41-stylesheet)
   - 4.2 [`@state-variant`](#42-state-variant)
   - 4.3 [`@state-def`](#43-state-def)
   - 4.4 [Rule Body](#44-rule-body)
   - 4.5 [`@if` Chain](#45-if-chain)
   - 4.6 [Condition Expressions](#46-condition-expressions)
5. [Static Semantics](#5-static-semantics)
   - 5.1 [Name Uniqueness](#51-name-uniqueness)
   - 5.2 [Scope and Reference Resolution](#52-scope-and-reference-resolution)
   - 5.3 [Type Compatibility](#53-type-compatibility)
6. [Runtime Semantics](#6-runtime-semantics)
   - 6.1 [State Evaluation](#61-state-evaluation)
   - 6.2 [Conditional Branch Selection](#62-conditional-branch-selection)
7. [Relation to CSS](#7-relation-to-css)
8. [Examples](#8-examples)

---

## 1. Introduction

ECSS (Extended CSS) is a strict superset of CSS that introduces three new at-rules for declarative component state management:

| Construct                   | Purpose                                       |
| --------------------------- | --------------------------------------------- |
| `@state-variant`            | Declares a named enumeration of string values |
| `@state-def`                | Declares a parameterised set of CSS rules     |
| `@if` / `@elseif` / `@else` | Selects CSS rules based on parameter values   |

All valid CSS is valid ECSS. ECSS source is intended to be transpiled to standard CSS by a separate toolchain; the parser defined here produces an Abstract Syntax Tree (AST) only.

---

## 2. Conformance

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

A conforming ECSS parser:

- MUST accept any input that is valid per the grammar in §4.
- MUST reject any input that violates a **MUST** or **MUST NOT** constraint in §5.
- MUST produce an AST that faithfully represents all constructs in the source.
- MAY emit diagnostics for violations of **SHOULD** constraints.

---

## 3. Lexical Structure

ECSS inherits the complete lexical grammar of CSS (tokenisation, whitespace, comments, string literals, identifiers). The following additional conventions apply.

### 3.1 Identifiers

A _plain identifier_ (`ident`) matches the CSS `<ident-token>` production. ECSS additionally defines:

- **PascalCase name** — an `ident` whose first character is an uppercase ASCII letter (`A–Z`). Used for `@state-variant` and `@state-def` names.
- **Custom property name** — a token of the form `--<ident>`, matching the CSS `<custom-property-name>` production. Used for parameter names.

### 3.2 String Values

String literals are delimited by either single quotes (`'`) or double quotes (`"`). ECSS does not define additional escape sequences beyond those of CSS.

### 3.3 Keywords

The following identifiers are reserved as ECSS at-rule names and MUST NOT be used as PascalCase names or parameter names:

`state-variant`, `state-def`, `if`, `elseif`, `else`

---

## 4. Grammar

The grammar is presented in an EBNF-like notation:

| Notation  | Meaning                       |
| --------- | ----------------------------- |
| `A B`     | A followed by B               |
| `A \| B`  | A or B (ordered choice)       |
| `A?`      | Zero or one occurrence of A   |
| `A*`      | Zero or more occurrences of A |
| `A+`      | One or more occurrences of A  |
| `( A )`   | Grouping                      |
| `"x"`     | Terminal string               |
| `<token>` | CSS token name                |

Whitespace and CSS comments are allowed between any two tokens unless explicitly prohibited. They are omitted from the grammar for clarity.

### 4.1 Stylesheet

```ebnf
stylesheet = stylesheet-item*

stylesheet-item
  = state-variant-rule
  | state-def-rule
  | css-qualified-rule
  | css-at-rule
```

`css-qualified-rule` and `css-at-rule` denote any syntactically valid CSS rule that is not an ECSS at-rule. They are passed through to the AST without semantic validation.

### 4.2 `@state-variant`

```ebnf
state-variant-rule = "@state-variant" pascal-name "{" variant-body "}"

variant-body = "values" ":" value-list ";"

value-list = value ( "," value )* ","?

value = <ident-token> | <string-token>
```

- `pascal-name` is an `<ident-token>` whose first character is an uppercase ASCII letter.
- Trailing comma in `value-list` is **OPTIONAL**.

### 4.3 `@state-def`

```ebnf
state-def-rule = "@state-def" state-def-head "{" rule-body "}"

state-def-head
  = pascal-name
  | pascal-name "(" param-list? ")"

param-list = param ( "," param )* ","?

param = custom-property-name param-type? param-default?

param-type = "boolean" | pascal-name

param-default = ":" param-default-value

param-default-value = <ident-token> | <string-token>
```

- When `param-type` is omitted, the parameter is implicitly typed as `boolean` with a default value of `false`.
- When `param-type` is `boolean`, the parameter default MUST be either `true` or `false` if specified.
- When `param-type` is a `pascal-name`, the parameter type is a reference to a `@state-variant` with that name.
- Parentheses MAY be omitted when the parameter list is empty.

#### Parameter Type Summary

| Syntax               | Type        | Default |
| -------------------- | ----------- | ------- |
| `--p`                | `boolean`   | `false` |
| `--p boolean`        | `boolean`   | `false` |
| `--p boolean: true`  | `boolean`   | `true`  |
| `--p boolean: false` | `boolean`   | `false` |
| `--p Variant`        | variant ref | none    |
| `--p Variant: "val"` | variant ref | `"val"` |

### 4.4 Rule Body

```ebnf
rule-body = rule-body-item*

rule-body-item
  = css-declaration
  | css-qualified-rule
  | if-chain
  | css-at-rule
```

A `css-declaration` is a standard CSS property–value pair optionally followed by `!important`.

### 4.5 `@if` Chain

```ebnf
if-chain = if-clause elseif-clause* else-clause?

if-clause     = "@if"     "(" condition ")" "{" rule-body "}"
elseif-clause = "@elseif" "(" condition ")" "{" rule-body "}"
else-clause   = "@else"                    "{" rule-body "}"
```

The `@elseif` and `@else` tokens MUST immediately follow the closing `}` of the preceding `@if` or `@elseif` clause. Only whitespace and CSS comments are permitted between `}` and the next keyword; any other token breaks the chain.

An `@else` clause MUST appear at most once and MUST be the final clause in the chain.

`@if` chains MAY be nested to any depth within a `rule-body`.

### 4.6 Condition Expressions

```ebnf
condition = or-expr

or-expr  = and-expr ( "||" and-expr )*
and-expr = cmp-expr ( "&&" cmp-expr )*

cmp-expr
  = primary "==" rhs
  | primary "!=" rhs
  | primary

primary
  = "(" condition ")"
  | custom-property-name

rhs = <ident-token> | <string-token>
```

#### Operator Precedence (highest to lowest)

| Level | Operator       | Associativity |
| ----- | -------------- | ------------- |
| 1     | `( )` grouping | —             |
| 2     | `==` `!=`      | left          |
| 3     | `&&`           | left          |
| 4     | `\|\|`         | left          |

#### Shorthand

A bare `custom-property-name` in a condition is equivalent to:

```
--param == true
```

---

## 5. Static Semantics

Static semantics are constraints that can be verified without executing the stylesheet. A conforming parser MAY enforce them during parsing or in a subsequent validation pass; either way, violations MUST be reported as errors.

### 5.1 Name Uniqueness

**Rule SEM-1.** Within a single ECSS source file, all `@state-variant` names MUST be distinct.

**Rule SEM-2.** Within a single ECSS source file, all `@state-def` names MUST be distinct.

**Rule SEM-3.** The set of `@state-variant` names and the set of `@state-def` names MUST be disjoint. A name MUST NOT be used for both a `@state-variant` and a `@state-def` in the same file.

### 5.2 Scope and Reference Resolution

**Rule SEM-4.** `@state-variant` declarations MUST appear at the top level of the stylesheet. They MUST NOT appear inside `@state-def`, CSS qualified rules, or any other block.

**Rule SEM-5.** `@state-def` declarations MUST appear at the top level of the stylesheet. They MUST NOT be nested inside other at-rules or qualified rules.

**Rule SEM-6.** `@if` / `@elseif` / `@else` constructs MUST appear only inside the body of a `@state-def`. They MUST NOT appear at the stylesheet top level.

**Rule SEM-7.** Every `custom-property-name` referenced in a condition expression MUST correspond to a parameter declared in the immediately enclosing `@state-def`. References to parameters of outer `@state-def` blocks (from nested `@if` chains) are not permitted.

### 5.3 Type Compatibility

**Rule SEM-8.** When a `param-type` is a `pascal-name`, that name MUST resolve to a `@state-variant` declared in the same file.

**Rule SEM-9.** In a comparison expression `--param == rhs` or `--param != rhs`:

- If `--param` is typed `boolean`, then `rhs` MUST be the identifier `true` or `false`.
- If `--param` is a variant reference, then `rhs` MUST be a string or identifier that is a declared value of the referenced `@state-variant`.

**Rule SEM-10.** A bare `--param` in a condition (shorthand for `--param == true`) is only valid when `--param` is typed `boolean`.

**Rule SEM-11.** When a `param-default-value` is specified for a variant-typed parameter, its value MUST be a declared value of the referenced `@state-variant`.

---

## 6. Runtime Semantics

Runtime semantics describe how an ECSS transpiler or runtime resolves the constructs to CSS. This section is informative for parser implementors and normative for transpiler implementors.

### 6.1 State Evaluation

A `@state-def` block is evaluated by binding each parameter to a concrete value:

- A `boolean` parameter with no supplied value is bound to its declared default, or `false` if no default is declared.
- A variant parameter with no supplied value is bound to its declared default. If no default is declared, the parameter MUST be explicitly supplied; omitting it is a runtime error.

### 6.2 Conditional Branch Selection

Given a bound parameter environment, an `@if` chain is evaluated as follows:

1. Evaluate the condition of the `@if` clause. If it is `true`, apply the body of that clause and skip all remaining clauses.
2. Otherwise, evaluate each `@elseif` clause in order. For the first clause whose condition is `true`, apply its body and skip remaining clauses.
3. If no clause has been selected and an `@else` clause is present, apply its body.

Condition evaluation is short-circuit: in `A && B`, `B` is not evaluated if `A` is `false`; in `A || B`, `B` is not evaluated if `A` is `true`.

---

## 7. Relation to CSS

### 7.1 Superset

Every syntactically and semantically valid CSS stylesheet is a valid ECSS stylesheet. ECSS does not redefine or restrict any CSS production.

### 7.2 New At-Rules

ECSS introduces the at-keywords `@state-variant`, `@state-def`, `@if`, `@elseif`, and `@else`. These keywords are not defined by any CSS specification. A CSS parser that does not understand them will ignore the corresponding blocks per the CSS error-handling rules (unknown at-rules with blocks).

### 7.3 CSS Nesting

Inside `@state-def` and `@if`/`@elseif`/`@else` bodies, CSS Nesting syntax (as defined in the [CSS Nesting Module Level 1](https://www.w3.org/TR/css-nesting-1/)) is valid and MUST be preserved in the AST.

---

## 8. Examples

### 8.1 Basic Enumeration and Usage

```ecss
@state-variant Theme {
  values: light, dark, "high contrast";
}

@state-variant Size {
  values: sm, md, lg;
}

@state-def Button(--theme Theme: "light", --size Size: "md", --disabled boolean: false) {
  border-radius: 6px;
  font-weight: 500;
  cursor: pointer;

  @if (--disabled) {
    opacity: 0.4;
    cursor: not-allowed;
    pointer-events: none;
  }

  @if (--size == "sm") {
    padding: 4px 8px;
    font-size: 12px;
  }
  @elseif (--size == "md") {
    padding: 8px 16px;
    font-size: 14px;
  }
  @else {
    padding: 12px 24px;
    font-size: 16px;
  }

  @if (--theme == "light") {
    background: #ffffff;
    color: #111111;
    border: 1px solid #cccccc;
  }
  @elseif (--theme == "dark") {
    background: #1e1e1e;
    color: #f0f0f0;
    border: 1px solid #444444;
  }
  @else {
    background: #000000;
    color: #ffffff;
    border: 2px solid #ffffff;
    font-size: 18px;
  }
}
```

### 8.2 Compound Conditions and Nested `@if`

```ecss
@state-variant Theme {
  values: light, dark;
}

@state-def Panel(--expanded boolean, --theme Theme: "light", --pinned boolean: false) {
  padding: 16px;
  border: 1px solid #e0e0e0;
  transition: all 0.2s ease;

  @if (--expanded && --theme == "dark") {
    background: #121212;
    border-color: #333333;

    & > .header {
      font-weight: bold;
      color: #ffffff;
    }

    @if (--pinned) {
      border-color: #0077ff;
    }
  }

  @if (--expanded || --pinned) {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
}
```

### 8.3 No-Parameter `@state-def`

```ecss
@state-def Card {
  padding: 24px;
  border: 1px solid #e0e0e0;
  border-radius: 12px;
  background: #ffffff;
}
```

### 8.4 Mixed CSS and ECSS

```ecss
/* Standard CSS — valid ECSS */
*, *::before, *::after {
  box-sizing: border-box;
}

@state-variant Status {
  values: idle, loading, error, success;
}

@state-def Badge(--status Status: "idle") {
  display: inline-flex;
  align-items: center;
  border-radius: 9999px;
  padding: 2px 10px;
  font-size: 12px;

  @if (--status == "idle")    { background: #f0f0f0; color: #555555; }
  @elseif (--status == "loading") { background: #e0f0ff; color: #0055cc; }
  @elseif (--status == "error")   { background: #ffe0e0; color: #cc0000; }
  @else                           { background: #e0ffe0; color: #006600; }
}

/* More standard CSS below */
@media (prefers-color-scheme: dark) {
  body { background: #0a0a0a; }
}
```

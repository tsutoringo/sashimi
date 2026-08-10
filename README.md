# Sashimi 🍣

Sashimi is an experimental statically typed language for the JavaScript ecosystem. It keeps JavaScript as the runtime target and TypeScript declarations as the interoperability boundary, while adding Rust-like traits that can be implemented without ordinary string-named prototype methods.

> Sashimi is a compiler prototype. The syntax and semantics are intentionally unstable.

## What works today

- A small TypeScript-like source syntax (`.sashimi`).
- `fn`, `pub fn`, `let`, arrays, strings, calls, member access, empty classes, `new`, `trait`, and `impl`.
- ES module imports for JavaScript/npm libraries, including named, aliased, namespace, and default import syntax.
- Basic `.d.ts` consumption for exported functions, classes, interfaces, and constants.
- Relative declaration resolution and npm `node_modules` resolution through `types`, `typings`, and `exports` type targets.
- Compile-time trait method resolution.
- A compiler-trusted core/prelude.
- `Len` for JavaScript `Array`, `String`, `Map`, and `Set`.
- `IntoIterator` for `Array<T>` and `Map<K, V>` through `iter()`.
- A Sashimi-owned lazy `Iterator<T>` with adapters and consumers such as `map`, `filter`, `take`, `collect`, `fold`, and `sum`.
- Intrinsic lowering such as `values.len()` → `values.length`.
- Symbol-backed methods for user-defined trait implementations.
- A strict foreign-target rule: user code cannot implement traits for foreign/built-in types.
- JavaScript, `.d.ts`, and source-map output.
- Zed highlighting and an LSP with diagnostics, completion, hover, and document symbols.
- No external runtime dependency; iterator support is emitted into generated JavaScript only when used.

## Example

```ts
pub fn arrayLength(values: Array<number>): number {
    return values.len();
}

fn main() {
    let values = [1, 2, 3];
    console.log(values.len());
}
```

Build it:

```sh
cargo run -- build examples/hello.sashimi
```

The important part of the generated JavaScript is:

```js
export function arrayLength(values) {
    return values.length;
}
```

The generated declaration contains:

```ts
export declare function arrayLength(values: Array<number>): number;
```

## JavaScript and TypeScript libraries

Sashimi keeps normal ESM imports in generated JavaScript and uses nearby TypeScript declaration files as compile-time type information.

```ts
import * as demo from "demo-lib";

pub fn total(): number {
    return demo.numbers().iter().sum();
}
```

If `demo-lib` declares:

```ts
export declare function numbers(): Array<number>;
```

Sashimi knows that `demo.numbers()` returns `Array<number>`, so the core `IntoIterator` and `Iterator` traits can participate in the rest of the expression. The emitted JavaScript still imports `demo-lib` normally.

For npm packages the compiler walks parent directories for `node_modules`, then reads declaration entry points from `package.json` `types`, `typings`, or `exports` type conditions. Relative imports such as `./library.js` use a sibling `library.d.ts` or `index.d.ts` when available. Packages without declarations can still be emitted as runtime imports, but their values remain `unknown` to the bootstrap type system.

## Iterators

`Array<T>` and `Map<K, V>` get `iter()` from the core `IntoIterator` trait. The method returns Sashimi's own lazy iterator rather than exposing a native JavaScript iterator directly.

```ts
fn identity(value: number): number {
    return value;
}

fn keep(value: number): boolean {
    return true;
}

fn main() {
    let values = [1, 2, 3, 4];
    let result = values
        .iter()
        .map(identity)
        .filter(keep)
        .skip(1)
        .take(2)
        .enumerate()
        .collect();

    console.log(result);
}
```

Iterator adapters currently include `map`, `filter`, `take`, `skip`, `enumerate`, `chain`, `zip`, `inspect`, `flat_map`, and `flatten`. Consumers include `next`, `collect`, `count`, `nth`, `last`, `find`, `position`, `any`, `all`, `fold`, `reduce`, `sum`, `product`, `min`, `max`, and `for_each`.

`Array<T>.iter()` yields `T`. `Map<K, V>.iter()` follows JavaScript's normal `Map` iteration and yields `[K, V]` entries. Iterator operations advance and consume the iterator; calling `iter()` again creates a fresh iterator.

A public Sashimi `Iterator<T>` is projected into generated TypeScript declarations as `SashimiIterator<T>` so it is not confused with TypeScript's built-in `Iterator<T>`.

## User-defined traits

```ts
trait Display {
    fn display(&self): string;
}

class User {}

impl Display for User {
    fn display(&self): string {
        return "User";
    }
}

fn main() {
    let user = new User();
    console.log(user.display());
}
```

Sashimi resolves `display` at compile time and lowers it to a symbol-backed method. The runtime never adds `User.prototype.display`.

## Core and prelude

Normal packages are deliberately not allowed to attach trait implementations to foreign JavaScript/TypeScript types. The compiler-trusted core library is the exception. Its common traits are made visible through the prelude.

For example, core owns the built-in `Len` and `IntoIterator` behavior, so these are available without imports:

```ts
[1, 2, 3].len()
[1, 2, 3].iter()
```

but a package cannot define its own `impl MyTrait for Array<T>`.

See [`SPEC.md`](./SPEC.md) for the language direction, [`core/README.md`](./core/README.md) for the bootstrap core model, and [`editors/zed/README.md`](./editors/zed/README.md) for Zed setup.

## CLI

```text
sashimi build <file.sashimi> [--out-dir DIR] [--package NAME]
sashimi check <file.sashimi> [--package NAME]
sashimi lsp
```

`build` writes JavaScript, a TypeScript declaration file, and a source map to `dist/` by default.

## Compiler pipeline

```text
.sashimi + package .d.ts
          ↓
lexer + parser + module resolution
          ↓
AST + imported type bindings
          ↓
semantic analysis / type inference
          ↓
trait + impl resolution
          ↓
JavaScript emitter ──→ .js + .map
Declaration emitter ─→ .d.ts
```

The parser and IR remain Sashimi-owned. The current `.d.ts` reader deliberately supports a useful bootstrap subset; a future Oxc-backed boundary can expand TypeScript syntax coverage without making the TypeScript AST Sashimi's canonical representation.

## Development

A reproducible Rust development shell is available through Nix:

```sh
nix develop
```

It provides stable Rust with Cargo, rustfmt, Clippy, rust-analyzer, and rust-src.

Inside the shell:

```sh
cargo fmt --check
cargo test
cargo run -- check examples/hello.sashimi
cargo run -- check examples/iterator.sashimi
cargo run -- build examples/trait.sashimi
```

## Near-term roadmap

1. Replace the bootstrap `.d.ts` reader with broader TypeScript declaration parsing while keeping the Sashimi-owned type model.
2. Add typed members, constructors, overloads, unions, generics, and re-exports from external declarations.
3. Integrate Oxc for JS/TS parsing, package resolution, and code generation where it improves compatibility.
4. Add generic bounds and generic trait implementations beyond the bootstrap subset.
5. Add diagnostics with real source spans throughout semantic analysis and imported declarations.
6. Emit useful source-map mappings rather than the current valid-but-empty mapping table.
7. Move core definitions into a distributed trusted package.
8. Replace the bootstrap Rust Tree-sitter grammar with a dedicated Sashimi grammar as syntax stabilizes.

## License

MIT

# Sashimi 🍣

Sashimi is an experimental statically typed language for the JavaScript ecosystem. It keeps JavaScript as the runtime target and TypeScript declarations as the interoperability boundary, while adding Rust-like traits that can be implemented without ordinary string-named prototype methods.

> Sashimi is a compiler prototype. The syntax and semantics are intentionally unstable.

## What works today

- A small TypeScript-like source syntax (`.sashimi`).
- `fn`, `pub fn`, `let`, arrays, strings, calls, member access, empty classes, `new`, `trait`, and `impl`.
- Compile-time trait method resolution.
- A compiler-trusted core/prelude.
- `Len` for JavaScript `Array`, `String`, `Map`, and `Set`.
- Intrinsic lowering such as `values.len()` → `values.length`.
- Symbol-backed methods for user-defined trait implementations.
- A strict foreign-target rule: user code cannot implement traits for foreign/built-in types.
- JavaScript, `.d.ts`, and source-map output.
- No runtime dependency.

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

For example, core owns the built-in `Len` behavior, so this is available without an import:

```ts
[1, 2, 3].len()
```

but a package cannot define its own `impl MyTrait for Array<T>`.

See [`SPEC.md`](./SPEC.md) for the language direction and [`core/README.md`](./core/README.md) for the bootstrap core model.

## CLI

```text
sashimi build <file.sashimi> [--out-dir DIR] [--package NAME]
sashimi check <file.sashimi> [--package NAME]
```

`build` writes JavaScript, a TypeScript declaration file, and a source map to `dist/` by default.

## Compiler pipeline

```text
.sashimi
   ↓
lexer + parser
   ↓
AST
   ↓
semantic analysis / type inference
   ↓
trait + impl resolution
   ↓
JavaScript emitter ──→ .js + .map
Declaration emitter ─→ .d.ts
```

The current parser and IR are intentionally Sashimi-owned. Future TypeScript/JavaScript interoperability can use Oxc at the boundary without making the TypeScript AST the language's canonical representation.

## Development

```sh
cargo fmt --check
cargo test
cargo run -- check examples/hello.sashimi
cargo run -- build examples/trait.sashimi
```

## Near-term roadmap

1. Parse imports and consume `.d.ts` declarations.
2. Replace the bootstrap external-type model with a real module/type resolver.
3. Integrate Oxc for JS/TS parsing, resolution, and code generation where it improves compatibility.
4. Add generic bounds and generic trait implementations beyond the bootstrap subset.
5. Add diagnostics with real source spans throughout semantic analysis.
6. Emit useful source-map mappings rather than the current valid-but-empty mapping table.
7. Move core definitions into a distributed trusted package.
8. Define the public trait projection into `.d.ts`.

## License

MIT

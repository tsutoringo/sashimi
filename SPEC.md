# Sashimi Language Specification

> Status: Draft 0.1
>
> This document describes the initial direction of Sashimi. Syntax and details may change while the compiler is being prototyped.

## 1. Overview

Sashimi is a statically typed language designed to interoperate with the JavaScript and TypeScript ecosystem while adding language features that are difficult or unsafe to express directly in TypeScript.

The first defining feature is a Rust-like trait system that supports implementing shared behavior for classes without adding ordinary string-named methods to their prototypes.

Sashimi compiles to:

- JavaScript for runtime execution.
- TypeScript declaration files (`.d.ts`) for ecosystem interoperability.
- Source maps for debugging.

TypeScript is an input/interoperability language, not Sashimi's compilation target.

## 2. Design goals

Sashimi should:

1. Interoperate naturally with existing JavaScript and TypeScript packages.
2. Allow Rust-like traits and trait-based method lookup.
3. Keep trait resolution a compile-time operation.
4. Avoid string-named prototype extensions for trait methods.
5. Prevent arbitrary packages from modifying the behavior of foreign JavaScript/TypeScript classes.
6. Produce ordinary JavaScript that can run in existing JS runtimes and tooling.
7. Produce `.d.ts` files for public APIs that can be consumed from TypeScript.
8. Keep Sashimi's type system independent from TypeScript's type system where necessary.

## 3. Non-goals for the initial implementation

The initial implementation does not need to provide:

- Full TypeScript type-system compatibility.
- Full Rust trait semantics.
- Dynamic trait objects.
- Associated types.
- Specialization.
- Higher-ranked trait bounds.
- A stable ABI for trait implementation details.
- Perfect `.d.ts` projection for every Sashimi type-system feature.

These may be added later if they fit the language.

## 4. Source interoperability

### 4.1 JavaScript and TypeScript modules

Sashimi programs may import existing JavaScript and TypeScript modules.

```ts
import { User } from "some-package";
```

For external packages, runtime behavior comes from JavaScript while type information should normally come from TypeScript declarations.

Conceptually:

```text
package JavaScript  -> runtime dependency
package .d.ts       -> compiler type information
```

The compiler may also parse TypeScript source directly when declarations are unavailable or when compiling mixed-source projects.

### 4.2 TypeScript compatibility

Sashimi should reuse familiar TypeScript syntax where doing so does not conflict with Sashimi semantics.

TypeScript syntax support and TypeScript type-system compatibility are separate concerns. Accepting a TypeScript syntax form does not require reproducing every TypeScript type-checking rule.

## 5. Compiler output

Given a Sashimi source module, the compiler should normally emit:

```text
input.sashimi
  -> input.js
  -> input.d.ts
  -> input.js.map
```

The source extension is provisional.

Sashimi does not normally emit TypeScript source as an intermediate artifact.

## 6. Traits

A trait defines behavior that can be implemented by a type.

Provisional syntax:

```ts
trait Display {
    fn display(&self): string;
}
```

A trait method may be called with method syntax when a unique applicable implementation is visible:

```ts
value.display();
```

Trait method lookup is statically resolved by the compiler.

### 6.1 Method resolution

For an expression such as:

```ts
value.display();
```

resolution conceptually proceeds as follows:

1. Determine the static type of `value`.
2. Search inherent members on that type.
3. Search visible traits containing `display`.
4. Find an applicable `impl` for the receiver type.
5. Require a unique result.
6. Lower the call to its selected runtime representation.

Ambiguous trait calls are compile-time errors.

A qualified form will be provided for disambiguation. Exact syntax is not yet fixed.

Possible syntax:

```ts
Display::display(value);
```

## 7. Trait implementations

Provisional syntax:

```ts
impl Display for User {
    fn display(&self): string {
        return self.name;
    }
}
```

### 7.1 Coherence and foreign types

Normal packages may only define implementations whose target type is owned by that package.

Examples:

```text
local trait    + local type    -> allowed
foreign trait  + local type    -> allowed
local trait    + foreign type  -> rejected
foreign trait  + foreign type  -> rejected
```

This rule is intentionally stricter than Rust's orphan rule. In particular, defining a new local trait does not grant permission to augment an arbitrary external JavaScript/TypeScript class.

The purpose is to prevent dependencies from silently attaching competing behavior to shared foreign runtime classes.

## 8. Privileged core implementations

Sashimi ships with a compiler-trusted core library.

The core library is allowed to implement traits for foreign and built-in JavaScript types such as:

- `Array`
- `String`
- `Map`
- `Set`
- `Promise`
- `Date`
- typed arrays
- selected host/platform types where appropriate

Example:

```ts
trait Len {
    fn len(&self): number;
}

// Allowed only in compiler-trusted core code.
impl<T> Len for Array<T> {
    fn len(&self): number {
        return self.length;
    }
}
```

User packages cannot acquire this privilege by naming themselves `core` or by using a reserved package name. Privilege is attached to compiler-distributed/trusted package identity.

## 9. Prelude

The standard library defines a prelude containing commonly used types and traits.

Prelude names are automatically available to normal source modules without an explicit import.

Conceptually, every source module behaves as if it had imported something similar to:

```ts
use core::prelude::*;
```

The prelude is a name-resolution feature, not a separate runtime mechanism.

Name resolution should prefer ordinary lexical and explicit imports before prelude names.

The initial prelude is expected to contain commonly used standard traits and types rather than the entire core library.

## 10. Runtime representation of trait methods

Trait resolution happens at compile time. The runtime representation is an implementation detail and must not participate in overload/trait selection.

### 10.1 Symbol-backed methods

The default runtime representation for trait methods may use JavaScript `Symbol` keys.

For example, this Sashimi code:

```ts
trait Display {
    fn display(&self): string;
}

impl Display for User {
    fn display(&self): string {
        return self.name;
    }
}

user.display();
```

may lower approximately to:

```js
const __Display_display = Symbol("Display.display");

User.prototype[__Display_display] = function () {
    return this.name;
};

user[__Display_display]();
```

The actual compiler output may import a shared generated symbol rather than create it in the same module. All uses of one trait method must refer to the same symbol identity.

This design does not add string-named methods such as `User.prototype.display`, so it avoids ordinary method-name collisions. It still mutates a prototype when a symbol-backed implementation is installed; Sashimi should describe this precisely rather than treating it as mutation-free.

### 10.2 Core intrinsics

The compiler may lower selected core trait implementations directly to native JavaScript operations instead of installing a symbol property.

For example:

```ts
array.len()
```

could lower to:

```js
array.length
```

and:

```ts
map.len()
```

could lower to:

```js
map.size
```

This is an optimization and runtime-integration mechanism. It must preserve the same compile-time trait semantics as a normal implementation.

## 11. Type system boundary

Sashimi owns its internal type representation.

External TypeScript declarations are imported into that representation rather than becoming the compiler's canonical AST/type model.

Conceptually:

```text
TypeScript/.d.ts AST
        |
        v
Sashimi external type model
        |
        v
Sashimi type checking + trait resolution
```

This boundary allows Sashimi to introduce features that do not have exact TypeScript equivalents.

## 12. Declaration-file generation

The compiler emits `.d.ts` files from Sashimi's public API.

Example Sashimi API:

```ts
pub fn greet(user: User): string {
    return user.display();
}
```

Possible declaration output:

```ts
export declare function greet(user: User): string;
```

Private trait implementation details do not need to appear in declaration output.

Public trait APIs should be projected into TypeScript only when their runtime representation can be described accurately. Symbol-backed trait APIs may be represented with exported `unique symbol` declarations and symbol-keyed members.

Declaration generation is a projection from Sashimi types to TypeScript types; it does not define Sashimi's own type semantics.

## 13. Module resolution

Sashimi should follow the JavaScript ecosystem's package resolution rules closely enough to consume modern npm packages, including relevant `package.json` `exports` and TypeScript declaration entry points.

The compiler implementation may use an existing JS/TS resolver rather than reimplementing Node-compatible resolution from scratch.

## 14. Proposed compiler architecture

The compiler should keep parsing, semantic analysis, trait resolution, and code generation separate.

```text
Sashimi source
    |
    v
Sashimi parser -----> AST
                       |
TypeScript/.d.ts ------+----> HIR / symbols
                              |
                              v
                         type checking
                              |
                              v
                        trait resolution
                              |
                              v
                         typed HIR / IR
                           /       \
                          v         v
                    JS emitter   d.ts emitter
```

A likely implementation strategy is:

- Rust for the compiler.
- A hand-written recursive-descent parser with Pratt expression parsing for Sashimi-specific syntax.
- Oxc components for JavaScript/TypeScript parsing, code generation, and module-resolution support where appropriate.
- A Sashimi-owned HIR and type system.

The exact dependency choices are implementation details rather than language requirements.

## 15. Initial standard-library direction

The core library should be small and capability-oriented.

Potential initial traits include:

```text
Display
Debug
Eq
Ord
Len
IntoIterator
Iterator
From
Into
```

The exact set is not yet specified.

Foreign-type implementations belong in core, while the prelude only determines which names are automatically in scope.

## 16. MVP

The first compiler prototype should prove the following path end-to-end:

1. Parse a small TypeScript-like Sashimi program.
2. Import a type from an existing `.d.ts` file.
3. Parse `trait` declarations.
4. Parse `impl` declarations.
5. Enforce the foreign-target implementation rule.
6. Load one privileged core implementation for a JavaScript built-in type.
7. Make core/prelude traits automatically visible.
8. Resolve `value.method()` to an inherent or trait method at compile time.
9. Emit runnable JavaScript using symbol-backed trait methods or a core intrinsic.
10. Emit a minimal `.d.ts` for exported ordinary functions/types.
11. Emit source maps.

A useful first demonstration program would be:

```ts
fn main() {
    let values = [1, 2, 3];
    console.log(values.len());
}
```

with `Len for Array<T>` supplied by core and `Len` supplied by the prelude.

## 17. Open design questions

The following are deliberately left open until the first prototype provides implementation feedback:

- Final source-file extension.
- Final function/receiver syntax (`fn`, `self`, `&self`, etc.).
- Whether Sashimi accepts all ordinary TypeScript source syntax or a deliberately smaller compatible subset.
- Generic trait implementation syntax and constraints.
- Exact trait disambiguation syntax.
- Whether trait symbols are emitted per method or per trait/vtable.
- How public traits are represented in generated `.d.ts` files.
- How packages identify ownership of nominal and structural types.
- Whether structural object types may ever be implementation targets.
- Whether core foreign implementations are always intrinsic, symbol-backed, or chosen per implementation.
- How multiple physical copies/versions of a Sashimi package preserve trait symbol identity.

## 18. Guiding principle

Sashimi should treat TypeScript and JavaScript as its ecosystem boundary, not as the definition of the language itself.

The compiler is free to erase or lower Sashimi-only semantics into JavaScript as long as runtime behavior is correct, while `.d.ts` output exists to make the resulting public API usable from TypeScript.
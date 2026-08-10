# Sashimi core

The bootstrap compiler currently embeds the trusted core trait registry in `src/core.rs`.

Core is compiler-trusted: it may provide implementations for JavaScript built-ins such as `Array`, `String`, `Map`, and `Set`. Normal packages cannot opt into this privilege.

The initial prelude contains `Len`, with intrinsic implementations:

| Type | Sashimi | JavaScript lowering |
| --- | --- | --- |
| `Array<T>` | `value.len()` | `value.length` |
| `String` | `value.len()` | `value.length` |
| `Map<K, V>` | `value.len()` | `value.size` |
| `Set<T>` | `value.len()` | `value.size` |

As the language bootstraps, these definitions should move from compiler data structures into a distributed core package while the compiler retains a trusted package identity rather than a user-accessible "core mode".

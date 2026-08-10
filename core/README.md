# Sashimi core

The bootstrap compiler currently embeds the trusted core trait registry in `src/core.rs`.

Core is compiler-trusted: it may provide implementations for JavaScript built-ins such as `Array`, `String`, `Map`, and `Set`. Normal packages cannot opt into this privilege.

## Prelude traits

The bootstrap prelude currently exposes `Len`, `IntoIterator`, and `Iterator`.

### Len

| Type | Sashimi | JavaScript lowering |
| --- | --- | --- |
| `Array<T>` | `value.len()` | `value.length` |
| `String` | `value.len()` | `value.length` |
| `Map<K, V>` | `value.len()` | `value.size` |
| `Set<T>` | `value.len()` | `value.size` |

### IntoIterator

`Array<T>` and `Map<K, V>` implement the compiler-trusted `IntoIterator` behavior exposed as `iter()`.

```ts
let values = [1, 2, 3];
let iterator = values.iter();
```

`Array<T>.iter()` yields `T`. `Map<K, V>.iter()` follows JavaScript's native map iteration order and yields `[K, V]` entries.

The lowering creates a Sashimi-owned lazy iterator wrapper rather than exposing the native JavaScript iterator directly.

### Iterator

The bootstrap `Iterator<T>` runtime is lazy and implements the JavaScript iterable protocol. Adapter methods return another Sashimi iterator unless otherwise noted.

Adapters:

- `map(mapper)`
- `filter(predicate)`
- `take(count)`
- `skip(count)`
- `enumerate()`
- `chain(other)`
- `zip(other)`
- `inspect(callback)`
- `flat_map(mapper)`
- `flatten()`

Consumers:

- `next()`
- `collect()`
- `count()`
- `nth(index)`
- `last()`
- `find(predicate)`
- `position(predicate)`
- `any(predicate)`
- `all(predicate)`
- `fold(initial, folder)`
- `reduce(reducer)`
- `sum()`
- `product()`
- `min()`
- `max()`
- `for_each(callback)`

Iterator operations consume the iterator as they advance it. Calling `iter()` again on the original `Array` or `Map` creates a fresh iterator.

For TypeScript declaration output, public Sashimi `Iterator<T>` values are projected as `SashimiIterator<T>` so they do not pretend to be TypeScript's built-in `Iterator<T>`.

As the language bootstraps, these definitions should move from compiler data structures into a distributed core package while the compiler retains a trusted package identity rather than a user-accessible "core mode".

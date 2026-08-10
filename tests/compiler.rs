use sashimi::{CompileOptions, compile};

fn compile_source(source: &str) -> sashimi::CompileOutput {
    compile(source, &CompileOptions::default()).expect("source should compile")
}

#[test]
fn core_len_for_array_lowers_to_length() {
    let output = compile_source(
        r#"
fn main() {
    let values = [1, 2, 3];
    console.log(values.len());
}
"#,
    );
    assert!(
        output.javascript.contains("console.log(values.length)"),
        "{}",
        output.javascript
    );
}

#[test]
fn core_len_for_string_lowers_to_length() {
    let output = compile_source(
        r#"
fn main() {
    let value = "sashimi";
    console.log(value.len());
}
"#,
    );
    assert!(
        output.javascript.contains("console.log(value.length)"),
        "{}",
        output.javascript
    );
}

#[test]
fn local_trait_impl_uses_symbol_backed_method() {
    let output = compile_source(
        r#"
trait Display {
    fn display(&self): string;
}

class User {}

impl Display for User {
    fn display(&self): string {
        return "user";
    }
}

fn main() {
    let user = new User();
    console.log(user.display());
}
"#,
    );
    assert!(
        output
            .javascript
            .contains("Symbol.for(\"sashimi:main::Display.display\")"),
        "{}",
        output.javascript
    );
    assert!(
        output
            .javascript
            .contains("User.prototype[__sashimi_trait_Display_display]"),
        "{}",
        output.javascript
    );
    assert!(
        output
            .javascript
            .contains("user[__sashimi_trait_Display_display]()"),
        "{}",
        output.javascript
    );
}

#[test]
fn user_code_cannot_impl_for_foreign_builtin() {
    let error = compile(
        r#"
trait Display {
    fn display(&self): string;
}

impl Display for Array<number> {
    fn display(&self): string {
        return "array";
    }
}
"#,
        &CompileOptions::default(),
    )
    .expect_err("foreign impl should fail");
    assert!(
        error.message.contains("only compiler-trusted core"),
        "{}",
        error.message
    );
}

#[test]
fn exported_function_emits_dts() {
    let output = compile_source(
        r#"
pub fn greet(name: string): string {
    return name;
}
"#,
    );
    assert!(output.javascript.contains("export function greet(name)"));
    assert!(
        output
            .declarations
            .contains("export declare function greet(name: string): string;")
    );
    assert!(output.source_map.contains("\"version\":3"));
}

#[test]
fn core_len_for_map_lowers_to_size() {
    let output = compile_source(
        r#"
fn main() {
    let values = new Map();
    console.log(values.len());
}
"#,
    );
    assert!(
        output.javascript.contains("console.log(values.size)"),
        "{}",
        output.javascript
    );
}

#[test]
fn duplicate_impl_is_rejected() {
    let error = compile(
        r#"
trait Display {
    fn display(&self): string;
}

class User {}

impl Display for User {
    fn display(&self): string {
        return "first";
    }
}

impl Display for User {
    fn display(&self): string {
        return "second";
    }
}
"#,
        &CompileOptions::default(),
    )
    .expect_err("duplicate impl should fail");
    assert!(error.message.contains("conflicting implementations"));
}

#[test]
fn array_iter_builds_sashimi_iterator_and_chains_adapters() {
    let output = compile_source(
        r#"
fn identity(value: number): number {
    return value;
}

fn keep(value: number): boolean {
    return true;
}

fn main() {
    let values = [1, 2, 3, 4];
    let result = values.iter().map(identity).filter(keep).skip(1).take(2).enumerate().collect();
    console.log(result);
}
"#,
    );

    assert!(
        output.javascript.contains("class __SashimiIterator"),
        "{}",
        output.javascript
    );
    assert!(
        output.javascript.contains(
            "__sashimi_iter(values).map(identity).filter(keep).skip(1).take(2).enumerate().collect()"
        ),
        "{}",
        output.javascript
    );
}

#[test]
fn map_iter_uses_map_entry_iteration() {
    let output = compile_source(
        r#"
fn entries(values: Map<string, number>) {
    let result = values.iter().collect();
    console.log(result);
}
"#,
    );

    assert!(
        output
            .javascript
            .contains("const result = __sashimi_iter(values).collect()"),
        "{}",
        output.javascript
    );
    assert!(
        output
            .javascript
            .contains("const iteratorFactory = iterable[Symbol.iterator]"),
        "{}",
        output.javascript
    );
}

#[test]
fn iterator_consumers_and_combinators_are_available() {
    let output = compile_source(
        r#"
fn keep(value: number): boolean {
    return true;
}

fn combine(left: number, right: number): number {
    return left;
}

fn main() {
    let values = [1, 2, 3];
    console.log(values.iter().find(keep));
    console.log(values.iter().position(keep));
    console.log(values.iter().any(keep));
    console.log(values.iter().all(keep));
    console.log(values.iter().fold(0, combine));
    console.log(values.iter().sum());
    console.log(values.iter().product());
    console.log(values.iter().min());
    console.log(values.iter().max());
}
"#,
    );

    for method in [
        ".find(keep)",
        ".position(keep)",
        ".any(keep)",
        ".all(keep)",
        ".fold(0, combine)",
        ".sum()",
        ".product()",
        ".min()",
        ".max()",
    ] {
        assert!(output.javascript.contains(method), "missing {method}: {}", output.javascript);
    }
}

#[test]
fn iterator_public_type_projects_to_sashimi_iterator_dts() {
    let output = compile_source(
        r#"
pub fn iter_values(values: Array<number>): Iterator<number> {
    return values.iter();
}
"#,
    );

    assert!(
        output
            .declarations
            .contains("export interface SashimiIterator<T> extends IterableIterator<T>"),
        "{}",
        output.declarations
    );
    assert!(
        output.declarations.contains(
            "export declare function iter_values(values: Array<number>): SashimiIterator<number>;"
        ),
        "{}",
        output.declarations
    );
}

#[test]
fn iterator_core_methods_validate_arity() {
    let error = compile(
        r#"
fn main() {
    let values = [1, 2, 3];
    values.iter().take();
}
"#,
        &CompileOptions::default(),
    )
    .expect_err("take without count should fail");

    assert!(
        error.message.contains("expects 1 argument(s), found 0"),
        "{}",
        error.message
    );
}

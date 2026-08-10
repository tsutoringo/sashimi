use sashimi::{compile, CompileOptions};

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

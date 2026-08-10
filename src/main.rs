use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use sashimi::{CompileOptions, compile};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());
    match command.as_str() {
        "lsp" => sashimi::lsp::run_stdio(),
        "build" | "check" => {
            let input = args.next().ok_or_else(usage)?;
            let mut out_dir = PathBuf::from("dist");
            let mut package_name = "main".to_string();
            let rest = args.collect::<Vec<_>>();
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--out-dir" => {
                        i += 1;
                        out_dir = PathBuf::from(rest.get(i).ok_or("--out-dir requires a value")?.clone());
                    }
                    "--package" => {
                        i += 1;
                        package_name = rest.get(i).ok_or("--package requires a value")?.clone();
                    }
                    other => return Err(format!("unknown option `{other}`\n\n{}", usage())),
                }
                i += 1;
            }

            let source = fs::read_to_string(&input).map_err(|e| format!("failed to read {input}: {e}"))?;
            let stem = Path::new(&input)
                .file_stem()
                .and_then(|x| x.to_str())
                .ok_or("input must have a valid file name")?;
            let output_name = format!("{stem}.js");
            let options = CompileOptions {
                package_name,
                source_name: input.clone(),
                output_name: output_name.clone(),
            };
            let output = compile(&source, &options).map_err(|e| e.render(&source, &input))?;

            if command == "check" {
                println!("checked {input}");
                return Ok(());
            }

            fs::create_dir_all(&out_dir).map_err(|e| format!("failed to create {}: {e}", out_dir.display()))?;
            fs::write(out_dir.join(&output_name), output.javascript).map_err(|e| e.to_string())?;
            fs::write(out_dir.join(format!("{stem}.d.ts")), output.declarations).map_err(|e| e.to_string())?;
            fs::write(out_dir.join(format!("{output_name}.map")), output.source_map).map_err(|e| e.to_string())?;
            println!("built {input} -> {}", out_dir.display());
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "sashimi <build|check|lsp> [file.sashimi] [--out-dir DIR] [--package NAME]".to_string()
}

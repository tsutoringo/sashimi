use zed_extension_api as zed;

struct SashimiExtension;

impl zed::Extension for SashimiExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if let Some(command) = worktree.which("sashimi") {
            return Ok(zed::Command {
                command,
                args: vec!["lsp".to_string()],
                env: worktree.shell_env(),
            });
        }

        let is_sashimi_repo = worktree
            .read_text_file("Cargo.toml")
            .is_ok_and(|manifest| manifest.contains("name = \"sashimi\""));
        if is_sashimi_repo {
            if let Some(cargo) = worktree.which("cargo") {
                return Ok(zed::Command {
                    command: cargo,
                    args: vec![
                        "run".to_string(),
                        "--quiet".to_string(),
                        "--".to_string(),
                        "lsp".to_string(),
                    ],
                    env: worktree.shell_env(),
                });
            }
        }

        Err("Sashimi language server was not found. Install `sashimi` on PATH, or open the Sashimi repository with Cargo available (for example via `nix develop`).".to_string())
    }
}

zed::register_extension!(SashimiExtension);

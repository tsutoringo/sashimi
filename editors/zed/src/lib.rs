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
        let command = worktree.which("sashimi").ok_or_else(|| {
            "Sashimi language server was not found on PATH. Run `nix develop` or install the `sashimi` binary, then restart the language server.".to_string()
        })?;

        Ok(zed::Command {
            command,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(SashimiExtension);

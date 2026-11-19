use zed_extension_api::{
    self as zed, serde_json, settings::LspSettings, Command, LanguageServerId, Result, Worktree,
};

struct StackExtension;

impl zed::Extension for StackExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let settings = LspSettings::for_worktree(id.as_ref(), worktree)?;
        let binary = settings.binary;
        let command = binary.as_ref().and_then(|b| b.path.clone())
            .or_else(|| worktree.which("stack-lang-server"))
            .ok_or("Stack language server was not found. Set lsp.stack-lang-server.binary.path or add stack-lang-server to PATH.")?;
        let mut env = worktree.shell_env();
        if let Some(overrides) = binary.as_ref().and_then(|b| b.env.as_ref()) {
            env.extend(
                overrides
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        }
        Ok(Command {
            command,
            args: binary
                .as_ref()
                .and_then(|b| b.arguments.clone())
                .unwrap_or_default(),
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree(id.as_ref(), worktree)?;
        let mut options = settings
            .initialization_options
            .unwrap_or_else(|| serde_json::json!({}));
        let object = options
            .as_object_mut()
            .ok_or("Stack initialization_options must be an object.")?;
        object.insert("lens_enabled".into(), serde_json::json!(false));
        object.entry("ini_path").or_insert(serde_json::json!(""));
        Ok(Some(options))
    }
}

zed::register_extension!(StackExtension);

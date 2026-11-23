use zed_extension_api::{
    self as zed, serde_json,
    settings::{CommandSettings, LspSettings},
    Command, LanguageServerId, Result, Worktree,
};

mod server_download;

struct StackExtension;

fn resolve_command(
    binary: Option<&CommandSettings>,
    shell_env: zed::EnvVars,
    discover: impl FnOnce() -> Result<String>,
) -> Result<Command> {
    let explicit_path = binary.and_then(|b| b.path.clone());
    let command = match explicit_path {
        Some(path) => path,
        None => discover()?,
    };
    let args = binary.and_then(|b| b.arguments.clone()).unwrap_or_default();
    let mut env: std::collections::BTreeMap<_, _> = shell_env.into_iter().collect();
    if let Some(overrides) = binary.and_then(|b| b.env.as_ref()) {
        env.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    Ok(Command {
        command,
        args,
        env: env.into_iter().collect(),
    })
}

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
        resolve_command(
            settings.binary.as_ref(),
            worktree.shell_env(),
            || match worktree.which("stack-lang-server") {
                Some(path) => Ok(path),
                None => server_download::install(id),
            },
        )
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
        object
            .entry("lens_enabled")
            .or_insert(serde_json::json!(true));
        object.entry("ini_path").or_insert(serde_json::json!(""));
        Ok(Some(options))
    }
}

zed::register_extension!(StackExtension);

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(value: serde_json::Value) -> CommandSettings {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn configured_binary_takes_precedence_and_overrides_shell_environment() {
        let binary = settings(serde_json::json!({
            "path": "C:/server with spaces/stack-lang-server.exe",
            "arguments": ["--flag"],
            "env": {"COMMON": "configured", "CONFIG_ONLY": "configured"}
        }));
        let command = resolve_command(
            Some(&binary),
            vec![
                ("COMMON".into(), "shell".into()),
                ("SHELL_ONLY".into(), "shell".into()),
            ],
            || panic!("Explicit binary path must take precedence over PATH"),
        )
        .unwrap();
        assert_eq!(command.command, binary.path.as_ref().unwrap().as_str());
        assert_eq!(command.args, ["--flag"]);
        let env: std::collections::BTreeMap<_, _> = command.env.iter().cloned().collect();
        assert_eq!(command.env.len(), env.len());
        assert_eq!(env["COMMON"], "configured");
        assert_eq!(env["CONFIG_ONLY"], "configured");
        assert_eq!(env["SHELL_ONLY"], "shell");
    }

    #[test]
    fn path_lookup_is_used_only_without_configured_paths() {
        let binary = settings(serde_json::json!({"arguments": ["--flag"]}));
        let command =
            resolve_command(Some(&binary), vec![], || Ok("from-path.exe".into())).unwrap();
        assert_eq!(command.command, "from-path.exe");
        assert_eq!(command.args, ["--flag"]);
        assert!(resolve_command(None, vec![], || Err("Discovery failed".into())).is_err());
    }
}

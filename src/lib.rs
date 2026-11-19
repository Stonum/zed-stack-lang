use zed_extension_api::{
    self as zed, serde_json,
    settings::{CommandSettings, LspSettings},
    Command, LanguageServerId, Result, Worktree,
};

struct StackExtension;

fn resolve_command(
    binary: Option<&CommandSettings>,
    fallback: Option<&CommandSettings>,
    shell_env: zed::EnvVars,
    which: impl FnOnce() -> Option<String>,
) -> Result<Command> {
    let command = binary
        .and_then(|b| b.path.clone())
        .or_else(|| fallback.and_then(|b| b.path.clone()))
        .or_else(which)
        .ok_or("Stack language server was not found. Set lsp.stack-lang-server.binary.path (or lsp.stack-sql-server.binary.path for SQL) or add stack-lang-server to PATH.")?;
    let args = binary
        .and_then(|b| b.arguments.clone())
        .or_else(|| fallback.and_then(|b| b.arguments.clone()))
        .unwrap_or_default();
    let mut env: std::collections::BTreeMap<_, _> = shell_env.into_iter().collect();
    for settings in [fallback, binary].into_iter().flatten() {
        if let Some(overrides) = &settings.env {
            env.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
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
        let fallback = if id.as_ref() == "stack-sql-server" {
            LspSettings::for_worktree("stack-lang-server", worktree)?.binary
        } else {
            None
        };
        resolve_command(
            settings.binary.as_ref(),
            fallback.as_ref(),
            worktree.shell_env(),
            || worktree.which("stack-lang-server"),
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
        object.insert("lens_enabled".into(), serde_json::json!(false));
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
    fn partial_sql_settings_preserve_base_path_and_merge_environment() {
        let base = settings(serde_json::json!({
            "path": "C:/server with spaces/stack-lang-server.exe",
            "arguments": ["--base"],
            "env": {"COMMON": "base", "BASE_ONLY": "base"}
        }));
        for sql in [
            settings(serde_json::json!({"arguments": []})),
            settings(serde_json::json!({"env": {"COMMON": "sql"}})),
        ] {
            let command = resolve_command(
                Some(&sql),
                Some(&base),
                vec![
                    ("COMMON".into(), "shell".into()),
                    ("SHELL_ONLY".into(), "shell".into()),
                ],
                || panic!("Explicit base path must take precedence over PATH"),
            )
            .unwrap();
            assert_eq!(command.command, base.path.as_ref().unwrap().as_str());
            assert_eq!(
                command.args,
                sql.arguments
                    .as_ref()
                    .or(base.arguments.as_ref())
                    .unwrap()
                    .clone()
            );
            let env: std::collections::BTreeMap<_, _> = command.env.iter().cloned().collect();
            assert_eq!(command.env.len(), env.len());
            assert_eq!(
                env["COMMON"],
                if sql.env.is_some() { "sql" } else { "base" }
            );
            assert_eq!(env["BASE_ONLY"], "base");
            assert_eq!(env["SHELL_ONLY"], "shell");
        }
    }

    #[test]
    fn sql_path_overrides_base_and_absent_sql_inherits_base() {
        let base = settings(serde_json::json!({"path": "base.exe", "arguments": ["--base"]}));
        let sql = settings(serde_json::json!({"path": "sql.exe", "arguments": ["--sql"]}));
        let command =
            resolve_command(Some(&sql), Some(&base), vec![], || panic!("PATH used")).unwrap();
        assert_eq!(command.command, "sql.exe");
        assert_eq!(command.args, ["--sql"]);
        let command = resolve_command(None, Some(&base), vec![], || panic!("PATH used")).unwrap();
        assert_eq!(command.command, "base.exe");
        assert_eq!(command.args, ["--base"]);
    }

    #[test]
    fn path_lookup_is_used_only_without_configured_paths() {
        let sql = settings(serde_json::json!({"arguments": ["--sql"]}));
        let command =
            resolve_command(Some(&sql), None, vec![], || Some("from-path.exe".into())).unwrap();
        assert_eq!(command.command, "from-path.exe");
        assert_eq!(command.args, ["--sql"]);
        assert!(resolve_command(None, None, vec![], || None).is_err());
    }
}

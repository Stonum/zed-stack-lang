use serde_json::{json, Value};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use url::Url;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const CLI_VERSION: &str = "tree-sitter 0.25.10";

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("generate") => grammar(root, "generate"),
        Some("test") => grammar(root, "test"),
        Some("queries") => grammar(root, "queries"),
        Some("xml-queries") => grammar(root, "xml-queries"),
        Some("smoke") => smoke(
            root,
            Path::new(&args.next().ok_or("Expected server executable path")?),
        ),
        _ => Err(
            "Usage: cargo run -p stack-dev -- <generate|test|queries|xml-queries|smoke SERVER>"
                .into(),
        ),
    }
}

fn grammar(root: &Path, action: &str) -> Result<()> {
    let local = root
        .join(".local/tree-sitter-cli/bin")
        .join(if cfg!(windows) {
            "tree-sitter.exe"
        } else {
            "tree-sitter"
        });
    let cli = if local.is_file() {
        local
    } else {
        PathBuf::from("tree-sitter")
    };
    let version = Command::new(&cli).arg("--version").output()?;
    let version_text = String::from_utf8_lossy(&version.stdout);
    if !version.status.success()
        || version_text
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
            != CLI_VERSION
    {
        return Err(format!("Expected {CLI_VERSION}; install with cargo install tree-sitter-cli --version =0.25.10 --locked --root .local/tree-sitter-cli").into());
    }
    let run = |args: &[&str]| -> Result<()> {
        let status = Command::new(&cli)
            .current_dir(if action == "xml-queries" {
                root.join(".local/tree-sitter-xml/xml")
            } else {
                root.join("grammars/stack")
            })
            .args(args)
            .status()?;
        if !status.success() {
            return Err(format!("tree-sitter {args:?} failed: {status}").into());
        }
        Ok(())
    };
    match action {
        "xml-queries" => {
            for language in ["stack-resource", "stack-dictionary"] {
                for query in ["highlights", "brackets", "indents", "outline"] {
                    run(&[
                        "query",
                        &format!("../../../languages/{language}/{query}.scm"),
                        "../../../examples/fixtures/sample.rx_api",
                        "--quiet",
                    ])?;
                }
            }
            Ok(())
        }
        "generate" => run(&["generate", "--abi", "14"]),
        "test" => run(&["test"]),
        "queries" => {
            for query in ["highlights", "brackets", "indents", "outline"] {
                run(&[
                    "query",
                    &format!("../../languages/stack/{query}.scm"),
                    "-n",
                    "1",
                    "--quiet",
                ])?;
                run(&[
                    "query",
                    &format!("../../languages/stack/{query}.scm"),
                    "../../examples/fixtures/sample.prg",
                    "--quiet",
                ])?;
                run(&[
                    "query",
                    &format!("../../languages/stack/{query}.scm"),
                    "../../examples/fixtures/sample.hdl",
                    "--quiet",
                ])?;
                println!("{query}: OK");
            }
            Ok(())
        }
        _ => unreachable!(),
    }
}

// A native development client, independent of the extension's WASM runtime.
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    messages: Receiver<std::result::Result<Value, String>>,
    trace: File,
    diagnostics: Vec<Value>,
    requests: Vec<Value>,
    folders: Value,
    serial: u64,
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Client {
    fn start(executable: &Path, base: &Path, folders: Value) -> Result<Self> {
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(base.join("stderr.txt"))?)
            .spawn()?;
        let input = child.stdin.take().ok_or("Missing stdin")?;
        let output = child.stdout.take().ok_or("Missing stdout")?;
        let (tx, messages) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                match read_message(&mut reader) {
                    Ok(message) => {
                        if tx.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error.to_string()));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            input: Some(input),
            messages,
            trace: File::create(base.join("trace.jsonl"))?,
            diagnostics: Vec::new(),
            requests: Vec::new(),
            folders,
            serial: 0,
        })
    }

    fn record(&mut self, direction: &str, message: &Value) -> Result<()> {
        writeln!(
            self.trace,
            "{}",
            json!({"direction":direction,"message":message})
        )?;
        self.trace.flush()?;
        Ok(())
    }

    fn send(&mut self, mut message: Value) -> Result<()> {
        if message.get("params").is_some_and(Value::is_null) {
            message.as_object_mut().unwrap().remove("params");
        }
        message["jsonrpc"] = json!("2.0");
        self.record("client", &message)?;
        let body = serde_json::to_vec(&message)?;
        write!(
            self.input.as_mut().ok_or("Closed stdin")?,
            "Content-Length: {}\r\n\r\n",
            body.len()
        )?;
        self.input
            .as_mut()
            .ok_or("Closed stdin")?
            .write_all(&body)?;
        self.input.as_mut().ok_or("Closed stdin")?.flush()?;
        Ok(())
    }

    fn receive(&mut self, deadline: Instant) -> Result<Value> {
        let message = self
            .messages
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))?
            .map_err(std::io::Error::other)?;
        self.record("server", &message)?;
        if message["method"] == "textDocument/publishDiagnostics" {
            self.diagnostics.push(message.clone());
        }
        if message.get("method").is_some() && message.get("id").is_some() {
            self.requests.push(message.clone());
            let result = if message["method"] == "workspace/workspaceFolders" {
                self.folders.clone()
            } else {
                Value::Null
            };
            self.send(json!({"id":message["id"],"result":result}))?;
        }
        Ok(message)
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.serial += 1;
        let id = self.serial;
        self.send(json!({"id":id,"method":method,"params":params}))?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let message = self.receive(deadline)?;
            if message["id"] == id && message.get("method").is_none() {
                if let Some(error) = message.get("error") {
                    return Err(format!("{method}: {error}").into());
                }
                return Ok(message["result"].clone());
            }
        }
    }

    fn notification(&mut self, method: &str, params: Value) -> Result<()> {
        self.send(json!({"method":method,"params":params}))
    }

    fn diagnostics_for(&mut self, uri: &str, version: u64, errors: bool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if self
                .diagnostics
                .iter()
                .any(|m| matches_diagnostics(m, uri, version, errors))
            {
                return Ok(());
            }
            self.receive(deadline)?;
        }
    }
}

fn matches_diagnostics(message: &Value, uri: &str, version: u64, errors: bool) -> bool {
    let params = &message["params"];
    params["uri"] == uri
        && params["version"] == version
        && params["diagnostics"]
            .as_array()
            .is_some_and(|d| d.is_empty() != errors)
}

fn read_message(reader: &mut impl BufRead) -> Result<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err("Server stdout closed".into());
        }
        if line == "\r\n" {
            break;
        }
        let (name, value) = line
            .trim_end()
            .split_once(':')
            .ok_or("Unframed data on stdout")?;
        if name.eq_ignore_ascii_case("Content-Length") {
            length = Some(value.trim().parse::<usize>()?);
        }
    }
    let length = length
        .filter(|n| *n <= 16 * 1024 * 1024)
        .ok_or("Missing or excessive Content-Length")?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn uri(path: &Path) -> Result<String> {
    Ok(Url::from_file_path(path)
        .map_err(|_| "Invalid file URI path")?
        .into())
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

fn smoke(root: &Path, source: &Path) -> Result<()> {
    let base = root.join(".local/lsp-smoke");
    fs::create_dir_all(&base)?;
    let executable = base.join(if cfg!(windows) {
        "server with spaces.exe"
    } else {
        "server with spaces"
    });
    fs::copy(source, &executable)?;
    let roots = [base.join("root one"), base.join("root two")];
    for root in &roots {
        fs::create_dir_all(root)?;
    }
    let folders = json!([{"uri":uri(&roots[0])?,"name":"root one"},{"uri":uri(&roots[1])?,"name":"root two"}]);
    fs::write(
        roots[1].join("helpers.hdl"),
        "func приветМодуль() { return 2; }\n",
    )?;
    let file = roots[0].join("sample.prg");
    let text = "func привет() { return 1; }\nvar число=привет();\n";
    fs::write(&file, text)?;
    let doc_uri = uri(&file)?;
    let doc = json!({"uri":doc_uri});
    let mut client = Client::start(&executable, &base, folders.clone())?;
    let capabilities = client.request("initialize", json!({"processId":std::process::id(),"rootUri":uri(&roots[0])?,"workspaceFolders":folders,"capabilities":{"workspace":{"workspaceFolders":true,"didChangeWatchedFiles":{"dynamicRegistration":true}},"textDocument":{"hover":{"contentFormat":["markdown"]},"completion":{"completionItem":{"snippetSupport":true}}}},"initializationOptions":{"lens_enabled":false,"ini_path":""}}))?;
    if !capabilities["capabilities"]["codeLensProvider"].is_null()
        && capabilities["capabilities"]["codeLensProvider"] != false
    {
        return Err("CodeLens unexpectedly enabled".into());
    }
    if capabilities["capabilities"]["documentRangeFormattingProvider"] != true {
        return Err("Range formatting not advertised".into());
    }
    client.notification("initialized", json!({}))?;
    // Stack currently exposes readiness through this notification rather than LSP progress.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let message = client.receive(deadline)?;
        if message["method"] == "custom/statusBar" && message["params"]["text"] == "" {
            break;
        }
    }
    client.notification(
        "textDocument/didOpen",
        json!({"textDocument":{"uri":doc["uri"],"languageId":"stack","version":1,"text":text}}),
    )?;
    client.diagnostics_for(&doc_uri, 1, false)?;
    let mut results = json!({"capabilities":capabilities,"actualExecutable":executable});
    for method in [
        "hover",
        "completion",
        "documentSymbol",
        "semanticTokens/full",
        "rangeFormatting",
    ] {
        let value = client.request(&format!("textDocument/{method}"), json!({"textDocument":doc,"position":{"line":1,"character":14},"range":{"start":{"line":0,"character":0},"end":{"line":2,"character":0}},"options":{"tabSize":4,"insertSpaces":true},"context":{"triggerKind":1}}))?;
        let content = if method == "semanticTokens/full" {
            &value["data"]
        } else {
            &value
        };
        let valid = if method == "completion" {
            value.is_array() || value["items"].is_array()
        } else {
            nonempty(content)
        };
        if !valid {
            return Err(format!("Unexpected empty or invalid {method} response").into());
        }
        results[method] = value;
    }
    for (version, content, errors) in [(2, "func broken( {\n", true), (3, text, false)] {
        client.notification("textDocument/didChange", json!({"textDocument":{"uri":doc["uri"],"version":version},"contentChanges":[{"text":content}]}))?;
        client.diagnostics_for(&doc_uri, version, errors)?;
    }
    for (name, language, text, tab) in [
        (
            "resource.rx_api",
            "stack-resource",
            "<resources><item name=\"Example\"/></resources>\n",
            3,
        ),
        (
            "dictionary.xdic",
            "stack-dictionary",
            "<dictionary><table name=\"Example\"/></dictionary>\n",
            4,
        ),
        ("query.sql", "sql", "select 1 as example;\n", 4),
    ] {
        let path = roots[0].join(name);
        fs::write(&path, text)?;
        let document_uri = uri(&path)?;
        let document = json!({"uri":document_uri});
        client.notification("textDocument/didOpen", json!({"textDocument":{"uri":document["uri"],"languageId":language,"version":1,"text":text}}))?;
        client.diagnostics_for(&document_uri, 1, false)?;
        let edits = client.request("textDocument/rangeFormatting", json!({"textDocument":document,"range":{"start":{"line":0,"character":0},"end":{"line":1,"character":0}},"options":{"tabSize":tab,"insertSpaces":true}}))?;
        if edits.as_array().is_none_or(|edits| edits.is_empty()) {
            return Err(format!("Expected nonempty formatting edits for {name}").into());
        }
        for (version, content, errors) in [
            (
                2,
                if language == "sql" {
                    "select ("
                } else {
                    "<broken"
                },
                true,
            ),
            (3, text, false),
        ] {
            client.notification("textDocument/didChange", json!({"textDocument":{"uri":document["uri"],"version":version},"contentChanges":[{"text":content}]}))?;
            client.diagnostics_for(&document_uri, version, errors)?;
        }
        results[name] = json!({"languageId":language,"formatting":edits});
    }
    client.request("shutdown", Value::Null)?;
    client.notification("exit", Value::Null)?;
    drop(client.input.take());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = client.child.try_wait()? {
            if !status.success() {
                return Err(format!("Server exited with {status}").into());
            }
            break;
        }
        if Instant::now() >= deadline {
            return Err("Server did not exit after shutdown".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
    results["diagnostics"] = json!(client.diagnostics);
    results["serverRequests"] = json!(client.requests);
    fs::write(
        base.join("results.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    let completions = results["completion"]
        .as_array()
        .or_else(|| results["completion"]["items"].as_array())
        .unwrap()
        .len();
    println!("LSP smoke OK: hover, symbols, tokens, formatting, diagnostics, shutdown; completion response: {completions} items");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_must_match_document_version_and_expected_state() {
        let uri = "file:///sample.sql";
        let error =
            json!({"params":{"uri":uri,"version":2,"diagnostics":[{"message":"Syntax error"}]}});
        let clear = json!({"params":{"uri":uri,"version":3,"diagnostics":[]}});
        assert!(matches_diagnostics(&error, uri, 2, true));
        assert!(matches_diagnostics(&clear, uri, 3, false));
        for (message, version, errors) in [(&error, 2, true), (&clear, 3, false)] {
            assert!(!matches_diagnostics(
                message,
                "file:///dictionary.xdic",
                version,
                errors
            ));
            assert!(!matches_diagnostics(message, uri, version + 1, errors));
            assert!(!matches_diagnostics(message, uri, version, !errors));
        }
        assert!(!matches_diagnostics(
            &json!({"params":{"uri":uri,"version":3}}),
            uri,
            3,
            false
        ));
    }
}

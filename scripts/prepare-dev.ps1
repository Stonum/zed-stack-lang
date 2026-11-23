$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$source = Join-Path $root 'tree-sitter/stack'
$repo = Join-Path $root '.local/stack-grammar.git'
$checkout = Join-Path $root 'grammars/stack'

function Invoke-SnapshotGit {
    & git @args
    if ($LASTEXITCODE -ne 0) {
        throw "Git failed while preparing the Stack grammar snapshot (exit code $LASTEXITCODE)."
    }
}

if (!(Test-Path -LiteralPath (Join-Path $repo 'HEAD'))) {
    Invoke-SnapshotGit init --quiet --bare $repo
}
$snapshot = @('-C', $source, "--git-dir=$repo", "--work-tree=$source")
Invoke-SnapshotGit @snapshot -c core.autocrlf=false add --all -- grammar.js src test
$tree = Invoke-SnapshotGit @snapshot write-tree
$rev = & git @snapshot rev-parse --verify --quiet HEAD
if ($LASTEXITCODE -ne 0 -or (Invoke-SnapshotGit @snapshot rev-parse 'HEAD^{tree}') -ne $tree) {
    $parent = if ($rev) { @('-p', $rev) } else { @() }
    $rev = Invoke-SnapshotGit @snapshot -c 'user.name=Local grammar snapshot' -c 'user.email=local@example.invalid' commit-tree $tree @parent -m 'Snapshot Stack grammar'
    Invoke-SnapshotGit @snapshot update-ref HEAD $rev
}

$uri = ([System.Uri]($repo.Replace('\', '/') + '/')).AbsoluteUri.TrimEnd('/')
$manifestPath = Join-Path $root 'extension.toml'
$manifest = [System.IO.File]::ReadAllText((Join-Path $root 'extension.toml.in'))
$manifest = $manifest.TrimEnd() + @"


[grammars.stack]
repository = "$uri"
rev = "$rev"
"@ + "`n"

# Native Tree-sitter tests from older checkouts may have left build files here.
if ((Test-Path -LiteralPath $checkout) -and !(Test-Path -LiteralPath (Join-Path $checkout '.git'))) {
    $backup = Join-Path $root ('.local/previous-stack-grammar-' + [guid]::NewGuid().ToString('N'))
    Move-Item -LiteralPath $checkout -Destination $backup
    Write-Output "Moved previous grammar directory to: $backup"
}

[System.IO.File]::WriteAllText($manifestPath, $manifest)
Write-Output "Install dev extension from: $root"

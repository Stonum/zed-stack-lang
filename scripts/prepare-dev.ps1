$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$dev = Join-Path $root '.local/dev-extension'
$repo = Join-Path $root '.local/grammar-source'
New-Item -ItemType Directory -Force $dev,$repo | Out-Null
Copy-Item "$root/grammars/stack/grammar.js","$root/grammars/stack/src","$root/grammars/stack/test" $repo -Recurse -Force
if (!(Test-Path "$repo/.git")) { git -C $repo init; if ($LASTEXITCODE) { throw 'git init failed' } }
git -C $repo add grammar.js src test
git -C $repo -c user.name='Local grammar snapshot' -c user.email='local@example.invalid' commit -m 'Snapshot Stack grammar' --allow-empty
if ($LASTEXITCODE) { throw 'grammar snapshot failed' }
$rev = git -C $repo rev-parse HEAD
Copy-Item "$root/Cargo.toml","$root/Cargo.lock" $dev -Force
Copy-Item "$root/src","$root/languages","$root/tools","$root/licenses" $dev -Recurse -Force
$uri = ([System.Uri]($repo.Replace('\','/') + '/')).AbsoluteUri.TrimEnd('/')
$manifest = Get-Content "$root/extension.toml" -Raw
$manifest += "`n[grammars.stack]`nrepository = `"$uri`"`nrev = `"$rev`"`n"
Set-Content "$dev/extension.toml" $manifest
Write-Output "Install dev extension from: $dev"

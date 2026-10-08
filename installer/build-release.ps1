# Builds everything a Brain.Skills release needs:
#   bin\brain-system.exe                  (Rust, brain-system/)
#   bin\skills.exe                        (Go, agent-skills-installer/)
#   dist\BrainSkills-Setup.exe            (Go bootstrapper, product=core)
#   dist\Skills-Installer-Hub-Setup.exe   (Go bootstrapper, product=hub)
#
# Usage (from anywhere):  powershell -ExecutionPolicy Bypass -File installer\build-release.ps1
# Then commit bin\*.exe, push, and attach dist\*.exe to a GitHub release
# (installer\publish-release.ps1 does that).
# Requires: cargo, go. Installed copies update from the repo's main branch,
# so the bin\ binaries must be committed for users to receive them.

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$version = (Get-Content (Join-Path $repoRoot 'VERSION') -Raw).Trim()
$dist = Join-Path $repoRoot 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null

function Step($m) { Write-Host "`n== $m" -ForegroundColor Cyan }
function Check($what) { if ($LASTEXITCODE -ne 0) { throw "$what failed (exit $LASTEXITCODE)" } }

Step "brain-system (Rust) -> bin\brain-system.exe"
Push-Location (Join-Path $repoRoot 'brain-system')
cargo test --release --quiet; Check 'cargo test'
cargo build --release --quiet; Check 'cargo build'
Pop-Location
Copy-Item (Join-Path $repoRoot 'brain-system\target\release\brain-system.exe') (Join-Path $repoRoot 'bin\brain-system.exe') -Force

Step "skills CLI (Go) -> bin\skills.exe"
Push-Location (Join-Path $repoRoot 'agent-skills-installer')
go build -trimpath -ldflags "-s -w" -o (Join-Path $repoRoot 'bin\skills.exe') ./cmd/agent-installer; Check 'go build skills'
Pop-Location

Step "setup bootstrappers (Go) -> dist\"
$setup = Join-Path $PSScriptRoot 'setup'
$scripts = Join-Path $setup 'scripts'
New-Item -ItemType Directory -Force -Path $scripts | Out-Null
Copy-Item (Join-Path $repoRoot 'install.ps1') $scripts -Force
Copy-Item (Join-Path $repoRoot 'install-skills.ps1') $scripts -Force
Push-Location $setup
# Icon + asInvoker manifest (no UAC prompt for a per-user installer).
go run github.com/akavel/rsrc@v0.10.2 -manifest setup.manifest -ico (Join-Path $repoRoot 'assets\brain.ico') -o rsrc_windows_amd64.syso; Check 'rsrc'
foreach ($p in @(@{ product = 'core'; out = 'BrainSkills-Setup.exe' }, @{ product = 'hub'; out = 'Skills-Installer-Hub-Setup.exe' })) {
    $env:GOOS = 'windows'; $env:GOARCH = 'amd64'
    go build -trimpath -ldflags "-s -w -X main.product=$($p.product) -X main.version=$version" -o (Join-Path $dist $p.out) .; Check "go build $($p.out)"
}
Pop-Location

Step "done"
Get-ChildItem (Join-Path $repoRoot 'bin\*.exe'), (Join-Path $dist '*.exe') | ForEach-Object {
    '{0,-34} {1,8:N0} KB  {2}' -f $_.Name, ($_.Length / 1KB), (Get-FileHash $_.FullName -Algorithm SHA256).Hash.Substring(0, 16)
}

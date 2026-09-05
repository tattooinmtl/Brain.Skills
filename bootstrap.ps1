# bootstrap.ps1 — clone Brain.Skills and run native Go installer
$ErrorActionPreference = 'Stop'

$RepoUrl = 'https://github.com/tattooinmtl/Brain.Skills.git'
$Dest    = 'C:\.skills'

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Error 'git is required (install from https://git-scm.com)'
    exit 1
}

if (Test-Path (Join-Path $Dest '.git')) {
    Write-Host "Existing checkout at $Dest - pulling latest updates..."
    git -C $Dest fetch --quiet origin
    git -C $Dest pull --ff-only
} elseif (Test-Path $Dest) {
    Write-Error "$Dest exists but is not a git repository."
    exit 1
} else {
    Write-Host "Cloning $RepoUrl to $Dest..."
    git clone --depth 20 $RepoUrl $Dest
}

$exe = Join-Path $Dest 'bin\skills.exe'
if (-not (Test-Path $exe)) {
    if (Get-Command go -ErrorAction SilentlyContinue) {
        Write-Host "Compiling native Go launcher..."
        $srcDir = Join-Path $Dest 'agent-skills-installer\cmd\agent-installer'
        go build -o $exe $srcDir
    } else {
        Write-Error "Executable $exe not found and Go compiler is missing."
        exit 1
    }
}

Write-Host 'Running native Go installer...'
$installArgs = $args
if ($installArgs.Count -eq 0) {
    & $exe install --auto
} else {
    & $exe install @installArgs
}

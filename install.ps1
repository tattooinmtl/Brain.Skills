# Brain.Skills installer, updater and uninstaller for Windows.
#
# Install (or run BrainSkills-Setup.exe from the GitHub releases page):
#   irm https://raw.githubusercontent.com/tattooinmtl/Brain.Skills/main/install.ps1 | iex
#
# Installs per user (no administrator) into %USERPROFILE%\.brain-skills:
#   app\bin\brain-system.exe   the Global Brain tray app: 3D neural view, wiki,
#                              Verifier Librarian, MCP server for Claude
#   app\bin\skills.exe         the skills CLI (also put on your PATH)
#   memory\                    your memory vault (Obsidian-style notes)
#   brain.json                 where your memory vault and skills live
# The skills library itself comes from the separate Skills Installer Hub
# (install-skills.ps1 / Skills-Installer-Hub-Setup.exe).
#
# Update: the tray app checks GitHub and runs this script (saved as
# update.ps1). Only files whose content changed are downloaded.
# Uninstall: Windows Settings > Apps, or  update.ps1 -Uninstall
#
# Your notes are never modified or deleted by this script. A vault in the old
# location (C:\.skills\memory) is copied into memory\ and verified; the old
# folder is then renamed to memory.moved-<date>, not deleted.
# Everything runs in one script block that ends with `return`, never `exit`:
# through `irm | iex`, `exit` would close your PowerShell window.

$BrainSkillsArgs = @($args)
& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $Repo = if ($env:BRAINSKILLS_REPO) { $env:BRAINSKILLS_REPO } else { 'tattooinmtl/Brain.Skills' }
    $Branch = if ($env:BRAINSKILLS_BRANCH) { $env:BRAINSKILLS_BRANCH } else { 'main' }
    $Uninstall = $BrainSkillsArgs -contains '-Uninstall'
    # Run as <root>\update.ps1 = updating that install; anything else = installing.
    $Updating = $PSScriptRoot -and (Test-Path (Join-Path $PSScriptRoot 'installed.json'))
    $Root = if ($Updating) { $PSScriptRoot } elseif ($env:BRAINSKILLS_HOME) { $env:BRAINSKILLS_HOME } else { Join-Path $HOME '.brain-skills' }
    $AppDir = Join-Path $Root 'app'
    $BinDir = Join-Path $AppDir 'bin'
    $Exe = Join-Path $BinDir 'brain-system.exe'
    $StateFile = Join-Path $Root 'installed.json'
    $ConfigFile = Join-Path $Root 'brain.json'
    $UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BrainSkills'
    $Utf8 = New-Object System.Text.UTF8Encoding $false
    $Headers = @{ 'User-Agent' = 'BrainSkills-installer' }
    # Repo files this installer manages. Keep in sync with in_core() in brain-system/src/updater.rs.
    $Managed = @('bin/brain-system.exe', 'bin/skills.exe', 'assets/brain.ico', 'assets/logo.jpg')

    function Say($msg, $color = 'Gray') { Write-Host "  $msg" -ForegroundColor $color }
    function Write-Json($path, $obj) { [IO.File]::WriteAllText($path, ($obj | ConvertTo-Json -Depth 6), $Utf8) }
    function Read-Json($path) { if (Test-Path $path) { try { return [IO.File]::ReadAllText($path) | ConvertFrom-Json } catch { } }; return $null }

    # git's blob id: sha1("blob <size>\0" + bytes). GitHub's tree API lists it per file.
    function Get-BlobSha([byte[]]$bytes) {
        $head = [Text.Encoding]::ASCII.GetBytes("blob $($bytes.Length)`0")
        $all = New-Object byte[] ($head.Length + $bytes.Length)
        [Buffer]::BlockCopy($head, 0, $all, 0, $head.Length)
        [Buffer]::BlockCopy($bytes, 0, $all, $head.Length, $bytes.Length)
        $sha1 = [Security.Cryptography.SHA1]::Create()
        return (($sha1.ComputeHash($all) | ForEach-Object { $_.ToString('x2') }) -join '')
    }
    # Same formula as build_of() in updater.rs: sha256 of sorted "path:sha" lines.
    function Get-Build($entries) {
        $lines = [string[]]@($entries | ForEach-Object { "$($_.path):$($_.sha)" })
        [Array]::Sort($lines, [StringComparer]::Ordinal)
        $sha = [Security.Cryptography.SHA256]::Create()
        $hash = $sha.ComputeHash($Utf8.GetBytes(($lines -join "`n")))
        return ((($hash | ForEach-Object { $_.ToString('x2') }) -join '')).Substring(0, 12)
    }

    function Stop-Brain {
        $procs = @(Get-CimInstance Win32_Process -Filter "Name='brain-system.exe'" -ErrorAction SilentlyContinue)
        if (-not $procs.Count) { return }
        foreach ($p in $procs) {
            Say "Closing the running brain ($($p.ExecutablePath))..." DarkGray
            & taskkill.exe /PID $p.ProcessId /T /F 2>&1 | Out-Null
        }
        Start-Sleep -Seconds 1
    }

    # Start menu + desktop .lnk paths (skips folders Windows can't resolve).
    function Get-ShortcutPaths {
        foreach ($pair in @(@('StartMenu', 'Programs\Brain.Skills.lnk'), @('Desktop', 'Brain.Skills.lnk'))) {
            $base = [Environment]::GetFolderPath($pair[0])
            if ($base) { Join-Path $base $pair[1] }
        }
    }

    function Set-Shortcuts([switch]$Create) {
        foreach ($p in (Get-ShortcutPaths)) {
            if (-not $Create -and -not (Test-Path $p)) { continue }   # a deleted desktop icon stays deleted
            if ($Create -and $env:BRAINSKILLS_NO_DESKTOP -and $p -like '*Desktop*') { continue }
            try {
                $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($p)
                $lnk.TargetPath = $Exe
                $lnk.WorkingDirectory = $BinDir
                $lnk.IconLocation = "$(Join-Path $AppDir 'assets\brain.ico'),0"
                $lnk.Description = 'Brain.Skills - Global Brain neural view, wiki and memory'
                $lnk.Save()
            } catch { }
        }
    }

    function Set-UserPath([switch]$Remove) {
        $cur = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @(($cur -split ';') | Where-Object { $_ -and ($_.TrimEnd('\') -ne $BinDir.TrimEnd('\')) })
        if (-not $Remove) { $parts += $BinDir }
        $new = $parts -join ';'
        if ($new -ne $cur) { [Environment]::SetEnvironmentVariable('Path', $new, 'User') }
    }

    function Register-App($version) {
        try {
            New-Item -Path $UninstallKey -Force | Out-Null
            $size = 0
            try { $size = [int]((Get-ChildItem -LiteralPath $AppDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1KB) } catch { }
            $values = [ordered]@{
                DisplayName = 'Brain.Skills'; DisplayVersion = "$version"; Publisher = 'Global Warning Networks'
                DisplayIcon = "$(Join-Path $AppDir 'assets\brain.ico')"; InstallLocation = $Root
                UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $Root 'update.ps1')`" -Uninstall"
                URLInfoAbout = "https://github.com/$Repo"
            }
            foreach ($k in $values.Keys) { Set-ItemProperty -Path $UninstallKey -Name $k -Value $values[$k] }
            foreach ($k in 'NoModify', 'NoRepair') { Set-ItemProperty -Path $UninstallKey -Name $k -Value 1 -Type DWord }
            if ($size) { Set-ItemProperty -Path $UninstallKey -Name EstimatedSize -Value $size -Type DWord }
        } catch { }
    }

    # ---------------------------------------------------------------- uninstall
    if ($Uninstall) {
        Write-Host ''
        Say 'Uninstalling Brain.Skills' Cyan
        $answer = Read-Host '  Remove the Brain.Skills app? Your memory vault and skills are kept. [y/N]'
        if ($answer -notmatch '^(y|yes)$') { Say 'Nothing removed.'; return }
        Stop-Brain
        if (Test-Path $Exe) { & $Exe --uninstall-startup 2>&1 | Out-Null }
        if (Get-Command claude -ErrorAction SilentlyContinue) {
            & cmd /c "claude plugin uninstall global-brain@global-brain >nul 2>&1"
            & cmd /c "claude plugin marketplace remove global-brain >nul 2>&1"
        }
        foreach ($p in (Get-ShortcutPaths)) {
            Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue
        }
        Set-UserPath -Remove
        Remove-Item -Path $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $AppDir -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $StateFile -Force -ErrorAction SilentlyContinue
        $cfg = Read-Json $ConfigFile
        if ($cfg) { Say "Kept your memory vault: $($cfg.vault_dir)" Green }
        Say 'Brain.Skills was removed.' Green
        # update.ps1 removes itself last (PowerShell has already read it).
        Remove-Item -LiteralPath (Join-Path $Root 'update.ps1') -Force -ErrorAction SilentlyContinue
        return
    }

    Write-Host ''
    Say $(if ($Updating) { 'Brain.Skills updater' } else { 'Brain.Skills installer' }) Cyan
    Say "Into: $Root"
    Write-Host ''

    # ---------------------------------------------------------------- latest commit + its file list
    try {
        $commit = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/commits/$Branch" -Headers $Headers -UseBasicParsing -TimeoutSec 30
        $sha = $commit.sha
        $tree = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/git/trees/$($sha)?recursive=1" -Headers $Headers -UseBasicParsing -TimeoutSec 60
    } catch {
        Say "Could not reach GitHub: $($_.Exception.Message)" Yellow
        if ($Updating) { Say 'Starting the version you have.' DarkGray; if (Test-Path $Exe) { Start-Process -FilePath $Exe -WorkingDirectory $BinDir } }
        return
    }
    $Raw = "https://raw.githubusercontent.com/$Repo/$sha"
    $files = @($tree.tree | Where-Object { $_.type -eq 'blob' -and $Managed -contains $_.path })
    if ($files.Count -ne $Managed.Count) { Say "The repository is missing some app files ($($files.Count)/$($Managed.Count)). Try again later." Yellow; return }
    $version = try { (Invoke-RestMethod -Uri "$Raw/VERSION" -Headers $Headers -UseBasicParsing -TimeoutSec 30).ToString().Trim() } catch { '0.0.0' }
    $build = Get-Build $files
    $label = "v$version (build $build)"

    $old = Read-Json $StateFile
    $oldHashes = @{}
    if ($old -and $old.files) { foreach ($f in $old.files) { $oldHashes[$f.p] = $f.h } }
    $upToDate = $old -and $old.build -eq $build -and (Test-Path $Exe)
    if ($Updating -and $upToDate) {
        Say "Brain.Skills $label is up to date." Green
        if (-not (Get-Process brain-system -ErrorAction SilentlyContinue)) { Start-Process -FilePath $Exe -WorkingDirectory $BinDir }
        return
    }

    # ---------------------------------------------------------------- download what changed
    $stage = Join-Path $Root '.staging-app'
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    $todo = @($files | Where-Object { $oldHashes[$_.path] -ne $_.sha -or -not (Test-Path (Join-Path $AppDir $_.path)) })
    if ($todo.Count) { Say "$(if ($old) { 'Updating to' } else { 'Installing' }) Brain.Skills $label - downloading $($todo.Count) file(s)..." Cyan }
    else { Say "Brain.Skills $label - files already current, refreshing the setup..." Cyan }
    foreach ($f in $todo) {
        $dest = Join-Path $stage $f.path
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        $ok = $false
        for ($try = 1; $try -le 3 -and -not $ok; $try++) {
            try {
                Invoke-WebRequest -Uri "$Raw/$($f.path)" -Headers $Headers -UseBasicParsing -TimeoutSec 300 -OutFile $dest
                $ok = (Get-BlobSha ([IO.File]::ReadAllBytes($dest))) -eq $f.sha
            } catch { Start-Sleep -Seconds 2 }
        }
        if (-not $ok) { Say "Download of $($f.path) failed or was corrupted. Nothing was changed; try again." Yellow; Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue; return }
        Say "  $($f.path)" DarkGray
    }

    # ---------------------------------------------------------------- swap files in
    Stop-Brain
    foreach ($f in $todo) {
        $dest = Join-Path $AppDir $f.path
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        Move-Item -LiteralPath (Join-Path $stage $f.path) -Destination $dest -Force
    }
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue

    # ---------------------------------------------------------------- where the vault and skills live
    # Everything lives in the user folder. A vault found in the old dev-checkout
    # location (C:\.skills\memory, used by installs before 1.0.1) is copied in,
    # verified file by file, and the old folder is renamed so nothing keeps
    # using it.
    $cfg = Read-Json $ConfigFile
    $userVault = Join-Path $Root 'memory'
    $legacy = if ($env:BRAINSKILLS_LEGACY_VAULT) { $env:BRAINSKILLS_LEGACY_VAULT.TrimEnd('\') } else { Join-Path 'C:\' '.skills\memory' }
    $legacyInUse = $cfg -and $cfg.vault_dir -and ($cfg.vault_dir.TrimEnd('\') -ieq $legacy)
    $hasNotesIn = { param($d) (Test-Path (Join-Path $d '10-Entities')) -or (Test-Path (Join-Path $d '30-Logs')) }
    if ($legacyInUse -and (& $hasNotesIn $userVault)) {
        # Two vaults: never guess which one is current.
        Say "Two memory vaults exist: $legacy (in use) and $userVault. Keeping $legacy; merge them by hand, then delete brain.json's vault_dir line to switch." Yellow
        $legacyInUse = $false   # brain.json keeps pointing at the vault in use
    }
    if (-not $cfg -or -not $cfg.vault_dir -or -not (Test-Path $cfg.vault_dir) -or $legacyInUse) {
        if ($env:BRAINSKILLS_VAULT) { $vault = $env:BRAINSKILLS_VAULT }
        else {
            $vault = $userVault
            $hasNotes = { param($d) (Test-Path (Join-Path $d '10-Entities')) -or (Test-Path (Join-Path $d '30-Logs')) }
            if (-not (& $hasNotes $vault) -and (& $hasNotes $legacy)) {
                Say "Moving your memory vault from $legacy into $vault ..." Cyan
                Stop-Brain
                & robocopy.exe $legacy $vault /E /COPY:DAT /DCOPY:T /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
                $bad = 0
                foreach ($f in Get-ChildItem -LiteralPath $legacy -Recurse -File -Force) {
                    $t = Join-Path $vault $f.FullName.Substring($legacy.Length)
                    if (-not (Test-Path -LiteralPath $t) -or (Get-FileHash -LiteralPath $f.FullName).Hash -ne (Get-FileHash -LiteralPath $t).Hash) { $bad++ }
                }
                if ($bad) {
                    Say "$bad file(s) did not copy correctly; keeping the vault at $legacy for now. Run the setup again to retry." Yellow
                    $vault = $legacy
                } else {
                    $retired = "$legacy.moved-$(Get-Date -Format yyyyMMdd)"
                    try { Rename-Item -LiteralPath $legacy -NewName (Split-Path $retired -Leaf); Say "Vault moved. The old copy is kept as $retired" Green }
                    catch { Say "Vault copied. Could not rename $legacy (in use?); it is no longer used." Yellow }
                }
            } elseif (-not (& $hasNotes $vault)) {
                foreach ($d in '00-Inbox', '10-Entities', '20-Concepts', '30-Logs') { New-Item -ItemType Directory -Force -Path (Join-Path $vault $d) | Out-Null }
                $welcome = Join-Path $vault '20-Concepts\Global-Brain.md'
                if (-not (Test-Path $welcome)) {
                    [IO.File]::WriteAllText($welcome, "# Global Brain`n`nThis is your memory vault. Agents write session logs to 30-Logs and new findings to 00-Inbox; verified knowledge lives in 10-Entities and 20-Concepts.`n", $Utf8)
                }
                Say "Created a new memory vault: $vault" Green
            }
        }
        $skillsDir = if ($cfg -and $cfg.skills_dir) { $cfg.skills_dir } else { Join-Path $Root 'skills' }
        Write-Json $ConfigFile ([ordered]@{ vault_dir = $vault; skills_dir = $skillsDir })
    }

    # ---------------------------------------------------------------- integrate with Windows
    try { Invoke-WebRequest -Uri "$Raw/install.ps1" -Headers $Headers -UseBasicParsing -TimeoutSec 60 -OutFile (Join-Path $Root 'update.ps1') } catch { }
    Write-Json $StateFile ([ordered]@{
        version = $version; build = $build; commit = $sha; date = (Get-Date).ToString('o')
        files = @($files | ForEach-Object { [ordered]@{ p = $_.path; h = $_.sha } })
    })
    Set-Shortcuts -Create:(-not $old)
    Set-UserPath
    Register-App $version
    & $Exe --install-startup 2>&1 | Out-Null
    Say 'Starts automatically when you sign in to Windows.' DarkGray

    # Claude Code plugin: lets Claude search and write to the brain (MCP).
    if (-not $env:BRAINSKILLS_NO_CLAUDE -and (Get-Command claude -ErrorAction SilentlyContinue)) {
        Say 'Connecting Claude Code (global-brain plugin)...' Cyan
        & $Exe --install-claude-plugin 2>&1 | Out-Null
        Say 'Claude Code plugin installed. Restart Claude Code to load it.' Green
    }

    # ---------------------------------------------------------------- start it
    Start-Process -FilePath $Exe -WorkingDirectory $BinDir
    $ready = $false
    for ($i = 0; $i -lt 60 -and -not $ready; $i++) {
        Start-Sleep -Milliseconds 500
        try { $ready = (Invoke-RestMethod -Uri 'http://127.0.0.1:6789/api/status' -UseBasicParsing -TimeoutSec 2).ready } catch { }
    }
    Write-Host ''
    Say "Brain.Skills $label is $(if ($old) { 'updated' } else { 'installed' })." Green
    Say $(if ($ready) { 'The brain is running: http://127.0.0.1:6789  (tray icon: right-click for menu)' } else { 'The brain is starting; look for the tray icon.' })
    if (-not (Test-Path (Join-Path $Root 'hub\skills-installed.json'))) {
        Say 'Next: install the skills library with Skills-Installer-Hub-Setup.exe, or:' Cyan
        Say "  irm https://raw.githubusercontent.com/$Repo/main/install-skills.ps1 | iex"
    }
    return
}

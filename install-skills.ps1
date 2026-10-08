# Skills Installer Hub: installs and updates the Brain.Skills skills library
# and links it into every AI agent found on this PC (Claude Code, Codex,
# Cursor, Gemini, MiniMax, Omni...). Per user, no administrator.
#
# Install (or run Skills-Installer-Hub-Setup.exe from the GitHub releases page):
#   irm https://raw.githubusercontent.com/tattooinmtl/Brain.Skills/main/install-skills.ps1 | iex
#
# Files go to %USERPROFILE%\.brain-skills:
#   skills\    the library (one folder per skill, each with SKILL.md)
#   commands\  slash commands (e.g. /skills for Claude Code)
#   hub\       skills.exe, update-skills.ps1, skills-installed.json
# Each agent's skills folder becomes a junction to skills\, so one update
# reaches every agent. Existing skills links are re-pointed; an agent's own
# real skills folder is left alone (run with -ReplaceFolders to move it aside
# as skills.backup-<date> and link the library instead; nothing is deleted).
#
# Update: run "Skills Installer Hub" from the Start menu, the brain tray menu,
# or hub\update-skills.ps1. Only changed files are downloaded, and only files
# the hub installed are ever removed; skills you add yourself are kept.
# Uninstall: Windows Settings > Apps, or  update-skills.ps1 -Uninstall

$SkillsHubArgs = @($args)
& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    Add-Type -AssemblyName System.IO.Compression.FileSystem

    $Repo = if ($env:BRAINSKILLS_REPO) { $env:BRAINSKILLS_REPO } else { 'tattooinmtl/Brain.Skills' }
    $Branch = if ($env:BRAINSKILLS_BRANCH) { $env:BRAINSKILLS_BRANCH } else { 'main' }
    $Uninstall = $SkillsHubArgs -contains '-Uninstall'
    $Relink = $SkillsHubArgs -contains '-Relink'
    # Run as <root>\hub\update-skills.ps1 = updating that install.
    $Updating = $PSScriptRoot -and (Test-Path (Join-Path $PSScriptRoot 'skills-installed.json'))
    $Root = if ($Updating) { Split-Path $PSScriptRoot } elseif ($env:BRAINSKILLS_HOME) { $env:BRAINSKILLS_HOME } else { Join-Path $HOME '.brain-skills' }
    $HubDir = Join-Path $Root 'hub'
    $SkillsDir = Join-Path $Root 'skills'
    $StateFile = Join-Path $HubDir 'skills-installed.json'
    $ConfigFile = Join-Path $Root 'brain.json'
    $SkillsExe = Join-Path $HubDir 'bin\skills.exe'
    $UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\BrainSkillsHub'
    $startMenu = [Environment]::GetFolderPath('StartMenu')
    $Shortcut = if ($startMenu) { Join-Path $startMenu 'Programs\Skills Installer Hub.lnk' } else { $null }
    $Utf8 = New-Object System.Text.UTF8Encoding $false
    $Headers = @{ 'User-Agent' = 'SkillsInstallerHub' }

    function Say($msg, $color = 'Gray') { Write-Host "  $msg" -ForegroundColor $color }
    function Write-Json($path, $obj) { [IO.File]::WriteAllText($path, ($obj | ConvertTo-Json -Depth 6 -Compress), $Utf8) }
    function Read-Json($path) { if (Test-Path $path) { try { return [IO.File]::ReadAllText($path) | ConvertFrom-Json } catch { } }; return $null }
    # Repo path -> install path. Keep in sync with in_skills() in brain-system/src/updater.rs.
    function Get-Target($p) {
        if ($p -eq 'bin/skills.exe') { return 'hub/bin/skills.exe' }
        if ($p.StartsWith('skills/') -or $p.StartsWith('commands/')) { return $p }
        return $null
    }
    function Get-BlobSha([byte[]]$bytes) {
        $head = [Text.Encoding]::ASCII.GetBytes("blob $($bytes.Length)`0")
        $all = New-Object byte[] ($head.Length + $bytes.Length)
        [Buffer]::BlockCopy($head, 0, $all, 0, $head.Length)
        [Buffer]::BlockCopy($bytes, 0, $all, $head.Length, $bytes.Length)
        return ((([Security.Cryptography.SHA1]::Create()).ComputeHash($all) | ForEach-Object { $_.ToString('x2') }) -join '')
    }
    function Get-Build($entries) {
        $lines = [string[]]@($entries | ForEach-Object { "$($_.path):$($_.sha)" })
        [Array]::Sort($lines, [StringComparer]::Ordinal)
        $hash = ([Security.Cryptography.SHA256]::Create()).ComputeHash($Utf8.GetBytes(($lines -join "`n")))
        return ((($hash | ForEach-Object { $_.ToString('x2') }) -join '')).Substring(0, 12)
    }
    # Agent skills folders that are links into our library.
    function Get-OurLinks {
        $want = (Resolve-Path -LiteralPath $SkillsDir -ErrorAction SilentlyContinue).Path
        if (-not $want) { return @() }
        @(Get-ChildItem -LiteralPath $HOME -Directory -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name.StartsWith('.') } | ForEach-Object {
            $link = Join-Path $_.FullName 'skills'
            $item = Get-Item -LiteralPath $link -Force -ErrorAction SilentlyContinue
            if ($item -and $item.LinkType -and (@($item.Target) | Where-Object { $_ -and ($_.TrimStart('\?') -replace '^UNC\\', '\\').TrimEnd('\') -eq $want.TrimEnd('\') })) { $link }
        })
    }
    # <root>\skills.json + skills\INDEX.md: the catalog other agents (e.g. Omni's
    # skillIndex) read. Regenerated after every install/update.
    function Update-Catalog {
        if (-not (Test-Path $SkillsExe)) { return }
        $env:SKILLS_ROOT = $Root
        & $SkillsExe sync 2>&1 | Out-Null
        Remove-Item Env:SKILLS_ROOT -ErrorAction SilentlyContinue
        if (Test-Path (Join-Path $Root 'skills.json')) { Say "Catalog: $(Join-Path $Root 'skills.json')" DarkGray }
    }

    function Set-BrainSkillsDir([string]$dir) {
        $cfg = Read-Json $ConfigFile
        $vault = if ($cfg -and $cfg.vault_dir) { $cfg.vault_dir } else { $null }
        $obj = [ordered]@{}
        if ($vault) { $obj.vault_dir = $vault }
        if ($dir) { $obj.skills_dir = $dir }
        if ($obj.Count) { [IO.File]::WriteAllText($ConfigFile, ($obj | ConvertTo-Json), $Utf8) }
        # The brain reads brain.json at start: restart it if it is running.
        $brain = @(Get-CimInstance Win32_Process -Filter "Name='brain-system.exe'" -ErrorAction SilentlyContinue | Where-Object { $_.ExecutablePath -like "$Root\*" })
        foreach ($p in $brain) {
            & taskkill.exe /PID $p.ProcessId /T /F 2>&1 | Out-Null
            Start-Sleep -Milliseconds 800
            Start-Process -FilePath $p.ExecutablePath -WorkingDirectory (Split-Path $p.ExecutablePath)
            Say 'Restarted the brain so it sees the new skills.' DarkGray
        }
    }

    # ---------------------------------------------------------------- uninstall
    if ($Uninstall) {
        Write-Host ''
        Say 'Uninstalling the Skills Installer Hub' Cyan
        $answer = Read-Host '  Remove the installed skills library and its links in your agents? Skills you added yourself stay. [y/N]'
        if ($answer -notmatch '^(y|yes)$') { Say 'Nothing removed.'; return }
        foreach ($l in Get-OurLinks) { cmd /c rmdir "$l" | Out-Null; Say "Unlinked $l" DarkGray }
        $old = Read-Json $StateFile
        if ($old -and $old.files) {
            foreach ($f in $old.files) { Remove-Item -LiteralPath (Join-Path $Root $f.t) -Force -ErrorAction SilentlyContinue }
            # Drop folders the removal left empty.
            foreach ($d in @($SkillsDir, (Join-Path $Root 'commands'))) {
                if (Test-Path $d) { Get-ChildItem -LiteralPath $d -Recurse -Directory -Force | Sort-Object { $_.FullName.Length } -Descending | Where-Object { -not (Get-ChildItem -LiteralPath $_.FullName -Force) } | Remove-Item -Force -ErrorAction SilentlyContinue
                    if (-not (Get-ChildItem -LiteralPath $d -Force)) { Remove-Item -LiteralPath $d -Force } }
            }
        }
        Remove-Item -LiteralPath (Join-Path $Root 'skills.json') -Force -ErrorAction SilentlyContinue
        if ($Shortcut) { Remove-Item -LiteralPath $Shortcut -Force -ErrorAction SilentlyContinue }
        Remove-Item -Path $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
        Set-BrainSkillsDir $null
        Remove-Item -LiteralPath $HubDir -Recurse -Force -ErrorAction SilentlyContinue
        Say 'The Skills Installer Hub was removed.' Green
        return
    }

    Write-Host ''
    Say $(if ($Updating) { 'Skills Installer Hub - checking for updates' } else { 'Skills Installer Hub' }) Cyan
    Say "Into: $Root"
    Write-Host ''

    # ---------------------------------------------------------------- latest commit + its file list
    try {
        $commit = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/commits/$Branch" -Headers $Headers -UseBasicParsing -TimeoutSec 30
        $sha = $commit.sha
        $tree = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/git/trees/$($sha)?recursive=1" -Headers $Headers -UseBasicParsing -TimeoutSec 60
    } catch { Say "Could not reach GitHub: $($_.Exception.Message)" Yellow; return }
    if ($tree.truncated) { Say 'GitHub returned a partial file list; try again later.' Yellow; return }
    $Raw = "https://raw.githubusercontent.com/$Repo/$sha"
    $files = @($tree.tree | Where-Object { $_.type -eq 'blob' -and (Get-Target $_.path) } | ForEach-Object {
        [pscustomobject]@{ path = $_.path; sha = $_.sha; t = (Get-Target $_.path) }
    })
    $build = Get-Build $files
    $version = "$($commit.commit.committer.date)".Substring(0, 10)
    $skillCount = @($files | Where-Object { $_.path -like 'skills/*/SKILL.md' -or $_.path -like 'skills/*SKILL.md' }).Count
    $label = "$version (build $build)"

    $old = Read-Json $StateFile
    $oldHashes = @{}
    if ($old -and $old.files) { foreach ($f in $old.files) { $oldHashes[$f.t] = $f.h } }
    $todo = @($files | Where-Object { $oldHashes[$_.t] -ne $_.sha -or -not (Test-Path (Join-Path $Root $_.t)) })
    $newTargets = @{}; foreach ($f in $files) { $newTargets[$f.t] = $true }
    $gone = @(if ($old -and $old.files) { $old.files | Where-Object { -not $newTargets[$_.t] } })

    if (-not $todo.Count -and -not $gone.Count -and -not $Relink -and $old) {
        if (-not (Test-Path (Join-Path $Root 'skills.json'))) { Update-Catalog }
        Say "Skills $label are up to date ($skillCount skills)." Green
        return
    }
    Say "$(if ($old) { 'Updating to' } else { 'Installing' }) skills $label - $($todo.Count) file(s) to download, $($gone.Count) to remove." Cyan

    # ---------------------------------------------------------------- download into staging
    $stage = Join-Path $Root '.staging-skills'
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    $need = @{}; foreach ($f in $todo) { $need[$f.path] = $f }
    $done = @{}
    if ($todo.Count -gt 40) {
        # Many files: one zip of the whole commit is far faster than per-file requests.
        Say 'Downloading the library archive...' DarkGray
        $zip = Join-Path $stage 'repo.zip'
        try {
            Invoke-WebRequest -Uri "https://codeload.github.com/$Repo/zip/$sha" -Headers $Headers -UseBasicParsing -TimeoutSec 600 -OutFile $zip
            $archive = [IO.Compression.ZipFile]::OpenRead($zip)
            try {
                foreach ($e in $archive.Entries) {
                    $slash = $e.FullName.IndexOf('/')
                    if ($slash -lt 0) { continue }
                    $p = $e.FullName.Substring($slash + 1)
                    $f = $need[$p]
                    if (-not $f) { continue }
                    $ms = New-Object IO.MemoryStream
                    $s = $e.Open(); $s.CopyTo($ms); $s.Dispose()
                    $bytes = $ms.ToArray()
                    if ((Get-BlobSha $bytes) -ne $f.sha) { continue }   # fetched individually below
                    $dest = Join-Path $stage $f.t
                    New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
                    [IO.File]::WriteAllBytes($dest, $bytes)
                    $done[$f.t] = $true
                }
            } finally { $archive.Dispose() }
        } catch { Say "Archive download failed ($($_.Exception.Message)); fetching files one by one." Yellow }
        Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
    }
    $left = @($todo | Where-Object { -not $done[$_.t] })
    $n = 0
    foreach ($f in $left) {
        $n++
        $dest = Join-Path $stage $f.t
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        $ok = $false
        for ($try = 1; $try -le 3 -and -not $ok; $try++) {
            try {
                $url = "$Raw/" + (($f.path -split '/' | ForEach-Object { [Uri]::EscapeDataString($_) }) -join '/')
                Invoke-WebRequest -Uri $url -Headers $Headers -UseBasicParsing -TimeoutSec 120 -OutFile $dest
                $ok = (Get-BlobSha ([IO.File]::ReadAllBytes($dest))) -eq $f.sha
            } catch { Start-Sleep -Seconds 2 }
        }
        if (-not $ok) { Say "Download of $($f.path) failed. Nothing was changed; try again." Yellow; Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue; return }
        if ($left.Count -gt 5 -and $n % 25 -eq 0) { Say "  $n / $($left.Count)" DarkGray }
    }

    # ---------------------------------------------------------------- swap in
    foreach ($f in $todo) {
        $dest = Join-Path $Root $f.t
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        Move-Item -LiteralPath (Join-Path $stage $f.t) -Destination $dest -Force
    }
    foreach ($f in $gone) { Remove-Item -LiteralPath (Join-Path $Root $f.t) -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    Write-Json $StateFile ([ordered]@{
        version = $version; build = $build; commit = $sha; skills = $skillCount; date = (Get-Date).ToString('o')
        files = @($files | ForEach-Object { [ordered]@{ t = $_.t; h = $_.sha } })
    })
    try { Invoke-WebRequest -Uri "$Raw/install-skills.ps1" -Headers $Headers -UseBasicParsing -TimeoutSec 60 -OutFile (Join-Path $HubDir 'update-skills.ps1') } catch { }

    # ---------------------------------------------------------------- link agents (first install / -Relink)
    if (-not $old -or $Relink) {
        Say 'Linking the library into your AI agents...' Cyan
        $env:SKILLS_ROOT = $Root
        # --repoint moves existing skills links to this library; an agent's own
        # (real) skills folder is kept unless -ReplaceFolders is given.
        $linkArgs = @('install', '--auto', '--repoint', '--skills-source', $SkillsDir)
        if ($SkillsHubArgs -contains '-ReplaceFolders') { $linkArgs += '--force' }
        & $SkillsExe @linkArgs
        Remove-Item Env:SKILLS_ROOT -ErrorAction SilentlyContinue
    }
    Update-Catalog
    $links = @(Get-OurLinks)

    # ---------------------------------------------------------------- Windows integration
    if ($Shortcut) { try {
        $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($Shortcut)
        $lnk.TargetPath = 'powershell.exe'
        $lnk.Arguments = "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$(Join-Path $HubDir 'update-skills.ps1')`""
        $lnk.WorkingDirectory = $HubDir
        $lnk.Description = 'Update the Brain.Skills skills library'
        $lnk.Save()
    } catch { } }
    try {
        New-Item -Path $UninstallKey -Force | Out-Null
        $values = [ordered]@{
            DisplayName = 'Skills Installer Hub (Brain.Skills)'; DisplayVersion = $version; Publisher = 'Global Warning Networks'
            InstallLocation = $Root; URLInfoAbout = "https://github.com/$Repo"
            UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $HubDir 'update-skills.ps1')`" -Uninstall"
        }
        foreach ($k in $values.Keys) { Set-ItemProperty -Path $UninstallKey -Name $k -Value $values[$k] }
        foreach ($k in 'NoModify', 'NoRepair') { Set-ItemProperty -Path $UninstallKey -Name $k -Value 1 -Type DWord }
    } catch { }
    $cfg = Read-Json $ConfigFile
    if (-not $cfg -or $cfg.skills_dir -ne $SkillsDir) { Set-BrainSkillsDir $SkillsDir }

    Write-Host ''
    Say "Skills $label $(if ($old) { 'updated' } else { 'installed' }): $skillCount skills in $SkillsDir" Green
    if ($links.Count) { Say "Linked into $($links.Count) agent(s): $(($links | ForEach-Object { Split-Path (Split-Path $_) -Leaf }) -join ', ')" Green }
    else { Say 'No agent folders linked yet. Run "update-skills.ps1 -Relink" after installing an agent.' Yellow }
    return
}

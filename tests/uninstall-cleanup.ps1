param(
    [string]$MakeNsis = "$env:LOCALAPPDATA/tauri/NSIS/makensis.exe"
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath "$PSScriptRoot/..").Path
$fixture = Join-Path $root "src-tauri/target/uninstall-cleanup-$([guid]::NewGuid())"
$profileDir = Join-Path $fixture 'profile'
New-Item -ItemType Directory -Path $profileDir -Force | Out-Null

# Run the real hook with its profile redirected into this disposable fixture.
$hook = [IO.File]::ReadAllText((Join-Path $root 'src-tauri/windows/hooks.nsh'))
$hook = $hook.Replace('$PROFILE', '$TestProfile')
if ($hook.Contains('$PROFILE')) { throw 'Test hook still references the real user profile.' }
[IO.File]::WriteAllText((Join-Path $fixture 'hooks.nsh'), $hook)
$script = @'
Unicode true
Name "Database cleanup test"
OutFile "cleanup-test.exe"
SilentInstall silent
RequestExecutionLevel user
!include LogicLib.nsh
!include FileFunc.nsh
!include hooks.nsh
Var TestProfile
Var DeleteAppDataCheckboxState
Var UpdateMode
Section
  StrCpy $TestProfile "$EXEDIR\profile"
  ${GetOptions} $CMDLINE "/DELETE=" $DeleteAppDataCheckboxState
  ${GetOptions} $CMDLINE "/UPDATE=" $UpdateMode
  !insertmacro NSIS_HOOK_POSTUNINSTALL
SectionEnd
'@
$scriptPath = Join-Path $fixture 'test.nsi'
[IO.File]::WriteAllText($scriptPath, $script)
& $MakeNsis /V2 $scriptPath
if ($LASTEXITCODE -ne 0) { throw 'NSIS cleanup test failed to compile.' }

$files = 'agent-skill-installer.db', 'agent-skill-installer.db-wal', 'agent-skill-installer.db-shm'
[IO.File]::WriteAllText((Join-Path $profileDir 'keep.txt'), 'unrelated file')
foreach ($case in @(
    @{ Name = 'unchecked'; Delete = 0; Update = 0; Keep = $true },
    @{ Name = 'upgrade'; Delete = 1; Update = 1; Keep = $true },
    @{ Name = 'locked database'; Delete = 1; Update = 0; Keep = $true; Locked = $true },
    @{ Name = 'selected'; Delete = 1; Update = 0; Keep = $false },
    @{ Name = 'already removed'; Delete = 1; Update = 0; Keep = $false; Missing = $true }
)) {
    if (-not $case.Missing) {
        foreach ($name in $files) { [IO.File]::WriteAllText((Join-Path $profileDir $name), 'test data') }
    }
    $locked = $null
    try {
        if ($case.Locked) {
            $locked = [IO.File]::Open((Join-Path $profileDir $files[0]), 'Open', 'ReadWrite', 'None')
        }
        $process = Start-Process -FilePath (Join-Path $fixture 'cleanup-test.exe') -WindowStyle Hidden -PassThru -ArgumentList "/DELETE=$($case.Delete)", "/UPDATE=$($case.Update)"
        if (-not $process.WaitForExit(10000)) {
            Stop-Process -Id $process.Id
            throw "Cleanup timed out: $($case.Name)"
        }
        if ($process.ExitCode -ne 0) { throw "Cleanup failed: $($case.Name)" }
        $process.Dispose()
        foreach ($name in $files) {
            if ((Test-Path -LiteralPath (Join-Path $profileDir $name)) -ne $case.Keep) {
                throw "Unexpected file state: $($case.Name), $name"
            }
        }
        if ([IO.File]::ReadAllText((Join-Path $profileDir 'keep.txt')) -ne 'unrelated file') {
            throw 'Unrelated file was changed.'
        }
    } finally {
        if ($locked) { $locked.Dispose() }
    }
}
Write-Output 'Passed: unchecked and upgrade preserve data; selected removes only database files; locked and missing files are handled.'

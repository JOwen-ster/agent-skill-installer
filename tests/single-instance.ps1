param(
    [string]$AppPath = "$PSScriptRoot/../src-tauri/target/debug/agent-skill-installer.exe"
)

$ErrorActionPreference = 'Stop'
$executable = (Resolve-Path -LiteralPath $AppPath).Path
if (Get-Process -Name 'agent-skill-installer' -ErrorAction SilentlyContinue) {
    throw 'Close the running installer before testing.'
}

$started = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()
try {
    # Launch together to cover the race before the first window exists.
    foreach ($attempt in 1..6) {
        $started.Add((Start-Process -FilePath $executable -WindowStyle Hidden -PassThru))
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 100
        $running = @($started | Where-Object { -not $_.HasExited })
    } while ($running.Count -ne 1 -and [DateTime]::UtcNow -lt $deadline)

    if ($running.Count -ne 1) { throw "Expected one instance; found $($running.Count)." }
    foreach ($process in $started) {
        if ($process.HasExited -and $process.ExitCode -ne 0) {
            throw "Duplicate launch failed with exit code $($process.ExitCode)."
        }
    }
    $survivor = $running[0]
    if ($survivor.WaitForExit(2000)) {
        throw 'The surviving instance exited during startup.'
    }

    $duplicate = Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
    $started.Add($duplicate)
    if (-not $duplicate.WaitForExit(10000) -or $duplicate.ExitCode -ne 0 -or $survivor.HasExited) {
        throw 'A later launch did not hand off to the running instance.'
    }

    Stop-Process -Id $survivor.Id
    $survivor.WaitForExit()
    $reopened = Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
    $started.Add($reopened)
    if ($reopened.WaitForExit(2000)) {
        throw 'The installer could not reopen after exiting.'
    }
    Write-Output 'Passed: simultaneous launches and a later launch left one instance, and reopening succeeded.'
} finally {
    foreach ($process in $started) {
        if (-not $process.HasExited) { Stop-Process -Id $process.Id }
        $process.Dispose()
    }
}

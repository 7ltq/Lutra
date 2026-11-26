param(
    [Parameter(Mandatory = $true)]
    [int]$RootProcessId,
    [ValidateRange(2, 3600)]
    [int]$Seconds = 30,
    [ValidateRange(1, 60)]
    [int]$Interval = 2
)

$ErrorActionPreference = 'Stop'
$root = Get-Process -Id $RootProcessId
$started = $root.StartTime
$watch = [System.Diagnostics.Stopwatch]::StartNew()
$previous = @{}
$logicalProcessors = [Environment]::ProcessorCount

while ($watch.Elapsed.TotalSeconds -lt $Seconds) {
    $currentRoot = Get-Process -Id $RootProcessId -ErrorAction SilentlyContinue
    if ($null -eq $currentRoot -or $currentRoot.StartTime -ne $started) { break }
    $snapshot = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($RootProcessId)
    do {
        $added = $false
        foreach ($entry in $snapshot) {
            if ($ids.Contains([int]$entry.ParentProcessId) -and $ids.Add([int]$entry.ProcessId)) {
                $added = $true
            }
        }
    } while ($added)

    $privateBytes = 0L
    $workingBytes = 0L
    $cpuDelta = 0.0
    $count = 0
    $now = $watch.Elapsed.TotalSeconds
    $next = @{}
    foreach ($processId in $ids) {
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if ($null -eq $process) { continue }
        try {
            $identity = "$processId/$($process.StartTime.Ticks)"
            $cpu = $process.TotalProcessorTime.TotalSeconds
            $privateBytes += $process.PrivateMemorySize64
            $workingBytes += $process.WorkingSet64
            $count++
            if ($previous.ContainsKey($identity)) {
                $cpuDelta += [Math]::Max(0, $cpu - $previous[$identity])
            }
            $next[$identity] = $cpu
        } catch {
            continue
        }
    }
    $cpuPercent = $null
    if ($null -ne $lastSample -and $now -gt $lastSample) {
        $cpuPercent = [Math]::Round(100 * $cpuDelta / (($now - $lastSample) * $logicalProcessors), 2)
    }
    [pscustomobject]@{
        ElapsedSeconds = [Math]::Round($now, 2)
        Processes = $count
        PrivateMiB = [Math]::Round($privateBytes / 1MB, 2)
        WorkingSetMiB = [Math]::Round($workingBytes / 1MB, 2)
        CPUPercent = $cpuPercent
    }
    $previous = $next
    $lastSample = $now
    Start-Sleep -Seconds $Interval
}

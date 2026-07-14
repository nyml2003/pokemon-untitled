[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$manifest = Join-Path $workspaceRoot "Cargo.toml"
$report = Join-Path ([System.IO.Path]::GetTempPath()) "ramus-pure-coverage-$PID.lcov"

try {
    cargo +nightly llvm-cov `
        --workspace `
        --all-targets `
        --locked `
        --manifest-path $manifest `
        --branch `
        --ignore-filename-regex 'boundary[\\/]|[\\/]tests[\\/]' `
        --lcov `
        --output-path $report

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $files = @()
    $current = $null
    foreach ($line in Get-Content -LiteralPath $report) {
        if ($line.StartsWith("SF:")) {
            $current = [ordered]@{
                filename = $line.Substring(3)
                lineMisses = 0
                branchMisses = 0
                functionsFound = 0
                functionsHit = 0
            }
        }
        elseif ($null -ne $current -and $line -match '^DA:\d+,0$') {
            $current.lineMisses += 1
        }
        elseif ($null -ne $current -and $line -match '^BRDA:.*,(0|-)$') {
            $current.branchMisses += 1
        }
        elseif ($null -ne $current -and $line -match '^FNF:(\d+)$') {
            $current.functionsFound = [int]$Matches[1]
        }
        elseif ($null -ne $current -and $line -match '^FNH:(\d+)$') {
            $current.functionsHit = [int]$Matches[1]
        }
        elseif ($line -eq "end_of_record" -and $null -ne $current) {
            $files += [pscustomobject]$current
            $current = $null
        }
    }

    $production = @($files | Where-Object {
        $_.filename -match '[\\/]src[\\/]' -and
        $_.filename -notmatch '[\\/]boundary[\\/]' -and
        $_.filename -notmatch '[\\/]tests[\\/]'
    })
    $incomplete = @($production | Where-Object {
        $_.lineMisses -ne 0 -or
        $_.branchMisses -ne 0 -or
        $_.functionsHit -ne $_.functionsFound
    })

    if ($incomplete.Count -gt 0) {
        foreach ($file in $incomplete) {
            Write-Error ("{0}: missed lines {1}, missed branches {2}, functions {3}/{4}" -f `
                $file.filename, `
                $file.lineMisses, `
                $file.branchMisses, `
                $file.functionsHit, `
                $file.functionsFound)
        }
        exit 1
    }

    $functionCount = ($production | Measure-Object -Property functionsFound -Sum).Sum
    Write-Output ("Ramus pure coverage passed: {0} files, no missed source lines or branches, functions {1}/{1}" -f `
        $production.Count, `
        $functionCount)
}
finally {
    if (Test-Path -LiteralPath $report) {
        Remove-Item -LiteralPath $report -Force
    }
}

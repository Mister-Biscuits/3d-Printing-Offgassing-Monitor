# Logs the board's tagged UART stream (H2,ts,val / ETH,ts,val / CO2,ts,val / TVOC,ts,val)
# into four separate per-signal .csv files.
# Usage: .\tools\capture_serial.ps1 [-Port COM7] [-Baud 115200] [-OutDir logs\run1]
param(
    [string]$Port = "COM7",
    [int]$Baud = 115200,
    [string]$OutDir = "logs\run_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
)

if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

$signals = @{
    "H2"   = @{ File = "h2.csv";      Header = "timestamp_s,h2_raw" }
    "ETH"  = @{ File = "ethanol.csv"; Header = "timestamp_s,ethanol_raw" }
    "CO2"  = @{ File = "co2.csv";     Header = "timestamp_s,co2_ppm" }
    "TVOC" = @{ File = "tvoc.csv";    Header = "timestamp_s,voc_ppb" }
}

$writers = @{}
foreach ($tag in $signals.Keys) {
    $path = Join-Path $OutDir $signals[$tag].File
    $w = New-Object System.IO.StreamWriter($path, $false)
    $w.AutoFlush = $true
    $w.WriteLine($signals[$tag].Header)
    $writers[$tag] = $w
}

$serialPort = New-Object System.IO.Ports.SerialPort $Port, $Baud, ([System.IO.Ports.Parity]::None), 8, ([System.IO.Ports.StopBits]::One)
$serialPort.ReadTimeout = 500
$serialPort.Open()

Write-Host "Logging $Port ($Baud baud) into $OutDir\{h2,ethanol,co2,tvoc}.csv -- Ctrl+C to stop"

try {
    while ($true) {
        try {
            $line = $serialPort.ReadLine().Trim()
        } catch [System.TimeoutException] {
            continue
        }

        $parts = $line.Split(",")
        if ($parts.Length -eq 3 -and $writers.ContainsKey($parts[0])) {
            $writers[$parts[0]].WriteLine("$($parts[1]),$($parts[2])")
        } else {
            # Not a tagged data row (startup/status message) -- just show it.
            Write-Host $line
        }
    }
} finally {
    foreach ($w in $writers.Values) { $w.Close() }
    $serialPort.Close()
    Write-Host "Closed $Port, saved to $OutDir"
}

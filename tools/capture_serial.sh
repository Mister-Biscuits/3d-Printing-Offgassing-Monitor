#!/usr/bin/env bash
# Linux counterpart to capture_serial.ps1.
# Demuxes the board's tagged UART stream (H2,ts,val / ETH,ts,val / CO2,ts,val / TVOC,ts,val)
# into four separate per-signal .csv files. No dependencies beyond coreutils/stty.
#
# Usage: ./tools/capture_serial.sh [/dev/ttyUSB0] [115200] [logs/run1]
set -euo pipefail

PORT="${1:-/dev/ttyUSB0}"
BAUD="${2:-115200}"
OUTDIR="${3:-logs/run_$(date +%Y%m%d_%H%M%S)}"

mkdir -p "$OUTDIR"

stty -F "$PORT" "$BAUD" cs8 -cstopb -parenb raw -echo -echoe -echok

echo "timestamp_s,h2_raw" > "$OUTDIR/h2.csv"
echo "timestamp_s,ethanol_raw" > "$OUTDIR/ethanol.csv"
echo "timestamp_s,co2_ppm" > "$OUTDIR/co2.csv"
echo "timestamp_s,voc_ppb" > "$OUTDIR/tvoc.csv"

echo "Logging $PORT ($BAUD baud) into $OUTDIR/{h2,ethanol,co2,tvoc}.csv -- Ctrl+C to stop"

while IFS= read -r line; do
    line="${line%$'\r'}"
    IFS=',' read -r tag ts val <<< "$line"
    case "$tag" in
        H2)   echo "$ts,$val" >> "$OUTDIR/h2.csv" ;;
        ETH)  echo "$ts,$val" >> "$OUTDIR/ethanol.csv" ;;
        CO2)  echo "$ts,$val" >> "$OUTDIR/co2.csv" ;;
        TVOC) echo "$ts,$val" >> "$OUTDIR/tvoc.csv" ;;
        *)    echo "$line" ;; # not a tagged data row (startup/status message) -- just show it
    esac
done < "$PORT"

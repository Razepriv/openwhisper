# Copy the latest Tauri-bundled installers into landing/assets/downloads/
# so the landing page can serve them directly (no GitHub redirect).
#
# Usage (from anywhere):
#   powershell -File landing/sync-installers.ps1
#
# Expects `bun run tauri build` (without --no-bundle) to have produced:
#   src-tauri/target/release/bundle/msi/*.msi
#   src-tauri/target/release/bundle/nsis/*.exe
#
# Renames everything to `OpenVoice_0.2.0_*` so the URLs match what the
# landing's HTML references. If the productName/version drift later,
# update the $expected map.

$ErrorActionPreference = "Stop"

$root        = Split-Path -Parent $PSCommandPath
$bundleDir   = Join-Path $root "..\src-tauri\target\release\bundle"
$downloadDir = Join-Path $root "assets\downloads"

if (-not (Test-Path $bundleDir)) {
    Write-Host "No bundle dir at $bundleDir — run `bun run tauri build` first." -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $downloadDir)) {
    New-Item -ItemType Directory -Path $downloadDir | Out-Null
}

# What the landing expects to find under /assets/downloads/.
# Sources are wildcards because Tauri may emit slightly different names
# across versions; the first matching file wins.
$expected = @(
    @{ Pattern = Join-Path $bundleDir "msi\*.msi";              Dest = "OpenVoice_0.2.0_x64_en-US.msi" },
    @{ Pattern = Join-Path $bundleDir "nsis\*x64-setup.exe";    Dest = "OpenVoice_0.2.0_x64-setup.exe" },
    @{ Pattern = Join-Path $bundleDir "nsis\*arm64-setup.exe";  Dest = "OpenVoice_0.2.0_arm64-setup.exe" }
)

$copied = 0
foreach ($e in $expected) {
    $src = Get-ChildItem -Path $e.Pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $src) {
        Write-Host ("  skip  {0}  (no match for {1})" -f $e.Dest, $e.Pattern) -ForegroundColor DarkYellow
        continue
    }
    $dest = Join-Path $downloadDir $e.Dest
    Copy-Item $src.FullName $dest -Force
    $kb = [int]((Get-Item $dest).Length / 1KB)
    Write-Host ("  copy  {0}  ({1:N0} KB from {2})" -f $e.Dest, $kb, $src.Name) -ForegroundColor Green
    $copied++
}

Write-Host ""
if ($copied -eq 0) {
    Write-Host "Nothing copied. The next `bun run tauri build` (without --no-bundle) will produce these." -ForegroundColor Yellow
} else {
    Write-Host "$copied file(s) copied to $downloadDir" -ForegroundColor Green
    Write-Host "Run ``bunx vercel --prod`` from landing/ to ship them." -ForegroundColor Gray
}

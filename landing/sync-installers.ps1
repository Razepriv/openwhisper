# Sync OpenVoice installers into landing/assets/downloads/.
#
# Two sources:
#
# 1. Tauri bundle output (Windows: produced by `bun run tauri build`):
#      src-tauri/target/release/bundle/msi/*.msi
#      src-tauri/target/release/bundle/nsis/*.exe
#
# 2. Manually-downloaded CI artifacts (Mac / Linux — we can't build
#    those from a Windows host). After triggering the GitHub Actions
#    release workflow, download the artifacts from the draft release
#    into ./mac-linux-staging/ and re-run this script — it'll pick
#    them up.
#
# Usage:
#   powershell -File landing/sync-installers.ps1
#
# Filename normalization: everything copied here is renamed to the
# `OpenVoice_0.2.0_*` form the landing's HTML references, so URLs stay
# stable across builds.

$ErrorActionPreference = "Stop"

$root         = Split-Path -Parent $PSCommandPath
$bundleDir    = Join-Path $root "..\src-tauri\target\release\bundle"
$downloadDir  = Join-Path $root "assets\downloads"
$stagingDir   = Join-Path $root "mac-linux-staging"

if (-not (Test-Path $downloadDir)) {
    New-Item -ItemType Directory -Path $downloadDir | Out-Null
}

# Windows — produced locally by tauri build
$winPatterns = @(
    @{ Pattern = Join-Path $bundleDir "msi\*.msi";              Dest = "OpenVoice_0.2.0_x64_en-US.msi" },
    @{ Pattern = Join-Path $bundleDir "nsis\*x64-setup.exe";    Dest = "OpenVoice_0.2.0_x64-setup.exe" },
    @{ Pattern = Join-Path $bundleDir "nsis\*arm64-setup.exe";  Dest = "OpenVoice_0.2.0_arm64-setup.exe" }
)

# Mac/Linux — dropped into mac-linux-staging/ by hand after a CI release.
# Just rename them to the canonical paths the landing HTML references.
$ciPatterns = @(
    @{ Pattern = Join-Path $stagingDir "*aarch64.dmg";          Dest = "OpenVoice_0.2.0_aarch64.dmg" },
    @{ Pattern = Join-Path $stagingDir "*x64.dmg";              Dest = "OpenVoice_0.2.0_x64.dmg" },
    @{ Pattern = Join-Path $stagingDir "*amd64.deb";            Dest = "OpenVoice_0.2.0_amd64.deb" },
    @{ Pattern = Join-Path $stagingDir "*x86_64.rpm";           Dest = "OpenVoice-0.2.0-1.x86_64.rpm" },
    @{ Pattern = Join-Path $stagingDir "*amd64.AppImage";       Dest = "OpenVoice_0.2.0_amd64.AppImage" }
)

$copied = 0
$missing = @()

foreach ($e in ($winPatterns + $ciPatterns)) {
    $src = Get-ChildItem -Path $e.Pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $src) {
        $missing += $e.Dest
        continue
    }
    $dest = Join-Path $downloadDir $e.Dest
    Copy-Item $src.FullName $dest -Force
    $mb = [math]::Round((Get-Item $dest).Length / 1MB, 1)
    Write-Host ("  copy  {0,-44}  ({1,5:N1} MB)" -f $e.Dest, $mb) -ForegroundColor Green
    $copied++
}

Write-Host ""
Write-Host "$copied file(s) copied to assets/downloads/." -ForegroundColor Cyan
if ($missing.Count -gt 0) {
    Write-Host ""
    Write-Host "Not yet present (will land here once their build runs):" -ForegroundColor DarkYellow
    foreach ($m in $missing) { Write-Host "  - $m" -ForegroundColor DarkYellow }
    Write-Host ""
    Write-Host "Windows builds: run `bun run tauri build` from D:\openwhisper\src" -ForegroundColor Gray
    Write-Host "Mac/Linux: trigger the GitHub Actions release workflow, then drop the" -ForegroundColor Gray
    Write-Host "  artifacts into landing/mac-linux-staging/ and re-run this script." -ForegroundColor Gray
    Write-Host "  Quick trigger via gh CLI:" -ForegroundColor Gray
    Write-Host "    gh workflow run release.yml --repo Razepriv/openwhisper" -ForegroundColor Gray
}
if ($copied -gt 0) {
    Write-Host ""
    Write-Host "Deploy: cd landing && bunx vercel --prod" -ForegroundColor Gray
}

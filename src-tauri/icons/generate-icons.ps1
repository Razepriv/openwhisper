# OpenVoice icon generator — soundwave mark
#
# Renders the brand mark (violet rounded square + 3 white audio-equalizer
# bars) at every size Tauri 2 needs, then packs the Windows .ico from the
# rendered PNGs. Produces:
#
#   src-tauri/icons/
#     32x32.png       — tray, taskbar
#     64x64.png       — kept for compat (Tauri linux helper)
#     128x128.png     — onboarding splash, dock
#     128x128@2x.png  — 256x256 retina
#     icon.png        — 512x512 master
#     icon.ico        — multi-resolution Windows icon (16/32/48/64/128/256)
#
# Drawing is done with System.Drawing.Graphics so we don't depend on
# ImageMagick or an SVG rasteriser. The geometry matches the inline SVG
# used in HandyTextLogo.tsx, the landing favicon, and the masthead mark.

Add-Type -AssemblyName System.Drawing

function New-SoundwaveIcon {
    param(
        [int]$Size,
        [string]$OutPath
    )

    $bmp = New-Object System.Drawing.Bitmap $Size, $Size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode  = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.Clear([System.Drawing.Color]::Transparent)

    # Violet background, rounded — viewBox 32x32 → corner radius 8.
    # Scale to the requested size proportionally.
    $cornerRadius = [single]($Size * (8.0 / 32.0))
    $bg = New-Object System.Drawing.Drawing2D.GraphicsPath
    $bg.AddArc(0.0,                       0.0,                       $cornerRadius * 2, $cornerRadius * 2, 180.0, 90.0)
    $bg.AddArc($Size - $cornerRadius * 2, 0.0,                       $cornerRadius * 2, $cornerRadius * 2, 270.0, 90.0)
    $bg.AddArc($Size - $cornerRadius * 2, $Size - $cornerRadius * 2, $cornerRadius * 2, $cornerRadius * 2,   0.0, 90.0)
    $bg.AddArc(0.0,                       $Size - $cornerRadius * 2, $cornerRadius * 2, $cornerRadius * 2,  90.0, 90.0)
    $bg.CloseFigure()
    $violet = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 124, 58, 237))
    $g.FillPath($violet, $bg)
    $violet.Dispose()
    $bg.Dispose()

    # Three white bars — equalizer pattern. Coordinates given in the 32x32
    # source viewBox, scaled to the output size.
    $bars = @(
        @{ X = 9.0;    Y = 13.0; W = 3.0; H = 6.0  },
        @{ X = 14.5;   Y = 9.0;  W = 3.0; H = 14.0 },
        @{ X = 20.0;   Y = 11.0; W = 3.0; H = 10.0 }
    )

    $scale = [single]($Size / 32.0)
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)

    foreach ($bar in $bars) {
        $x = [single]($bar.X * $scale)
        $y = [single]($bar.Y * $scale)
        $w = [single]($bar.W * $scale)
        $h = [single]($bar.H * $scale)
        $r = [single](1.5 * $scale)
        $r2 = $r * 2.0

        # Clamp radius so it doesn't exceed half the bar's smaller dimension —
        # otherwise small sizes draw mangled corners.
        $maxR = [Math]::Min($w, $h) / 2.0
        if ($r2 -gt $maxR * 2.0) { $r2 = $maxR * 2.0 }

        $bp = New-Object System.Drawing.Drawing2D.GraphicsPath
        $bp.AddArc($x,             $y,             $r2, $r2, 180.0, 90.0)
        $bp.AddArc($x + $w - $r2,  $y,             $r2, $r2, 270.0, 90.0)
        $bp.AddArc($x + $w - $r2,  $y + $h - $r2,  $r2, $r2,   0.0, 90.0)
        $bp.AddArc($x,             $y + $h - $r2,  $r2, $r2,  90.0, 90.0)
        $bp.CloseFigure()
        $g.FillPath($white, $bp)
        $bp.Dispose()
    }

    $white.Dispose()
    $g.Dispose()
    $bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Write-Ico {
    # Build a multi-resolution Windows .ico from a set of PNG paths.
    # PNG-in-ICO is supported on all modern Windows versions (Vista+).
    param(
        [string[]]$PngPaths,
        [string]$OutPath
    )

    $images = foreach ($p in $PngPaths) {
        $bytes = [System.IO.File]::ReadAllBytes($p)
        $bmp = [System.Drawing.Image]::FromFile($p)
        $w = $bmp.Width
        $h = $bmp.Height
        $bmp.Dispose()
        [pscustomobject]@{ Bytes = $bytes; Width = $w; Height = $h }
    }

    $count = $images.Count
    $headerSize = 6
    $entrySize  = 16
    $dataStart  = $headerSize + $entrySize * $count

    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $ms

    # ICONDIR
    $bw.Write([UInt16]0)          # reserved
    $bw.Write([UInt16]1)          # type = icon
    $bw.Write([UInt16]$count)     # image count

    # Build entries; offsets accumulate after all entries.
    $offset = $dataStart
    foreach ($img in $images) {
        $w8 = if ($img.Width  -ge 256) { 0 } else { [byte]$img.Width  }
        $h8 = if ($img.Height -ge 256) { 0 } else { [byte]$img.Height }
        $bw.Write([byte]$w8)
        $bw.Write([byte]$h8)
        $bw.Write([byte]0)        # color count (0 = no palette)
        $bw.Write([byte]0)        # reserved
        $bw.Write([UInt16]1)      # color planes
        $bw.Write([UInt16]32)     # bits per pixel
        $bw.Write([UInt32]$img.Bytes.Length)
        $bw.Write([UInt32]$offset)
        $offset += $img.Bytes.Length
    }

    foreach ($img in $images) {
        $bw.Write($img.Bytes)
    }

    $bw.Flush()
    [System.IO.File]::WriteAllBytes($OutPath, $ms.ToArray())
    $ms.Dispose()
}

$iconDir = Split-Path -Parent $PSCommandPath
$tmpDir  = Join-Path $env:TEMP "openvoice-icon-build"
if (Test-Path $tmpDir) { Remove-Item $tmpDir -Recurse -Force }
New-Item $tmpDir -ItemType Directory | Out-Null

# Sizes used by tauri.conf.json + general system needs.
$tauriSizes = @(
    @{ Size =  32; Name = "32x32.png" },
    @{ Size =  64; Name = "64x64.png" },
    @{ Size = 128; Name = "128x128.png" },
    @{ Size = 256; Name = "128x128@2x.png" },
    @{ Size = 512; Name = "icon.png" }
)
foreach ($t in $tauriSizes) {
    $out = Join-Path $iconDir $t.Name
    New-SoundwaveIcon -Size $t.Size -OutPath $out
    Write-Host ("Wrote {0,4}x{1,-4}  {2}" -f $t.Size, $t.Size, $out)
}

# Multi-resolution ICO. Common Windows sizes (Explorer uses 16/32/48,
# alt-tab uses 256, taskbar 32-48 at 1x and up).
$icoSizes = 16, 32, 48, 64, 128, 256
$icoPaths = foreach ($s in $icoSizes) {
    $p = Join-Path $tmpDir "ico-$s.png"
    New-SoundwaveIcon -Size $s -OutPath $p
    $p
}
$icoOut = Join-Path $iconDir "icon.ico"
Write-Ico -PngPaths $icoPaths -OutPath $icoOut
Write-Host ("Wrote ICO with sizes [{0}]  {1}" -f ($icoSizes -join ', '), $icoOut)

# Landing PNG sizes — apple-touch-icon (180) + favicon-32 + favicon-16.
$landingDir = Resolve-Path (Join-Path $iconDir "..\..\landing\assets")
foreach ($pair in @(
    @{ Size = 180; Name = "icon-180.png" },
    @{ Size =  32; Name = "icon-32.png"  },
    @{ Size =  16; Name = "icon-16.png"  },
    @{ Size = 512; Name = "icon-512.png" }
)) {
    $out = Join-Path $landingDir $pair.Name
    New-SoundwaveIcon -Size $pair.Size -OutPath $out
    Write-Host ("Wrote {0,4}x{1,-4}  {2}" -f $pair.Size, $pair.Size, $out)
}

Remove-Item $tmpDir -Recurse -Force
Write-Host "`nDone."

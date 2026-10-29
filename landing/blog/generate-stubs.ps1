# ════════════════════════════════════════════════════════════════════════════
# Generate "coming-soon" stub HTML files for every draft post in manifest.json.
#
# Why this script exists:
#   - Each launch post links to upcoming posts (related-card grid, inline links).
#   - Until the cron flips them live, those URLs need to exist as something
#     other than a 404. A stub page that says "publishing on <date>, subscribe
#     to RSS" gives crawlers something to index and gives early visitors a
#     graceful landing.
#   - When the cron flips a post live, the maintainer replaces the stub HTML
#     with the real post — same URL, same OG tags, no broken links.
#
# Usage:
#   pwsh ./generate-stubs.ps1               # write only files that don't exist
#   pwsh ./generate-stubs.ps1 -Force        # overwrite existing stubs
#
# Idempotent: by default it skips slugs whose HTML file already exists, so
# the four launch posts (introducing-openvoice, windows-quickstart,
# openvoice-vs-wispr-flow, why-whisper-tiny-ships-in-the-installer) aren't
# touched.
# ════════════════════════════════════════════════════════════════════════════

[CmdletBinding()]
param(
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
$blogDir   = $PSScriptRoot
$postsDir  = Join-Path $blogDir 'posts'
$manifest  = Get-Content (Join-Path $blogDir 'manifest.json') -Raw | ConvertFrom-Json

if (-not (Test-Path $postsDir)) {
  New-Item -ItemType Directory -Path $postsDir | Out-Null
}

$categoryLabels = @{}
foreach ($key in $manifest.categories.PSObject.Properties.Name) {
  $categoryLabels[$key] = $manifest.categories.$key.label
}

# Pretty-print "2026-06-04" as "June 4, 2026" without depending on Get-Date
# parsing in a way that's locale-sensitive.
function Format-LongDate {
  param([string]$IsoDate)
  $parts  = $IsoDate.Split('-')
  $year   = [int]$parts[0]
  $month  = [int]$parts[1]
  $day    = [int]$parts[2]
  $names  = 'January','February','March','April','May','June','July','August','September','October','November','December'
  return "$($names[$month-1]) $day, $year"
}

$created = 0
$skipped = 0

# Launch posts have hand-written content — the stub generator must never
# overwrite them, even with -Force. The cron's publish-due-posts.ps1 only
# flips the manifest's draft flag; it never touches the HTML files.
$LAUNCH_POSTS = @(
  'introducing-openvoice',
  'windows-quickstart',
  'openvoice-vs-wispr-flow',
  'why-whisper-tiny-ships-in-the-installer'
)

foreach ($post in $manifest.posts) {
  $slug      = $post.slug
  $outPath   = Join-Path $postsDir "$slug.html"

  if ($LAUNCH_POSTS -contains $slug) {
    $skipped++
    continue
  }

  if ((Test-Path $outPath) -and -not $Force) {
    $skipped++
    continue
  }

  $catKey    = $post.category
  $catLabel  = if ($categoryLabels.ContainsKey($catKey)) { $categoryLabels[$catKey] } else { $catKey }
  $dayLabel  = if ($post.day -eq 0) { 'Day 0' } else { "Day $($post.day)" }
  $longDate  = Format-LongDate $post.publish
  $minutes   = $post.readMinutes
  $title     = $post.title
  $dek       = $post.dek
  $tags      = ($post.tags -join ', ')

  # HTML-escape user-visible strings — tags inside the manifest could in
  # theory contain quotes/ampersands; defensively encode all of them.
  function Encode-Html {
    param([string]$s)
    if ($null -eq $s) { return '' }
    return $s.Replace('&','&amp;').Replace('<','&lt;').Replace('>','&gt;').Replace('"','&quot;').Replace("'", '&#39;')
  }

  $titleEnc = Encode-Html $title
  $dekEnc   = Encode-Html $dek
  $tagsEnc  = Encode-Html $tags

  $html = @"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content="$dekEnc Publishing on $longDate as part of the OpenVoice Dispatch — a 30-day series on local-first voice dictation." />
    <meta name="theme-color" content="#7c3aed" />
    <meta name="robots" content="noindex, follow" />
    <meta property="og:title" content="$titleEnc — coming $longDate" />
    <meta property="og:description" content="$dekEnc" />
    <meta property="og:type" content="article" />
    <meta property="og:url" content="https://handy.apexaios.io/blog/posts/$slug.html" />
    <meta property="article:published_time" content="$($post.publish)" />
    <meta property="article:section" content="$catLabel" />
    <title>$titleEnc — coming $longDate · OpenVoice Dispatch</title>
    <link rel="alternate" type="application/rss+xml" title="OpenVoice Dispatch RSS" href="/blog/feed.xml" />
    <link rel="canonical" href="https://handy.apexaios.io/blog/posts/$slug.html" />
    <link rel="preconnect" href="https://fonts.bunny.net" crossorigin />
    <link href="https://fonts.bunny.net/css?family=inter:400,500,600,700&display=swap" rel="stylesheet" />
    <link rel="stylesheet" href="/styles.css" />
    <link rel="stylesheet" href="/blog/blog.css" />
    <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='8' fill='%237c3aed'/%3E%3Crect x='9' y='13' width='3' height='6' rx='1.5' fill='white'/%3E%3Crect x='14.5' y='9' width='3' height='14' rx='1.5' fill='white'/%3E%3Crect x='20' y='11' width='3' height='10' rx='1.5' fill='white'/%3E%3C/svg%3E" />
    <link rel="icon" type="image/png" sizes="32x32" href="/assets/icon-32.png" />
    <link rel="apple-touch-icon" sizes="180x180" href="/assets/icon-180.png" />
  </head>
  <body>
    <a class="skip-link" href="#main">Skip to content</a>

    <header class="site-nav">
      <div class="nav-inner">
        <a href="/" class="brand" aria-label="OpenVoice home">
          <svg class="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
            <rect width="32" height="32" rx="8" class="brand-mark-fill" />
            <rect x="9" y="13" width="3" height="6" rx="1.5" fill="currentColor" class="brand-mark-stroke" />
            <rect x="14.5" y="9" width="3" height="14" rx="1.5" fill="currentColor" class="brand-mark-stroke" />
            <rect x="20" y="11" width="3" height="10" rx="1.5" fill="currentColor" class="brand-mark-stroke" />
          </svg>
          <span class="brand-wordmark">OpenVoice</span>
        </a>
        <nav class="nav-links" aria-label="Primary">
          <a href="/#features">Features</a>
          <a href="/#privacy">Privacy</a>
          <a href="/#download">Download</a>
          <a href="/blog/" aria-current="page">Dispatch</a>
        </nav>
        <div class="nav-actions">
          <button type="button" class="theme-toggle" id="themeToggle" aria-label="Toggle theme">
            <span class="theme-icon" data-theme-icon></span>
            <span class="theme-label" data-theme-label>System</span>
          </button>
          <a class="btn btn-ghost" href="https://github.com/Razepriv/openwhisper" rel="noopener">GitHub</a>
        </div>
      </div>
    </header>

    <main id="main">
      <article class="article">
        <a class="article-back" href="/blog/">← The Dispatch</a>

        <div class="draft-banner">
          <strong>Coming $longDate.</strong> This post is part of the OpenVoice Dispatch series and publishes on the date above. Until then, you'll find a placeholder. <a href="/blog/feed.xml">Subscribe via RSS</a> to be notified when it goes live.
        </div>

        <header class="article-header">
          <span class="article-kicker">$catLabel · $dayLabel</span>
          <h1 class="article-title">$titleEnc</h1>
          <p class="article-dek">$dekEnc</p>
          <div class="article-byline">
            <span>By the OpenVoice maintainers</span>
            <span>Scheduled for $longDate</span>
            <span>$minutes min read</span>
          </div>
        </header>

        <div class="article-body">
          <h2>What this post will cover</h2>
          <p>$dekEnc</p>

          <p>
            Tags: <em>$tagsEnc</em>.
          </p>

          <p>
            In the meantime, the four launch posts are available now — including
            <a href="/blog/posts/introducing-openvoice.html">an overview of what shipped in v0.2.0</a>,
            <a href="/blog/posts/windows-quickstart.html">the five-minute Windows install</a>,
            <a href="/blog/posts/openvoice-vs-wispr-flow.html">the head-to-head against Wispr Flow</a>, and
            <a href="/blog/posts/why-whisper-tiny-ships-in-the-installer.html">the privacy argument for bundling Whisper Tiny in the installer</a>.
          </p>

          <div class="cta-box">
            <h3 class="cta-box-title">Try OpenVoice today</h3>
            <p class="cta-box-dek">Free, open-source, 100% local. Available now for Mac, Windows, and Linux.</p>
            <a class="btn btn-primary btn-lg" href="/#download">Download OpenVoice</a>
          </div>
        </div>

        <section class="related">
          <div class="related-label">Read now</div>
          <div class="related-grid">
            <a class="related-card" href="/blog/posts/introducing-openvoice.html">
              <div class="related-card-kicker">Day 0 · Launch</div>
              <h3 class="related-card-title">Introducing OpenVoice</h3>
            </a>
            <a class="related-card" href="/blog/posts/windows-quickstart.html">
              <div class="related-card-kicker">Day 1 · Tutorial</div>
              <h3 class="related-card-title">5-minute Windows install</h3>
            </a>
            <a class="related-card" href="/blog/posts/openvoice-vs-wispr-flow.html">
              <div class="related-card-kicker">Day 2 · vs</div>
              <h3 class="related-card-title">OpenVoice vs Wispr Flow</h3>
            </a>
          </div>
        </section>
      </article>
    </main>

    <footer class="colophon">
      <div class="colophon-rule"></div>
      <div class="colophon-grid">
        <div>
          <div class="colophon-label">OpenVoice</div>
          <p>100% local, 100% free voice dictation for Mac, Windows, and Linux. <a href="/">Back to the home page</a>.</p>
        </div>
        <div>
          <div class="colophon-label">Subscribe</div>
          <p>New posts daily for 30 days. Get them in your reader via <a href="/blog/feed.xml">RSS</a>.</p>
        </div>
        <div>
          <div class="colophon-label">Lineage</div>
          <p>Forked from <a href="https://github.com/cjpais/Handy" rel="noopener">cjpais/Handy</a> (MIT). Built on whisper.cpp, llama.cpp, and Tauri.</p>
        </div>
        <div>
          <div class="colophon-label">Find us</div>
          <p>
            <a href="https://github.com/Razepriv/openwhisper" rel="noopener">GitHub</a> ·
            <a href="https://github.com/Razepriv/openwhisper/releases" rel="noopener">Releases</a> ·
            <a href="/blog/feed.xml">RSS</a>
          </p>
        </div>
      </div>
      <div class="colophon-bottom">
        <span>(c) <span id="year">2026</span> OpenVoice contributors</span>
        <span>No telemetry. No tracking. No cookies on this page.</span>
      </div>
    </footer>

    <script src="/script.js" defer></script>
  </body>
</html>
"@

  Set-Content -Path $outPath -Value $html -Encoding UTF8 -NoNewline
  $created++
}

Write-Host "OK   created stubs: $created"
Write-Host "skip already-present: $skipped"

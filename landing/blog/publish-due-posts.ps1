# ============================================================================
# Flip any past-due draft posts to live and regenerate feed + sitemap.
#
# Runs daily via .github/workflows/blog-drip.yml at 09:00 UTC. Also safe to
# run by hand whenever - it's idempotent (a post can only be flipped from
# draft to live once; running again is a no-op).
#
# Logic:
#   1. Read manifest.json
#   2. For each post where draft == true AND publish <= today (UTC):
#        - Set draft = false
#        - Log the slug
#   3. If anything changed, write manifest.json back AND regenerate
#      feed.xml + sitemap.xml. The exit code is 2 to signal "files
#      changed, commit & redeploy" to the workflow.
#   4. If nothing changed, exit 0 - workflow short-circuits.
#
# Exit codes:
#   0 - no change (no posts came due today)
#   2 - manifest mutated, feed/sitemap regenerated, ready to commit
#   1 - error
# ============================================================================

[CmdletBinding()]
param(
  [string]$Today  # YYYY-MM-DD; defaults to today in UTC. Useful for testing.
)

$ErrorActionPreference = 'Stop'
$blogDir      = $PSScriptRoot
$manifestPath = Join-Path $blogDir 'manifest.json'

if ([string]::IsNullOrEmpty($Today)) {
  $Today = [datetime]::UtcNow.ToString('yyyy-MM-dd')
}

Write-Host "[publish-due-posts] Today (UTC): $Today"

if (-not (Test-Path $manifestPath)) {
  Write-Error "manifest.json not found at $manifestPath"
  exit 1
}

# Treat the JSON as raw text rather than round-tripping through
# ConvertFrom-Json/ConvertTo-Json, which would mangle ordering and strip
# the leading $comment field. Parse a copy purely to identify what needs
# flipping; edit the raw text via scoped regex.
$rawJson = Get-Content $manifestPath -Raw

$parsed  = $rawJson | ConvertFrom-Json
$flipped = New-Object System.Collections.Generic.List[string]

foreach ($post in $parsed.posts) {
  if ($post.draft -and $post.publish -le $Today) {
    $flipped.Add($post.slug) | Out-Null
  }
}

if ($flipped.Count -eq 0) {
  Write-Host "[publish-due-posts] No posts due today. Exit 0."
  exit 0
}

Write-Host "[publish-due-posts] Flipping $($flipped.Count) post(s) to live:"
foreach ($s in $flipped) { Write-Host "  - $s" }

# For each slug due today, find the block from "slug": "<slug>" through
# the next "draft": true and flip that one boolean. Scoping by slug
# prevents accidentally flipping a different post's draft flag.
foreach ($slug in $flipped) {
  $escapedSlug = [regex]::Escape($slug)
  $pattern = "(`"slug`":\s*`"$escapedSlug`"[\s\S]*?`"draft`":\s*)true"
  $replacement = '$1false'
  $newJson = [regex]::Replace($rawJson, $pattern, $replacement, [System.Text.RegularExpressions.RegexOptions]::None, [timespan]::FromSeconds(5))
  if ($newJson -eq $rawJson) {
    Write-Warning "[publish-due-posts] Slug '$slug' matched in parse but not in raw regex. Skipping."
    continue
  }
  $rawJson = $newJson
}

Set-Content -Path $manifestPath -Value $rawJson -Encoding UTF8 -NoNewline
Write-Host "[publish-due-posts] manifest.json updated."

# Regenerate feed + sitemap so they reflect the new live set.
$feedScript = Join-Path $blogDir 'generate-feed.ps1'
if (Test-Path $feedScript) {
  Write-Host "[publish-due-posts] Regenerating feed + sitemap..."
  & $feedScript
} else {
  Write-Warning "[publish-due-posts] generate-feed.ps1 not found - skipping feed/sitemap regen."
}

Write-Host "[publish-due-posts] Done. Exit 2 to signal changes."
exit 2

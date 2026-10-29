# ════════════════════════════════════════════════════════════════════════════
# Generate /blog/feed.xml (RSS 2.0) from manifest.json.
#
# Only includes posts where draft == false AND publish <= today, so the feed
# matches what visitors see on the blog index. Re-run any time the cron flips
# a post live — Vercel will pick up the new XML on the next deploy.
#
# Also writes /blog/sitemap.xml listing every live post for search-engine
# discovery. Stubs are intentionally noindex (in their <head> meta) and
# don't appear here.
#
# Usage:
#   pwsh ./generate-feed.ps1
# ════════════════════════════════════════════════════════════════════════════

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$blogDir   = $PSScriptRoot
$manifest  = Get-Content (Join-Path $blogDir 'manifest.json') -Raw | ConvertFrom-Json
$today     = (Get-Date).ToString('yyyy-MM-dd')

$siteTitle    = $manifest.site.title
$siteDek      = $manifest.site.dek
$siteBaseUrl  = $manifest.site.baseUrl
$siteFeedUrl  = $manifest.site.feedUrl
$siteAuthor   = $manifest.site.author

$categoryLabels = @{}
foreach ($key in $manifest.categories.PSObject.Properties.Name) {
  $categoryLabels[$key] = $manifest.categories.$key.label
}

# RSS uses RFC-822 dates ("Fri, 31 May 2026 09:00:00 +0000"); we publish at
# 09:00 UTC each day (the cron schedule), so stamp every item at that time.
function Format-Rfc822 {
  param([string]$IsoDate)
  $parts = $IsoDate.Split('-')
  $dt = [datetime]::new([int]$parts[0], [int]$parts[1], [int]$parts[2], 9, 0, 0, [System.DateTimeKind]::Utc)
  return $dt.ToString('ddd, dd MMM yyyy HH:mm:ss +0000', [System.Globalization.CultureInfo]::InvariantCulture)
}

function Encode-Xml {
  param([string]$s)
  if ($null -eq $s) { return '' }
  return $s.Replace('&','&amp;').Replace('<','&lt;').Replace('>','&gt;').Replace('"','&quot;').Replace("'", '&apos;')
}

# @() forces array wrapping so .Count is reliable when the pipeline
# returns a single item (Where-Object collapses to scalar otherwise).
$liveItems = @($manifest.posts | Where-Object { -not $_.draft -and $_.publish -le $today } |
  Sort-Object -Property publish -Descending)

$lastBuildDate = Format-Rfc822 $today

$itemsXml = ($liveItems | ForEach-Object {
  $post     = $_
  $slug     = $post.slug
  $url      = "$siteBaseUrl/posts/$slug.html"
  $title    = Encode-Xml $post.title
  $dek      = Encode-Xml $post.dek
  $pubDate  = Format-Rfc822 $post.publish
  $catLabel = if ($categoryLabels.ContainsKey($post.category)) { $categoryLabels[$post.category] } else { $post.category }
  $catEnc   = Encode-Xml $catLabel
  @"
    <item>
      <title>$title</title>
      <link>$url</link>
      <guid isPermaLink="true">$url</guid>
      <description>$dek</description>
      <category>$catEnc</category>
      <pubDate>$pubDate</pubDate>
      <author>noreply@apexaios.io ($siteAuthor)</author>
    </item>
"@
}) -join "`n"

$siteTitleEnc = Encode-Xml $siteTitle
$siteDekEnc   = Encode-Xml $siteDek

$feed = @"
<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>$siteTitleEnc</title>
    <link>$siteBaseUrl/</link>
    <atom:link href="$siteFeedUrl" rel="self" type="application/rss+xml" />
    <description>$siteDekEnc</description>
    <language>en-us</language>
    <lastBuildDate>$lastBuildDate</lastBuildDate>
    <generator>OpenVoice Dispatch — generate-feed.ps1</generator>
$itemsXml
  </channel>
</rss>
"@

$feedPath = Join-Path $blogDir 'feed.xml'
Set-Content -Path $feedPath -Value $feed -Encoding UTF8 -NoNewline
Write-Host "feed.xml written with $($liveItems.Count) live items"

# ── Sitemap ────────────────────────────────────────────────────────────
$sitemapItems = ($liveItems | ForEach-Object {
  $slug    = $_.slug
  $url     = "$siteBaseUrl/posts/$slug.html"
  $lastmod = $_.publish
  @"
  <url>
    <loc>$url</loc>
    <lastmod>$lastmod</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
"@
}) -join "`n"

$sitemap = @"
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>$siteBaseUrl/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
$sitemapItems
</urlset>
"@

$sitemapPath = Join-Path $blogDir 'sitemap.xml'
Set-Content -Path $sitemapPath -Value $sitemap -Encoding UTF8 -NoNewline
Write-Host "sitemap.xml written with $($liveItems.Count) URL(s)"

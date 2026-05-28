# handy — landing page

Static marketing page for handy. Plain HTML/CSS/JS, no build step.

## What's here

```
landing/
├── index.html      # the whole page
├── styles.css      # design tokens + layout (mirrors src/App.css)
├── script.js       # theme toggle, OS detection, footer year
└── assets/         # any future images / screenshots
```

## Preview locally

Any static-file server works. Two options:

```bash
# Python (no install if you have Python)
cd landing && python -m http.server 8080
# then open http://localhost:8080

# Bun (already a project dependency)
cd landing && bunx serve .
# then open the URL it prints
```

## Deploy

Anywhere that serves static files. Three sensible options for handy:

1. **GitHub Pages** — push to `gh-pages` branch or enable Pages on `main /landing`.
2. **Cloudflare Pages** — connect the repo, set the build output directory to `landing/`.
3. **Vercel / Netlify** — same idea, set the publish directory to `landing/`.

No env vars, no secrets, no API routes. The download links resolve to GitHub Releases.

## Design system

Tokens mirror the app's editorial palette (see `../src/App.css`):

- **Accent**: violet — `#7c3aed` (light) / `#a78bfa` (dark), used sparingly
- **Type**: Inter from Bunny Fonts (privacy-friendly Google Fonts mirror), JetBrains Mono for code
- **Theme**: `<html data-theme="light|dark">` set from `localStorage`; falls through to `prefers-color-scheme` when no preference is stored

## Editing checklist

When releasing a new version, update these strings:

- `index.html` → `eyebrow` ("v0.2.0 — first public release")
- `index.html` → download card file names (`handy_0.2.0_*`)
- `index.html` → `Download v0.2.0` section heading + CTA button
- Asset URLs are version-agnostic — they all point to `/releases/latest`.

## Accessibility

- Skip link to `#main` for keyboard users
- Visible focus ring on all interactive elements via `:focus-visible`
- `prefers-reduced-motion` is respected (disables the listening-pulse + smooth scroll)
- Semantic landmarks (`header`, `main`, `footer`, `nav`, `section`)

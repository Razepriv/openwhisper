# OpenVoice — landing page

Static marketing page for OpenVoice. Plain HTML/CSS/JS, no build step.

**Live at** → [handy.apexaios.io](https://handy.apexaios.io)

## What's here

```
landing/
├── index.html      # the whole page
├── styles.css      # design tokens + layout (mirrors src/App.css)
├── script.js       # theme toggle, OS detection, footer year
├── vercel.json     # Vercel config (headers, CSP, cache rules)
└── assets/         # screenshots / future images
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

The page ships **live on Vercel** today. Two backup paths if you want
mirrors or to fork the deploy:

### Vercel (production)

Vercel project: `razeprivs-projects/openvoice-landing`. Two ways to ship a
new version:

```bash
# CLI — auto-deploys whatever's in landing/ as production
cd landing && bunx vercel --prod

# Or via the dashboard
# 1. Push to main (no build step — Vercel just uploads the folder)
# 2. The connected Git integration auto-deploys
```

`vercel.json` in this folder sets:
- Strict security headers (CSP, X-Frame-Options, Permissions-Policy)
- Long-cache (`immutable`) on `/assets/*`
- Hour-cache on `styles.css` / `script.js`
- Clean URLs (no `.html` extension needed)

The `.vercel/` folder (project ID + auth) is gitignored — anyone
running `vercel link` from this folder gets prompted to link to the
existing `openvoice-landing` project.

### GitHub Pages (alternate / mirror)

A workflow at `.github/workflows/landing.yml` deploys this folder to
GitHub Pages on every push to `main`. Enable it once in repo settings:

```
Settings → Pages → Source = "GitHub Actions" → Save
```

Live URL after enabling: `https://razepriv.github.io/openwhisper/`.

### Cloudflare Pages / Netlify

Same idea — connect the repo, set the publish directory to `landing/`.
No build command needed.

No env vars, no secrets, no API routes. All download links resolve to
GitHub Releases.

## Design system

Tokens mirror the app's editorial palette (see `../src/App.css`):

- **Accent**: violet — `#7c3aed` (light) / `#a78bfa` (dark), used sparingly
- **Type**: Inter from Bunny Fonts (privacy-friendly Google Fonts mirror), JetBrains Mono for code
- **Theme**: `<html data-theme="light|dark">` set from `localStorage`; falls through to `prefers-color-scheme` when no preference is stored

## Editing checklist

When releasing a new version, update these strings:

- `index.html` → `eyebrow` ("v0.2.0 — first public release")
- `index.html` → download card file names (`OpenVoice_0.2.0_*`)
- `index.html` → `Download v0.2.0` section heading + CTA button
- Asset URLs are version-agnostic — they all point to `/releases/latest`.

## Accessibility

- Skip link to `#main` for keyboard users
- Visible focus ring on all interactive elements via `:focus-visible`
- `prefers-reduced-motion` is respected (disables the listening-pulse + smooth scroll)
- Semantic landmarks (`header`, `main`, `footer`, `nav`, `section`)

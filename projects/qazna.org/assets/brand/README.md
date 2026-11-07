# Qazna brand assets

The canonical red shanyrak is `qazna-shanyrak.svg`. It preserves the geometry,
1.83-second pulse and moving strokes of the author-provided
`shanraq.org/web/static/brand/shanraq-light.svg`; the source remains unchanged.
Source SHA-256: `2f1d4e8ed83a44294ef1e6244edfce31b0b10ae09a142e0c5c292af4699cffcb`.
The adaptation was requested by the author on 7 November 2026. The red mark
represents continuous circulation of provision, not a medical service.

- `qazna-shanyrak.svg`: scalable animated master, with reduced-motion support.
- `favicon.svg`: static, solid, transparent favicon derived from the same lines.
- `favicon.ico`: 16/32/48-pixel PNG entries for browser compatibility.
- `favicon-32.png`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png`: raster derivatives.
- `badges/`: local README badges generated from Cargo metadata and declared project requirements.

Run `python3 scripts/brand_assets.py` to regenerate derivatives and badges using
only the Python standard library. No uploaded image, external image service,
web font or CDN is required. Source and derivatives must not be copied into
multiple application directories: reference this directory, or copy from it as a
build step. Animation support depends on the renderer; favicon assets are static.
These project graphics are provided under CC BY-SA 4.0, as project documentation.
The source brand remains the author's work; this does not confer trademark rights.

For a web build that serves this directory at `/brand/`:

```html
<link rel="icon" href="/brand/favicon.svg" type="image/svg+xml">
<link rel="icon" href="/brand/favicon.ico" sizes="16x16 32x32 48x48">
<link rel="apple-touch-icon" href="/brand/apple-touch-icon.png">
```

# Qazna brand assets

The canonical red shanyrak is `qazna.svg`. It preserves the geometry,
1.83-second pulse and moving strokes of the author-provided
`shanraq.org/web/static/brand/shanraq-light.svg`; the source remains unchanged.
Source SHA-256: `2f1d4e8ed83a44294ef1e6244edfce31b0b10ae09a142e0c5c292af4699cffcb`.
The adaptation was requested by the author on 7 November 2026. The red mark
represents continuous circulation of provision, not a medical service.

Both diagonal groups have bright red central channels (`#ff0815`) and deep red
outer channels (`#cd0011`), with a `#ff000d` base at 30% opacity.
This palette was adjusted by the author on 7 November 2026 to remove the brown
cast; its exact source values are preserved. The two groups flow in opposite directions. Color is
arranged symmetrically across the mark; it is not a literal anatomical diagram.
The dash period matches the animation displacement, avoiding a jump at the loop
boundary. The static derivatives preserve this central/outer palette. The logo
and small icons were visually inspected on light and dark backgrounds in Chrome.

- `qazna.svg`: scalable animated master, with reduced-motion support.
- `favicon.svg`: static, solid, transparent favicon derived from the same lines.
- `favicon.ico`: woven 16/32/48-pixel PNG entries for browser compatibility.
- `favicon-32.png`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png`: woven raster derivatives.

At the author's request, the raster variants alternate over/under at neighbouring
crossings. A soft contact shadow and restrained cylindrical highlight distinguish
the channels even where two bright lines cross. Their transparent background,
canonical geometry and central/outer palette are preserved. The accepted animated
`qazna.svg` stays unchanged; `favicon.svg` remains the flat static vector option.
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

# Tartarus repository cover

The repository header links a lightweight 1920 × 1080 JPEG preview to the full
3840 × 2160 PNG (16:9). Both preserve the same composition. The cover was generated
with the built-in OpenAI image-generation tool on 7 November 2026, then exported
with macOS `sips`: the generated raster was 1672 × 941, resampled to 3840 × 2160.
Thus 4K describes the delivered dimensions, not native generated detail.

This is a newly generated editorial illustration, not a historical photograph,
archaeological reconstruction, source tablet or deciphered inscription. The
cuneiform-like marks are decorative. It does not replace the book's existing cover
or any source image. No blanket license for the book/archive is implied.

The text badges are repository-native SVG. Rust 1.88 is Qazna's declared minimum,
PostgreSQL 16 is the tested database version, and the CI badge explicitly follows
the development branch. Software status does not validate historical hypotheses.

Files:
- `tartarus-cover-4k.png`: 3840 × 2160 publication asset.
- `tartarus-cover-web.jpg`: 1920 × 1080 README preview.
- `badges/*.svg`: book/archive/project and technology links.

## Generation prompt

Use case: stylized-concept. Create a premium editorial cover illustration for the GitHub research repository tartarus associated with the book «Тартар и Я». Wide 16:9 landscape, exact 3840x2160 pixels 4K. A beautiful sweeping Kazakhstan steppe at dawn, deep midnight blue sky opening into luminous turquoise and warm gold, layered archaeological earth in foreground with a clay tablet carrying decorative cuneiform-like impressions (artistic motifs, not a claimed transcription), a warm red and gold continuous branching circulation of light connecting ancient layers to a restrained modern network of settlements, energy, farms and knowledge on the horizon. Elegant, visually coherent and richly detailed, no clutter, no sci-fi robots or aliens, no national flags. One large highly legible central title in warm ivory: "ТАРТАР и Я", all on one line, same color, dignified serif typography with subtle aged texture. Small subordinate line beneath in clean type: "TARTARUS". Keep text safely inset from all edges. Evocative ancient knowledge and humane future, illustrative rather than documentary, cinematic depth, polished print-quality artwork. No extra words, no watermark, no badges within image.

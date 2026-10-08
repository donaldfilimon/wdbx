# Abbey trailer format review - 2026-10-03

**Source Standards: PASS. Source Spec: PASS. Encoded local acceptance: PASS.**
Human listening and final creative approval remain separate.

Reviewer: root, independent of the media implementer. Reviewed the complete
`/tmp/webpress-trailer-profiles-review.patch`, actual preview frames and decoded
4K master contact sheet. Scope is profile support, isolated new outputs and
regressions in the existing renderer; product source and original authored inputs
are outside the change. No new dependencies, assets or product claims introduced.

## Standards

- Explicit profiles select pixel dimensions, CSS canvas and rasterization scale.
  Unknown profiles fail; captured PNG dimensions and decoded MP4 dimensions must
  match. Both cuts require H.264 video, 48 kHz AAC and stereo channels.
- Every profile owns its output directory and manifest. Existing output refusal
  and legacy 1080p verification remain intact; inferred verification profile comes
  from the recorded report and explicit mismatches fail.
- Renderer hash and profile metadata pin the source and layout. Original delivery
  provenance stays historical and does not falsely acquire a new render hash.
- Existing three real-I/O manifest regressions are retained. Three additional
  tests cover declared profile geometry, unknown profile refusal and rejection
  of landscape dimensions for portrait output. Actual Chromium bounds/raster
  checks supplement these tests; mirror assertions alone are not visual proof.
- One browser and two codec threads bound resource use. Frame sampling, source
  hash checks, full decode and faststart checks remain in the pipeline.

## Spec and visual inspection

Root inspected portrait first, third and fifth chapter previews and the 4K third
chapter preview. Portrait has a separate 540 × 960 CSS layout at 2× rasterization:
reflowed heading and subcopy, stacked chips, separate brand/edition and footer
rows. Text is legible and fits; all original five scenes/copy remain. It is not a
landscape crop. 4K uses the original 1200 × 675 CSS composition rasterized at 3.2×,
not an upscale of the old MP4. The decoded 4K contact sheet was also inspected
across all five chapters after rendering; copy and layout are intact.

The minor request to assert the short cut's stereo channels and H.264 codec was
implemented before rendering. Worker reports six focused regressions passing
and seven bounds checks per profile. Root has not asserted human audio approval.

## Reviewed source hashes

- `render.py`: `8b7673f87e1eb23959a64c474fa84266a167e198073fb67219bd744a874c19f8`
- `test_render.py`: `4f1c04c4bfbc44c62b33eef9e33fafb415d9275a3ca18d41182e325ac71c1fa4`
- `RENDERING.md`: `18e45be3890cd8b94f89ad7254f6c1240e04468870cacfcb899cbcc7a9e45eff`


## Initial artifact boundary (superseded below)

The 4K master and 15-second derivative completed and decoded successfully. Portrait
render was still active at initial review. Final review will record encoded portrait inspection and
artifact hashes after the process finishes. Existing 1080p delivery is preserved;
no broad WebPress product-gate rerun is claimed for this example-only slice.


## Final encoded closure

Both profiles and both 15-second derivatives reached terminal exit 0. Worker
verified all four complete decodes, dimensions, 900/450 frames, 30/15 second duration,
H.264/yuv420p at 30 fps, 48 kHz stereo AAC and faststart. Root independently inspected
both decoded portrait five-beat contact sheets after encoding, confirming legible
reflowed copy and intact headers/footer, and rehashed every listed artifact in
both profile manifests with zero mismatches. Root also inspected all five decoded
4K master beats. This is sampled visual review plus complete technical decode,
not human listening or review of every displayed frame.

All 21 original authored/input/output/manifest files preserve their earlier bytes.
The retained receipt is
`.superpowers/sdd/2026-10-03-ecosystem/evidence/abbey-trailer-profiles-receipt.json`
from the repository root. Six focused regressions,
Python AST parsing and exact Biome checks passed. No encoder or renderer remains
running. No full product gate was repeated for this isolated example/profile work.

Final artifacts:

- `4k/abbey-trailer-4k.mp4`: 5,258,105 bytes, SHA-256 `9c34b742dabf2fe17810b4f64ccb4379f9e29a44b31056dd1e9bdd396146cf22`.
- `4k/abbey-trailer-15s-4k.mp4`: 3,396,225 bytes, SHA-256 `4f8d6f568e613aa6f47deb7ba2a19852e681954f4bafb6abd0f67f2a78afb3ca`.
- `vertical/abbey-trailer-vertical.mp4`: 2,573,075 bytes, SHA-256 `045803439914b3d9721eaacedfeedc61ee6e49af669ec89f40b73e93716bcd38`.
- `vertical/abbey-trailer-15s-vertical.mp4`: 1,573,489 bytes, SHA-256 `488979c715639a1194b7df7eb987674e3b1e9dbcc6503257a711bca51f82c70e`.

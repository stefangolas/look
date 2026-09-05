# Render evidence and third-party comparison images

`step-pcurve-shared-edge-before.png` and
`step-pcurve-shared-edge-after.png` are repository-owned renders of
`tests/fixtures/surface_curve_pcurve_master.step` with the same ISO camera
and default renderer settings. The before image was rendered at
`8190f7d`: two of 24 STEP faces were refused with
`EdgeTraversalUnresolved`, leaving 4,349 triangles and 13,047 vertices. The
after image was rendered with the shared-edge 3D-carrier fix: all faces are
present, with 5,344 triangles and 16,032 vertices.

Both the fixture and these two generated renders are licensed under the
repository's MIT-or-Apache-2.0 terms.

`damaged-helmet-look-vs-f3d.png` is the side-by-side comparison built from
`damaged-helmet-look.png` and `damaged-helmet-f3d.png` by
`benchmarks/make-comparison.py`. Regenerate it from the repository root:

```console
python benchmarks/make-comparison.py \
  --left docs/images/damaged-helmet-look.png --left-label "look" \
  --left-note "602 ms median" \
  --right docs/images/damaged-helmet-f3d.png --right-label "F3D 3.5" \
  --right-note "939 ms median" \
  --title "Khronos Damaged Helmet - 1.56x faster, matched settings" \
  --footer "6 fresh launches each - 512x512 front orthographic - source PBR - no AA/AO/tone mapping - #252525 background" \
  --output docs/images/damaged-helmet-look-vs-f3d.png
```

The Sponza and New York Boulevard comparisons were withdrawn. F3D renders
Intel Sponza black at every camera, lighting, tone mapping, and texture
resolution tried, so a side-by-side there would show a failed render next to a
working one rather than a difference in renderer quality. Their timing tables
remain in `BENCHMARKS.md`, which is the honest form for that result.

`damaged-helmet-look.png`, `damaged-helmet-f3d.png`, and
`damaged-helmet-tileset.png` are renderer outputs of the Khronos Damaged Helmet
glTF sample and are not covered by this repository's MIT-or-Apache-2.0 software
license.

Model credits and licenses:

- Copyright 2018 ctxwing, rebuild and glTF conversion, CC BY 4.0 International.
- Copyright 2016 theblueturtle_, earlier version of the model, CC BY-NC 4.0
  International.

Source and full legal metadata:
https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/DamagedHelmet

The images are included only to compare renderer output. They were produced at
512x512 using a front orthographic camera, source PBR materials, disabled AA,
AO, and tone mapping, a `#252525` background, F3D 3.5's default five-light kit,
and no F3D user configuration. The tileset uses the same model and source
materials, four 384x384 orthographic tiles (`front`, `right`, `top`, and `iso`),
and a two-column atlas.

`nyc-boulevard-look-vs-f3d.png` is a side-by-side renderer output of “New York
blvd.” by matousekfoto. The downloaded GLB embeds a CC-BY-NC-4.0 license and
links to:
https://sketchfab.com/3d-models/new-york-blvd-ed0701bcb94c4b1692bd97a54df19ad7

The image was produced at 4096x4096 using a front orthographic camera, source
materials, disabled AA, AO, and tone mapping, a `#252525` background, and no
F3D user configuration. Labels and downsampling were added to the comparison
image; the underlying full-resolution PNGs remain in the ignored benchmark
output directory.

`sponza-look-vs-f3d.png` is a side-by-side renderer output of Intel's Sponza
Base Scene. Intel distributes the sample under Creative Commons Attribution:
https://www.intel.com/content/www/us/en/developer/topic-technology/graphics-research/samples.html

The comparison used the original geometry and 72 textures resized from
4096x4096 to 2048x2048. It was produced at 512x512 using a front orthographic
camera, source PBR materials, disabled AA, AO, and tone mapping, a `#252525`
background, and no F3D user configuration.

`sponza-foliage-look-vs-f3d.png` uses the same settings and combines Intel's
official Sponza Base, Ivy, and Trees packages. The base textures were resized
to 2048x2048 while the Ivy and Trees textures remain at their distributed
resolution. The combined scene and comparison image are covered by the same
Creative Commons Attribution terms linked above.

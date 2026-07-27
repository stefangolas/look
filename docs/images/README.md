# Third-party comparison images

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
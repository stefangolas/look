# Test fixtures

- `triangle.stl` is the smallest parser and package smoke fixture.
- `aluminum_ball_bearing.glb` is a repository-owned, generated glTF 2.0 PBR
  fixture with a 128x64 sphere mesh and embedded 1024x512 base-color,
  occlusion/roughness/metallic, and normal textures.

- `bracket.step` is a repository-owned AP203 boundary representation: a block
  with a cylindrical boss and a through bore, so tessellation must handle
  planar and cylindrical trimmed faces rather than reading triangles. It exists
  to keep the STEP path covered without depending on an external CAD corpus.

- `surface_curve_pcurve_master.step` is repository-owned output from the
  build123d 0.11.0 reproducer in issue 1. Its SHA-256 is
  `bbec6fd474a959b945f83d9f11386a21429d1419c2da80e39e8c3efde2d69576`. It
  covers a `SURFACE_CURVE` whose PCurve master is shared by faces on unlike
  supporting surfaces.

Regenerate the ball bearing deterministically from the repository root:

```console
cargo run --release --example generate_ball_bearing
```

The generated model and generator are licensed under the repository's
MIT-or-Apache-2.0 terms. Do not replace the binary without regenerating it from
the checked-in source and rerunning the rendering and benchmark tests.

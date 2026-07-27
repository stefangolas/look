# Test fixtures

- `triangle.stl` is the smallest parser and package smoke fixture.
- `aluminum_ball_bearing.glb` is a repository-owned, generated glTF 2.0 PBR
  fixture with a 128x64 sphere mesh and embedded 1024x512 base-color,
  occlusion/roughness/metallic, and normal textures.

Regenerate the ball bearing deterministically from the repository root:

```console
cargo run --release --example generate_ball_bearing
```

The generated model and generator are licensed under the repository's
MIT-or-Apache-2.0 terms. Do not replace the binary without regenerating it from
the checked-in source and rerunning the rendering and benchmark tests.

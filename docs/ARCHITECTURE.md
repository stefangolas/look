# Architecture

`look` has one scene compiler and one native renderer with two execution modes.

```text
one shot: model -> parse/compile -> GPU upload -> render/readback -> PNG
session:  model -> parse/compile -> GPU upload -> retain scene
                                      |-> render/readback -> PNG
                                      |-> render/readback -> PNG
```

The session path changes lifecycle, not visual behavior. Both modes use the
same cameras, lighting, shaders, render targets, and PNG encoder.

## Modules

- `cli.rs`: command normalization, CLI-to-config mapping, and JSON responses
- `config.rs`: strict YAML types, defaults, validation, and named views
- `scene.rs`: direct GLB/STL parsing, transforms, bounds, geometry hashing,
  deduplication, instancing, and source-texture preparation
- `camera.rs`: stable bounds-fit perspective and orthographic matrices
- `renderer/wgpu_renderer.rs`: adapter/device setup, resource pooling, shader
  pipelines, GPU scene cache, per-view rendering, atlas rendering, and readback
- `technical.wgsl`: compact deterministic technical-material path
- `source_material.wgsl`: glTF metallic-roughness material path
- `output.rs`: PNG encoding and output naming
- `cache.rs`: versioned inspection-metadata cache
- `server.rs`: authenticated loopback sessions and GPU-scene lifetime
- `timing.rs`: named stage timings used in JSON results and benchmarks

## Fast paths

Technical rendering uses a compact 24-byte vertex stream and never decodes or
uploads source textures. Source mode decodes only referenced textures and uses
glTF material inputs. Equal geometry payloads share GPU buffers, and repeated
nodes become instances when their geometry/material representation is equal.

The renderer lazily creates only the pipelines a scene requires. A renderer
instance pools targets and readback resources and keeps up to four recently
used scenes GPU-resident. Multi-view atlas rendering uses one color/depth target
and one render pass with a viewport and scissor per tile. It then performs one
readback and one PNG encode.

## Session protocol

`look persist` auto-starts the hidden `look __serve` process. The server binds an
ephemeral `127.0.0.1` port and writes a versioned state file containing a random
token. Requests are newline-delimited JSON envelopes and are capped at 1 MiB.
Relative output paths are resolved against the client's working directory.

The server processes render work serially. This avoids accidental GPU-context
duplication, makes session behavior deterministic, and lets all sessions share
one adapter, device, pipeline set, target pool, and bounded GPU scene cache.
Sessions use idle TTLs and can be released explicitly. The whole server exits
after its idle interval when no work remains.

The protocol is intentionally local and private. It is not a network service,
does not bind non-loopback interfaces, and should not be exposed through a port
forwarder.

## Determinism boundary

`look` fixes traversal, camera fit, lighting presets, shader parameters, output
size, and background. A given executable and GPU stack should be stable enough
for perceptual regression tests. Rasterization, texture filtering, floating
point behavior, and driver shader compilation can still vary across backends;
cross-GPU validation therefore uses image metrics rather than byte equality.

The actual adapter, backend, vendor/device IDs, and driver are available through
`look doctor --json` and should be retained with benchmark artifacts.

## Deliberate omissions

The current executable does not include a browser, scene graph framework,
desktop UI, plugin ABI, STEP/IGES conversion, object-ID/depth/normal passes, or
disk-cached decoded geometry. Those can be added when a measured workflow needs
them without putting their abstractions on today's hot path.

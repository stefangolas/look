# WORK PACKET REF-RECORD-HYPERCAR — stage the three unstaged hypercar rows

The hypercar census recorded three rows UNSTAGED: hypercar/details,
hypercar/suspension_front, hypercar/suspension_rear have NO recorded
hypercar reference — their reference-file short names are occupied by
same-named f1 rows (reference/details.json carries row_id f1/details, etc.).
This packet fixes the reference naming convention, records the three
references through the OCC door, and stages the rows so the next census can
run them through the kernel door.

```yaml
id:          REF-RECORD-HYPERCAR
contract:    [REF-RECORD-HYPERCAR]
class:       mechanical
crates:      [truck123d]
depends_on:  []
write_allow:
  - corpus/ttc/reference/
  - corpus/ttc/MANIFEST.json
  - corpus/ttc/door.py
read_allow:
  - docs/HYPERCAR_CENSUS.md
  - corpus/ttc/
tests_required: []
anchors:
  - {id: A1, expect: 14, cmd: "grep -c 'hypercar' corpus/ttc/MANIFEST.json"}
  - {id: A2, expect: 45, cmd: "bash -c 'ls corpus/ttc/reference/*.json | wc -l'"}
budget:      {turns: 45, ctx_tokens: 130000}
```

## Method (the recorded census protocol, verbatim)

1. **Namespace fix, never an overwrite.** The three new reference files get
   hypercar-prefixed short names (e.g. `reference/hypercar_details.json`)
   OR the manifest rows gain an explicit `reference` field naming the file —
   pick the convention that touches the least surface, record the choice,
   and update the manifest rows to point at it. NEVER overwrite or edit the
   existing f1 reference files; they are the f1 rows' oracle.
2. **OCC recording runs.** One fresh python per row, serial, quiet machine
   (`--engine occ`, door_version 1), producing the recorded facts record
   (solid_count, volume, bbox, STL triangles) exactly as the existing 45
   references were recorded. Three runs total; wall ~1–20 min each
   (suspension rows are 89/63-solid assemblies — budget the wait).
3. **Re-run once per reference to confirm bit-identical reproduction** (the
   OCC baseline stability discipline; the corpus documents the flake
   regime — serial and quiet is mandatory, a second flake is a stop).
4. **Manifest rows update**: the three rows' staged flags per the manifest's
   own convention, pointing at the new references. No kernel-side file may
   be touched.

## Done when

The three reference files exist with the recorded facts shape; the manifest
rows stage them; re-runs reproduce bit-identically; anchors hold; the RESULT
carries the three rows' door evidence (solid_count / STL tris / wall_s) and
the naming-convention choice.

## Forbidden

Editing any existing reference file. Any kernel or bridge edit. Parallel
door runs. Editing the f1 rows' manifest entries beyond adding the reference
pointer fields.

## Stop conditions

- An OCC DNF or a flake on re-run → stop-and-file per the corpus's recorded
  OCC-flake regime (re-run serial and quiet once; a second flake stops).

## Finish by writing RESULT.json at the WORKTREE ROOT (then COMMIT first)

Commit subject: `census: hypercar unstaged rows recorded — 3 references staged, manifest namespaced (REF-RECORD-HYPERCAR)`.

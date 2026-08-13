# Failure Diagnostics (DIAG-002)

Structured, machine-readable diagnostics are a **constitutive** property of the
STEP/tessellation pipeline: every failed face, every certified rejection, and
every STEP→topology conversion loss automatically emits exactly one diagnostic
record. No environment variable is required to enable collection, and no
recompile, bespoke probe, or face-ID list is needed to analyze an unseen model.

## The contract

```text
TERMINAL FAILURE
    ⇒
EXACTLY ONE FaceDiagnosticRecord emitted

CONVERSION LOSS
    ⇒
EXACTLY ONE ImportDiagnosticRecord emitted
```

A face recovered by a formal recovery route emits nothing (it is not a terminal
failure). A face whose legacy attempt failed and whose every retry also failed
emits its **legacy** record exactly once, no matter how many layers the refusal
propagated through.

## Default behavior

```console
look model.step
```

If faces fail, the process writes one JSON record per failed/rejected face to
**stderr** — one record per line, no pretty-printing. External runners capture
it by redirecting stderr per model:

```console
look model.step 2> model.failures.jsonl
```

The tessellator never writes files implicitly and stdout stays clean for normal
program output.

## Sink selection

| Env | Effect |
| --- | --- |
| *(unset)* | default: records go to stderr |
| `TRUCK_FACE_DIAG_JSONL=<path>` | redirect/copy records to the file |
| `TRUCK_FACE_DIAG=off` | suppress external emission (defaults back to on) |

`TRUCK_FACE_DIAG_JSONL` **selects the destination; it does not activate
collection.** Collection is on by default because the formal recovery routes
read the derived witness bucket as an admission rule — the sink is a production
input, not a report. `TRUCK_FACE_DIAG=off` suppresses only emission, so
geometry and terminal outcomes are byte-identical with diagnostics on or off.
The full performance/embedding kill is `TRUCK_FACE_DIAG=off` together with
`TRUCK_FORMAL_RECOVERY=0`, which turns collection off with the routes that
consume it.

Diagnostic I/O can never crash or alter tessellation: a serialization failure
is swallowed, and a broken/unwritable file sink falls back to stderr with one
warning.

## Schema

`schema_version` is `1`. Analysis scripts must key off this field, never off
`Debug` formatting. Future versions may add fields without breaking v1 readers.

### `FaceDiagnosticRecord` — one per failed/rejected face

Identity: `document_id`, `source_face_id`, `source_use_id`
Terminal: `disposition` (`Failed` | `RejectedIntrinsic`), `terminal_reason`,
`failure_stage`
Structure: `surface_family`, `bound_count`, `edge_use_count`,
`distinct_vertex_count`, `periodic_axes`, `source_closed_axes`
Extents: `world_rank`, `world_bbox`, `world_diameter`, `approximate_world_area`,
`uv_rank`, `uv_bbox`
Tolerances: `tolerance.chord_tolerance`, `tolerance.source_geometric_uncertainty`,
`tolerance.incidence_tolerance`, `tolerance.compatibility_factor`
Witnesses: `source_edge`, `projection`, `lift`, `boundary`,
`validity_certificate`, `cdt_stages`, `route_decisions`
Census: `chart_rank`, `source_segment_count`, `synthetic_segment_count`,
`lift_status`, `deck_status`, `projection_status`, `seam_segment_count`,
`boundary_pieces`, `two_loop_join`, `seam_mechanism`, `insertion_conflicts`,
`overlap_conflicts`, `unattributed_overlaps`, `projection_witness`,
`cap_activation`, `derived_bucket`, `arr`

Every field is populated when the value was already computed during normal
processing or is cheaply derivable (an O(boundary) min/max pass, a counted
wire). Quantities requiring an additional geometric solve are left `None`
rather than computed for the record.

### `ImportDiagnosticRecord` — one per STEP conversion loss

Emitted by the CLI at the `truck-stepio` boundary, in the same sink:
`schema_version`, `document_id`, `source_face_id`, `source_use_id`,
`source_entity_type`, `conversion_stage`, `conversion_failure_kind`,
`representation_shell_context`, `refusal_tag`, `provenance_established`,
`declared_shell_faces`, `surviving_shell_faces`.

The hard invariant: if a source face existed and produced no output
face/provenance, a record explains where it was lost. Conversion records are
component-owned by Look (the import boundary), never forced into truck's
tessellation record.

## Failure stages

Recorded where the refusal occurs, never inferred later from the terminal
reason: `StepConversion`, `SourceEdgeTraversal`, `BoundaryProjection`,
`BoundaryLift`, `BoundaryConstruction`, `ValidityClassification`, `Arrangement`,
`ConstraintInsertion`, `MaterialSelection`, `TriangleValidation`, `Refinement`,
`SurfaceEvaluation`, `Other`.

## Terminal constructors (the compile-time gate)

A bare reason can no longer become a terminal failure:

```rust
// the only ways to make a terminal failure
let failure = diagnosis::fail(reason, stage);          // disposition = Failed
let failure = diagnosis::reject(reason, stage, cert);  // disposition = RejectedIntrinsic
```

Both finalize a `FaceDiagnosticRecord` from the current face's accumulated
evidence. Stage functions record **witnesses**; only the terminal finalizer
(`diagnosis::finalize_and_emit`, called once per face at the terminal decision
point) emits. A future terminal path that does not go through `fail`/`reject`
will not compile.

## Major witness types

- **`projection`** — `ProjectionRefusalWitness`: `kind` (e.g.
  `NoInverseCandidate`, `ResidualAboveTolerance`, `EvaluatorOutOfDomain`,
  `SingularEvaluation`, `PartialProjection`), attempted/successful/failed
  sample counts, min/max residual, acceptance tolerance, candidate UV, world
  point.
- **`lift`** — `LiftWitness`: candidate copy count, axes, deck shifts, why no
  candidate dominated.
- **`boundary`** — `BoundaryWitness`: bound/edge, pieces attempted/accepted,
  point counts, constraints that would have been presented, the exact refusal.
- **`source_edge`** — `SourceEdgeWitness`: bound, edge use, endpoint residuals,
  declared source uncertainty, effective incidence tolerance.
- **`validity_certificate`** — the FACE-VALIDITY certificate behind a
  `RejectedDegenerate`.
- Preserved from NoOdd/CDT: `cdt_stages` counts, `insertion_conflicts`,
  `overlap_conflicts`, `two_loop_join`, `seam_mechanism`, `arr`, `derived_bucket`.

## Analyzing the JSONL

```console
# one JSON object per line; jq-style aggregation works directly
jq -r 'select(.terminal_reason == "BoundaryProjectionFailed") | .projection.kind' model.failures.jsonl | sort | uniq -c

# conversion-stage histogram
jq -r 'select(.conversion_stage != null) | .conversion_stage' model.failures.jsonl | sort | uniq -c

# a failing face's local evidence
jq 'select(.source_face_id == 97164)' model.failures.jsonl
```

### Validation on core_xy.step (5,670 faces)

A default `look core_xy.step` emitted 47 records with no enable flag:

```text
BoundaryProjectionFailed x34
    failure-kind histogram: EvaluatorOutOfDomain x33, SingularEvaluation x1
    periodic vs nonperiodic: 34 nonperiodic
    full vs partial: the walk died at the first sample for 3 faces, partway for the rest
    residual/tolerance: 33 sit at ~1.3e-6 against an 8.1e-4 tolerance (within tolerance,
        outside the declared parameter range)
conversion losses x13
    conversion-stage histogram: edge_conversion x12, bound_conversion x1
    refusal tags: EdgeCurveConversionFailed x12, AllBoundsCollapsed x1
    provenance established for all 13
```

The partition was derived from the emitted records alone, with no recompile and
no bespoke probe.

## Constitutive vs targeted

**Constitutive — always available on failure:** identity, terminal reason,
stage, surface/topology structure, periodicity/closure, cheap world/UV
extent/rank, actual tolerances, source-edge witness, projection/lift/boundary
refusal witnesses, CDT stage counts, validity certificate, route decisions.

**Targeted replay only (never automatic):** dense 65+ point alternative
interval searches, dense Jacobian maps, alternate-chart exhaustive search, OCCT
comparison, full exact-surface error census, large parameter sweeps, multiple
hypothetical recovery algorithms. The default record tells you which faces
warrant those.

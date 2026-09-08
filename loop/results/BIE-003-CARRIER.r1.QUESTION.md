# QUESTION.md — BIE-003-CARRIER (SPEC_GAP)

## What was done and verified

All in-write-set work is implemented and committed (`e6a5393`):

- `vendor/truck/truck-geometry/src/constructive/intersection_carrier.rs` (new):
  the `CertifiedImplicitIntersectionCurve` carrier — certified sample stream
  (position + orthonormal `IntersectionFrame` + producing `Method`), the
  carrier-local `CarrierUnresolved`/`CarrierCell` witness mirrors of BIE-000,
  refusing typed construction (`Method::None` bare floats, non-finite
  positions, degenerate zero chords, invalid frames; H-2, never panics),
  cumulative-chord-length parameterization so the stored unit frame tangents
  are the exact knot derivatives of the cubic-Hermite continuous evaluation,
  and a tessellation/ledger-facing `polyline()` accessor whose
  `parameter_division` over the domain returns exactly the certified polyline.
- `canonical.rs`: the new `Curve::CertifiedImplicitIntersectionCurve` variant
  threading every macro arm (`derive_curve_method!`,
  `derive_curve_self_method!`), `From`/`TryFrom`, the `lift_up` refusal arm,
  typed-refusal `include` arms on every spline/plane/revolve carrier path, and
  the `canonical_ripple_delegates_all_methods` test.
- `span.rs`: replaced `Surface::SpineFrameSurface(_) => Vec::new()` with real
  sampling/enclosure span records over `(s0, s1, v0, v1)`, plus
  `swept_face_span_covers_windowed_domain`.
- `constructive/mod.rs`: `pub mod intersection_carrier;` + re-export.

With the ripple arms temporarily applied (see below) the five required test
names pass, the full `truck-geometry` lib suite (69 tests) passes, and
`cargo fmt --check`/`cargo clippy -p truck-geometry --all-targets -- -D warnings`
are clean. Two integration tests in
`truck-geometry/tests/constructive_spine_enum.rs` fail identically on the
pristine HEAD (pre-existing, unrelated).

## The gap

The packet's scope decision 1 mandates a new additive canonical `Curve`
variant (a cross-crate fact). That enum ripple reaches exhaustive `match`
arms in files that are **outside the packet's `write_allow`** (and
`truck-shapeops` is explicitly forbidden), so the required "done when" checks
cannot pass in-slot:

1. `vendor/truck/truck-geometry/src/recognize.rs` — same crate, not in the
   write set, yet `cargo test -p truck-geometry` cannot even compile without
   it:
   - `recognize_curve` (`match c`), arm verbatim:
     `| Curve::SpineFrameCurve(_) => CanonicalCarrierWitness::Unrecognized,`
   - `recognize_surface` / extruded `match extruded.entity_curve()`, arm
     verbatim:
     `| Curve::SpineFrameCurve(_) => CanonicalCarrierWitness::Unrecognized,`
2. `vendor/truck/truck-modeling/src/cad.rs` (needed for
   `cargo check -p truck-shapeops`):
   - `edge_enclosure`, arm verbatim:
     `| Curve::SpineFrameCurve(_) => Err(Refusal::UnsupportedEnvelope(\n  EnvelopeCase::NonCanonicalCarrier,\n)),`
   - `check_profile_curve`, same grouped refusal arm.
3. `vendor/truck/truck-shapeops/src/section.rs` (`curve_box`) — explicitly
   forbidden to edit, yet `cargo check -p truck-shapeops` compiles it; arm
   verbatim:
   `| Curve::SpineFrameCurve(_) => Err(non_canonical()),`
4. `vendor/truck/truck-stepio/src/out/geometry.rs` (two sites, discovered by
   the downstream check) — also outside the write set.

Each site only needs the new variant added to the existing grouped
refusal/unrecognized arm:
`| Curve::CertifiedImplicitIntersectionCurve(_)`. That is exactly the
same-class edit the landed `Curve::SpineFrameCurve` variant required in these
files (commit `c8bd3ef`). Because none of these files may be edited under
`write_allow`, the packet is not completable as dispatched: this is the
packet's own stop-condition class ("the canonical ripple reaches an arm that
cannot delegate" / a genuinely missing capability), reported as SPEC_GAP
rather than improvised out-of-scope edits.

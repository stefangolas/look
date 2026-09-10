# CONTEXT.md - mechanically generated from the tree at dispatch time.
# Signatures, callers, tests only. No claims. Regenerated per dispatch.

## WRITE: truck123d/Cargo.toml

## WRITE: truck123d/src/binding.rs (MISSING at dispatch time)

## WRITE: truck123d/src/marshal.rs
L39    enum     ExceptionClass - The two-class Python exception hierarchy (PB-000 §2):
L52    struct   RefusedPayload - The attributes a `Refused` exception instance carries, exactly as the
L92    struct   Ledger - The κ ledger of an `Unresolved` exception: the budget the kernel published
L104   struct   UnresolvedPayload - The attributes an `Unresolved` exception instance carries (PB-000 §2):
L115   struct   Marshaled - The marshaled form of one `Refusal`: the typed class tag, a human message,
L127   enum     MarshaledPayload - The typed payload union: exactly the two classes of the frozen mapping.
L134   impl     Marshaled
L138   fn       from_refusal - Marshals a landed `Refusal` against the frozen mapping. The match is
L291   fn       envelope_case_name - `EnvelopeCase` → snake_case name. Exhaustive; the six payloads of §2.
L303   fn       witness_name - `UnresolvedWitness` → snake_case name. Exhaustive; the five witnesses of §2.
L314   fn       prop_name - `Prop` → snake_case name. Exhaustive over the landed property set.
L334   fn       truth_name - `Truth` → snake_case name.
L344   fn       collapse_reason_name - `CollapseReason` → snake_case name.
L352   fn       method_name - `Method` → snake_case name.
L443   fn       ledger_from_budget - Convenience: the full `Ledger` of a kernel `Budget`.

## WRITE: truck123d/src/python.rs
L23    struct   PyTruckSolid - Opaque kernel-solid handle (PB-004 scope decision 2): no kernel type is
L28    impl     PyTruckSolid
L48    fn       marshal_refused - Builds a `Refused` exception instance carrying the serde payload of a
L60    fn       marshal_unresolved - Builds an `Unresolved` exception instance carrying the serde payload of an
L75    fn       kernel_evidence_compose - The canonical kernel call-slot exposed to Python (PB-005 entries will

## READ: loop/audits/DOOR_GAP_AUDIT-2026-09-09.md

## READ: docs/CONSTRUCTIVE_GEOMETRY_KERNEL_SPEC_V2.md
L119   enum     ClaimVerdict - A proposition about an object that already exists.
L126   type     Construction - The outcome of an attempt to construct an object.
L128   struct   Refusal
L155   trait    CertifiedPatch
L167   trait    CertifiedPatchC2 - Required by R2, R7, and the contact classifier. NOT required by R1 tracing,
L173   trait    CertifiedPatchC3 - Required only by the A2 cusp classifier. Takes a BOX, not a point:
L277   struct   Frame3
L280   struct   SpineFrameRecipe
L290   enum     Spine
L312   enum     FrameLaw
L336   enum     ProfileLaw
L348   enum     SamplingPolicy
L384   struct   EdgeSampleLedger
L399   struct   ManifoldDiagnostics
L422   struct   SpineFrameSurface
L752   struct   ContactCert
L836   struct   Canal
L939   struct   TopologyClaim
L945   fn       certify_claimed

## READ: docs/CERTIFICATE_MAPPING.md

## READ: vendor/truck/truck-evidence/src/contact/ (MISSING at dispatch time)

## READ: vendor/truck/truck-certified/src/ (MISSING at dispatch time)

## CALLER SITES (grep of defining names outside their file)
Ledger  vendor\truck\truck-certified\src\construct\bie\mod.rs:43  //! PL-at-tessellation policy (`EdgeSampleLedger`-compatible; truck-meshalgo


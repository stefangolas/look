import subprocess, sys, re, os

os.chdir(r"C:\Users\stefa\look")

# (file, pattern, expected_count) -- exactly as stated in the 9 packet anchor tables
CHECKS = [
    # BIE-000
    ("vendor/truck/truck-base/src/evidence.rs", r"pub enum Refusal", 1),
    ("vendor/truck/truck-base/src/evidence.rs", r"NumericallyUnresolved", 2),
    ("vendor/truck/truck-evidence/src/contact/mod.rs", r"pub enum BoundedStratum", 1),
    ("vendor/truck/truck-geometry/src/constructive/sweep_surface.rs", r"impl ParametricSurface for SpineFrameSweep", 1),
    ("vendor/truck/truck-certified/src/construct/mod.rs", r"^pub mod", 26),
    # BIE-001
    ("vendor/truck/truck-certified/src/formal/exact.rs", r"pub struct CertifiedInterval", 1),
    ("vendor/truck/truck-certified/src/hull.rs", r"pub fn hull_bernstein_2d", 1),
    ("vendor/truck/truck-certified/src/lib.rs", r"^pub mod", 14),
    ("vendor/truck/truck-certified/src/contract.rs", r"pub struct IntervalEnclosure", 1),
    # BIE-002
    ("vendor/truck/truck-evidence/src/num/krawczyk.rs", r"pub trait KrawczykSystem<const N: usize>", 1),
    ("vendor/truck/truck-evidence/src/num/krawczyk.rs", r"pub fn krawczyk<const N: usize>", 1),
    ("vendor/truck/truck-certified/src/kernel/engine.rs", r"pub fn krawczyk_c1_n4", 1),
    ("vendor/truck/truck-certified/src/formal/exact.rs", r"pub struct Expansion", 1),
    ("vendor/truck/truck-evidence/src/num/mod.rs", r"^pub mod", 3),
    # BIE-003
    ("vendor/truck/truck-geometry/src/canonical.rs", r"IntersectionCurve\(IntersectionCurve<Box<Curve>, Box<Surface>, Box<Surface>>\)", 1),
    ("vendor/truck/truck-geometry/src/span.rs", r"Surface::SpineFrameSurface\(_\) => Vec::new\(\)", 1),
    ("vendor/truck/truck-geometry/src/constructive/sweep_surface.rs", r"pub struct SpineFrameSweep", 1),
    ("vendor/truck/truck-geometry/src/constructive/mod.rs", r"^pub mod", 1),
    ("vendor/truck/truck-base/src/bvh.rs", r"pub fn candidate_pairs\(", 1),
    # BIE-004
    ("vendor/truck/truck-certified/src/kernel/engine.rs", r"pub fn krawczyk_c1_n4", 1),
    ("vendor/truck/truck-evidence/src/num/krawczyk.rs", r"pub fn krawczyk<const N: usize>", 1),
    # BIE-005
    ("vendor/truck/truck-geometry/src/arrange.rs", r"enum Carrier2D", 1),
    ("vendor/truck/truck-geometry/src/arrange.rs", r"pub struct ArrRegion", 1),
    ("vendor/truck/truck-geometry/src/arrange.rs", r"^pub fn arrange\(profile", 1),
    # BIE-006
    ("vendor/truck/truck-evidence/src/contact/mod.rs", r"CanonicalCarrierWitness::Unrecognized =>", 1),
    ("vendor/truck/truck-shapeops/src/boolean/classify.rs", r"pub fn classify_fragments", 1),
    ("vendor/truck/truck-shapeops/src/boolean/mod.rs", r"pub fn fragment_decision", 1),
    ("vendor/truck/truck-shapeops/src/boolean/assemble.rs", r"pub fn boolean\(", 1),
    ("vendor/truck/truck-topology/src/entity_id.rs", r"pub enum EntityId", 1),
    # BIE-007
    ("vendor/truck/truck-topology/src/manifold.rs", r"pub fn diagnose", 1),
    ("vendor/truck/truck-shapeops/src/lib.rs", r"^pub mod", 5),
    # SEM-PCURVE-MASTER-001-FIX
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"fn sub_parse_curve3d", 1),
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"Curve3D => Self::sub_parse_curve3d\(&c.curve_3d", 1),
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"PcurveS1 =>", 2),
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"PcurveS2 =>", 2),
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"impl TryFrom<&SurfaceCurve> for Curve3D", 1),
    ("vendor/truck/truck-stepio/src/in/mod.rs", r"master_representation", 3),
    # new-path absence assertions made in packets
    ("vendor/truck/truck-shapeops/tests/bie_gates.rs", "__ABSENT__", 0),
    ("vendor/truck/truck-stepio/tests/sem_pcurve_master_001.rs", "__ABSENT__", 0),
]

fails = 0
for path, pat, expect in CHECKS:
    if pat == "__ABSENT__":
        ok = not os.path.exists(path)
        if not ok:
            print(f"FAIL {path}: expected ABSENT, exists")
            fails += 1
        else:
            print(f"ok   {path}: absent as expected")
        continue
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
    n = len(re.findall(pat, text, re.MULTILINE))
    status = "ok  " if n == expect else "FAIL"
    if n != expect:
        fails += 1
    print(f"{status} {path} :: /{pat}/ = {n} (expect {expect})")

print()
print(f"{fails} mismatches" if fails else "ALL ANCHORS HOLD")
sys.exit(1 if fails else 0)

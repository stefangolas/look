"""Machine-check for BG-FID-003-r2 witness numbers (session-18 rule).

Every number the packet quotes is printed here through the packet's own
formulas. A number that disagrees with the packet is a stop condition for
dispatch.
"""
import math

EPS = 0.05
R = 2.0

print("== (ii) sinusoid: extreme tangent deviation, |cos| form ==")
A = 0.04
OMEGA = 4000.0
tan_phi = A * OMEGA / R  # at omega*t = 0, radius R + a*sin(0) = R
abs_cos_extreme = 1.0 / math.sqrt(1.0 + tan_phi**2)
s = EPS / R  # = eps / tube_scale (circle: tube_scale = R)
print(f"atan(80)               = {math.atan(80.0):.10f} rad (packet: ~1.5583)")
print(f"pi/2 - asin(eps/R)     = {math.pi/2 - math.asin(EPS/R):.10f} rad (packet: ~1.5458)")
print(f"|cos| at extreme       = 1/sqrt(1+80^2) = {abs_cos_extreme:.10f}")
print(f"s = eps/tube_scale     = {s:.10f}")
assert abs_cos_extreme < s, "extreme |cos| must be BELOW s for the violation to fire"
print(f"violation margin s-|cos| = {s - abs_cos_extreme:.6f}  OK")

# how fast |cos| rises away from the extreme (decides (ii) refinement depth)
def abs_cos_dev(t):
    # deviation angle between approx tangent and exact (circle) tangent at t
    num = A * OMEGA * math.cos(OMEGA * t)
    den = R + A * math.sin(OMEGA * t)
    return abs(den) / math.sqrt(num * num + den * den)
# (i)-scale cell width for cross-reference (derivation in the tractability section)
X2_MAX = A * OMEGA * OMEGA + 2 * A * OMEGA + R + A
W = EPS / 4
h_i = math.sqrt(8.0 * W / X2_MAX)

# decisive width: the cell around the extreme whose whole |cos| range sits <= s
# (the violation arm certifies "all pairs in the cell pair have |cos| <= s")
for k in range(1, 20):
    h = 10.0 ** (-k)
    lo = abs_cos_dev(-h / 2)
    hi = abs_cos_dev(h / 2)
    if max(lo, hi) <= s:
        print(f"  decisive at width 1e-{k}: |cos| range [{lo:.6f}, {hi:.6f}] <= s = {s}")
        print(f"  (i)-scale width h = {h_i:.2e} -> (ii) needs "
              f"{max(0, math.ceil(math.log2(h_i / 10.0**(-k))))} extra levels, or none if h_i <= 1e-{k}")
        break
    else:
        print(f"  width 1e-{k}: range [{lo:.6f}, {hi:.6f}] straddles s -> subdivide")

print()
print("== (i) sinusoid tractability: cell count at enclosure budget w = eps/4 ==")
W = EPS / 4
X2_MAX = A * OMEGA * OMEGA + 2 * A * OMEGA + R + A
h = math.sqrt(8.0 * W / X2_MAX)
n = math.ceil(2 * math.pi / h)
print(f"|X''| <= {X2_MAX:.0f}, h = {h:.3e}, N ~ {n} cells (BVH mandatory, O(N*M) forbidden)")
assert n < 3e4

print()
print("== separation component: circle, parameter gap G = pi ==")
for gap in (math.pi,):
    sig = 2 * R * math.sin(gap / 2)  # chord at parameter gap exactly = gap (worst pair)
    print(f"R={R}, G={gap:.6f}: sigma = 2R sin(G/2) = {sig:.10f}, sigma/2 = {sig/2:.10f}, curv = {R}")
    assert abs(sig - 4.0) < 1e-12 and abs(sig / 2 - R) < 1e-12
print("tube_scale = min(R, sigma/2) = R = 2.0; gate 2*eps = 0.1 < 2 OK")

print()
print("== separation helper soundness witness: ellipse a=2, b=0.5, G=2.0 ==")
EA, EB = 2.0, 0.5
G = 2.0
SPAN = 2 * math.pi

def ex(t):
    return (EA * math.cos(t), EB * math.sin(t))

best = (None, None, None)
M = 4000
for i in range(M):
    t = i * SPAN / M
    for j in range(i, M):
        u = j * SPAN / M
        # parameter gap, wrapped (Closed)
        d = abs(t - u)
        d = min(d, SPAN - d)
        if d >= G:
            p, q = ex(t), ex(u)
            dist = math.hypot(p[0] - q[0], p[1] - q[1])
            if best[0] is None or dist < best[0]:
                best = (dist, t, u)
print(f"brute-force sigma over {M}x{M} grid at G={G}: {best[0]:.8f} (at t={best[1]:.4f}, u={best[2]:.4f})")
print(f"helper must return <= {best[0]:.8f} (+tiny slack) and > usefulness floor 0.75 (floor strictly below true value)")

print()
print("== gate test with constructed components (the hairpin isolation) ==")
curv, sep = 10.0, 0.12
tube = min(curv, 0.5 * sep)
print(f"tube_scale = min({curv}, {sep}/2) = {tube}; 2*eps = {2*EPS}; {2*EPS} >= {tube} -> refusal fires")
print(f"attributable to separation: 2*eps = {2*EPS} < curv = {curv} OK")
assert 2 * EPS >= tube and 2 * EPS < curv

print()
print("== (iii) kind-mismatch witness: near-full-circle Open approx ==")
DELTA = 0.001  # parameter shortfall
seam_gap = 2 * R * math.sin(DELTA / 2)
print(f"R={R}, span [0, 2pi-{DELTA}]: endpoint gap = 2R sin(delta/2) = {seam_gap:.8f}")
print(f"{seam_gap:.8f} < eps = {EPS} -> geometric endpoint check PASSES; only the kind gate catches it")
assert seam_gap < EPS

print()
print("== test 5: coarse radius refusal ==")
r5 = 0.08
tube5 = min(r5, 0.5 * (2 * r5 * math.sin(math.pi / 2)))
print(f"R={r5}: curv = {r5}, sigma/2 = {0.5*2*r5:.4f}, tube_scale = {tube5:.4f}; 2*eps = 0.1 >= {tube5:.4f} -> refuse")
assert 2 * EPS >= tube5

print()
print("== line pair: extended-real identity ==")
print("curv = +inf (straight), sigma = +inf (no pair at G over span 1), tube_scale = +inf;")
print(f"s = eps/inf = 0; pass needs |cos| > 0; parallel tangents give abs dot = |a||e| > 0 OK (either orientation)")

print()
print("ALL CHECKS PASS")

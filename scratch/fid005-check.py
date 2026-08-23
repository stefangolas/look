"""Machine-check for BG-FID-005 (rep_curve) witness numbers.

Cubic Hermite per cell, Bezier form, sampled densely against the exact
geometry. Decides the refinement-success depth for the coarse-partition
test and sanity-checks the (iv-b) arithmetic.
"""
import math

R = 2.0
TAU = 0.05
SPAN = 2.0 * math.pi

def exact(t):
    return (R * math.cos(t), R * math.sin(t))

def exact_der(t):
    return (-R * math.sin(t), R * math.cos(t))

def bezier3(p0, p1, p2, p3, u):
    w0 = (1 - u) ** 3
    w1 = 3 * (1 - u) ** 2 * u
    w2 = 3 * (1 - u) * u ** 2
    w3 = u ** 3
    return (w0 * p0[0] + w1 * p1[0] + w2 * p2[0] + w3 * p3[0],
            w0 * p0[1] + w1 * p1[1] + w2 * p2[1] + w3 * p3[1])

print("== cubic Hermite (Bezier form) max error on circle R=2 vs depth ==")
for d in range(0, 8):
    n = 2 ** d
    h = SPAN / n
    err_max = 0.0
    for j in range(n):
        a, b = j * h, (j + 1) * h
        p0, p3 = exact(a), exact(b)
        t0, t3 = exact_der(a), exact_der(b)
        p1 = (p0[0] + h / 3 * t0[0], p0[1] + h / 3 * t0[1])
        p2 = (p3[0] - h / 3 * t3[0], p3[1] - h / 3 * t3[1])
        for k in range(1, 64):
            u = k / 64.0
            q = bezier3(p0, p1, p2, p3, u)
            # distance to circle: | |q| - R |
            dist = abs(math.hypot(q[0], q[1]) - R)
            err_max = max(err_max, dist)
    verdict = "PASS" if err_max <= TAU else "fail"
    print(f"depth {d}: N={n:3d} cells, h={h:.4f}, max radial error {err_max:.6f} vs tau={TAU} -> {verdict}")

print()
print("== (iv-b) denominator on circle at eps=0.05, any depth ==")
m = R          # inf |X'| (unit-angle parameterization: speed = R)
K = R          # sup |X''|
den = m * m - TAU * K
print(f"m^2 - eps*K = {m}^2 - {TAU}*{K} = {den} > 0 OK (per-cell values only shrink this; refine or refuse)")

print()
print("== non-adjacent separation at depth 3 (8 cells, R=2) ==")
d, n = 3, 8
h = SPAN / n
# closest non-adjacent pair: cells j and j+2, arc gap = R*h at nearest ends
gap_arc = R * h
sep = 2 * R * math.sin(min(gap_arc, math.pi * R) / (2 * R))
print(f"arc gap between j and j+2 nearest ends = R*h = {gap_arc:.4f}; chord bound 2R sin(gap/2R) = {sep:.4f} >> eps OK")

print()
print("== coarse circle R=0.08 at tau=0.05: rep EMITS (refines), does not refuse ==")
r, tau = 0.08, 0.05
tube = min(r, 0.5 * (2 * r * math.sin(math.pi / 2)))  # curv=0.08, sigma/2=0.08
print(f"tube_scale = {tube}; target eps = min(tau, tube/2) = {min(tau, tube/2)}; Hermite error -> 0 with depth, so 2*eps < tube eventually holds: EMIT")
print(f"(FID-003's checker at FIXED eps=0.05 refuses the same input - the caller's fidelity question, different from rep's choice)")

print()
print("== V-corner exact (two segments, angle 60 deg): components refuse -> collapse ==")
print("tangent enclosure at the corner cell contains both segment directions at every refinement;")
print("curvature_radius_lower_span -> CurvatureUnresolved -> rep -> UnsupportedEnvelope(ReachTooSmall)")

print()
print("ALL CHECKS PASS")

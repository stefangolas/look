"""Probe v3: recover the ACTUAL station parameters v_i of OCCT ThruSections
by 1-D minimization (iso-vs-section deviation), then fit candidate laws.

Answers: which station law (uniform/chord/sqrt-chord/centripetal), and
whether sections are hit exactly (dev ~ tolerance) or approximated.
"""

import math
import sys

sys.path.insert(0, "scratch")

from probe_thrussections_law import (
    bspline_faces,
    dist_to_polyline,
    rounded_section_points,
    sample_curve,
    section_wire,
)

from OCP.BRepOffsetAPI import BRepOffsetAPI_ThruSections


def recover_station_vs(bs, section_polys, v0, v1, n_scan=160):
    out = []
    for poly in section_polys:
        def dev(v):
            iso = bs.VIso(v)
            u0, u1 = iso.FirstParameter(), iso.LastParameter()
            pts = [
                (lambda p: (p.X(), p.Y(), p.Z()))(
                    iso.Value(u0 + (u1 - u0) * k / 149)
                )
                for k in range(150)
            ]
            return max(dist_to_polyline(p, pts) for p in poly[:40])

        scan = [dev(v0 + (v1 - v0) * k / n_scan) for k in range(n_scan + 1)]
        k0 = min(range(n_scan + 1), key=lambda k: scan[k])
        lo = v0 + (v1 - v0) * max(0, k0 - 2) / n_scan
        hi = v0 + (v1 - v0) * min(n_scan, k0 + 2) / n_scan
        for _ in range(60):
            m1 = lo + (hi - lo) / 3
            m2 = hi - (hi - lo) / 3
            if dev(m1) < dev(m2):
                hi = m2
            else:
                lo = m1
        v_star = (lo + hi) / 2
        out.append((v_star, dev(v_star)))
    return out


def main():
    for N in (4, 5, 8, 10, 16, 24):
        xs = []
        wires = []
        polys = []
        for k in range(N):
            x = 0.0 if k == 0 else xs[-1] + (60.0 + 40.0 * ((k * 7) % 5))
            xs.append(x)
            pts = rounded_section_points(
                x, 0.0, 400.0, 300.0 + 2.0 * k, 200.0 + 1.5 * k, 40.0 + k
            )
            w, cv = section_wire(pts)
            wires.append(w)
            polys.append(sample_curve(cv))
        ts = BRepOffsetAPI_ThruSections(True, False, 1e-7)
        for w in wires:
            ts.AddWire(w)
        ts.CheckCompatibility(False)
        ts.Build()
        bs = bspline_faces(ts.Shape())[0]
        v0, v1 = bs.VKnot(1), bs.VKnot(bs.LastVKnotIndex())
        rec = recover_station_vs(bs, polys, v0, v1)
        vs = [r[0] for r in rec]
        devs = [r[1] for r in rec]
        rec_n = [(v - vs[0]) / (vs[-1] - vs[0]) for v in vs]
        total = xs[-1] - xs[0]
        cs = [math.sqrt(xs[j + 1] - xs[j]) for j in range(N - 1)]
        tot = sum(cs)
        cands = {
            "uniform": [i / (N - 1) for i in range(N)],
            "chord": [(xs[i] - xs[0]) / total for i in range(N)],
            "sqrt_chord": [math.sqrt(xs[i] - xs[0]) / math.sqrt(total) for i in range(N)],
            "centripetal": [0.0] + [sum(cs[:i]) / tot for i in range(1, N)],
        }
        fits = {
            name: max(abs(a - b) for a, b in zip(rec_n, cv))
            for name, cv in cands.items()
        }
        bestlaw = min(fits, key=fits.get)
        print(f"N={N:>2}  vdeg={bs.VDegree()}  max section-hit dev={max(devs):.2e}")
        print(f"      recovered v (normalized): {[round(v, 4) for v in rec_n]}")
        print(
            "      law residuals: "
            + ", ".join(f"{k}={v:.2e}" for k, v in fits.items())
        )
        print(f"      BEST: {bestlaw}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

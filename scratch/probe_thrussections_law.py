"""Probe v2: pin the OCCT ThruSections station law.

Questions:
  Q1  station law: uniform-in-index (v_i = i/(N-1)) vs chord-length vs
      sqrt-chord?  Decided by evaluating v-isocurves at each candidate
      assignment and measuring geometric deviation from the input sections.
  Q2  v-degree law vs N: single global polynomial (degree N-1)? capped?
      piecewise?  N-sweep 4..24.
  Q3  are input sections hit EXACTLY (knot-insertion compatibility) or only
      within tolerance (re-approximation)?  The max deviation answers it.
  Q4  ruled lofts: per-interval linear with uniform v per interval?

Synthetic closed sections only; no corpus data touched.
"""

import math
import sys

from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepBuilderAPI import (
    BRepBuilderAPI_MakeEdge,
    BRepBuilderAPI_MakeWire,
)
from OCP.BRepOffsetAPI import BRepOffsetAPI_ThruSections
from OCP.GCPnts import GCPnts_QuasiUniformDeflection
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE
from OCP.TopoDS import TopoDS

TOL = 1e-7


def rounded_section_points(cx, cy, cz, hw, hh, corner, n_per_side=8):
    pts = []
    x, y0, z0 = cx, cy, cz
    def arc(cy_, cz_, r, a0, a1):
        for i in range(1, n_per_side + 1):
            a = a0 + (a1 - a0) * i / n_per_side
            pts.append((x, cy_ + r * math.cos(a), cz_ + r * math.sin(a)))
    pts.append((x, y0 + hw, z0 - hh + corner))
    for i in range(1, n_per_side):
        t = i / n_per_side
        pts.append((x, y0 + hw, z0 - hh + corner + t * (2 * (hh - corner))))
    arc(y0 + hw - corner, z0 + hh - corner, corner, 0.0, math.pi / 2)
    for i in range(1, n_per_side):
        t = i / n_per_side
        pts.append((x, y0 + hw - corner - t * (2 * (hw - corner)), z0 + hh))
    arc(y0 - hw + corner, z0 + hh - corner, corner, math.pi / 2, math.pi)
    for i in range(1, n_per_side):
        t = i / n_per_side
        pts.append((x, y0 - hw, z0 + hh - corner - t * (2 * (hh - corner))))
    arc(y0 - hw + corner, z0 - hh + corner, corner, math.pi, 1.5 * math.pi)
    for i in range(1, n_per_side):
        t = i / n_per_side
        pts.append((x, y0 - hw + corner + t * (2 * (hw - corner)), z0 - hh))
    arc(y0 + hw - corner, z0 - hh + corner, corner, 1.5 * math.pi, 2.0 * math.pi)
    return pts[:-1]


def section_wire(pts):
    from OCP.GeomAPI import GeomAPI_Interpolate
    from OCP.TColgp import TColgp_HArray1OfPnt
    from OCP.gp import gp_Pnt
    n = len(pts)
    arr = TColgp_HArray1OfPnt(1, n)
    for i, (x, y, z) in enumerate(pts):
        arr.SetValue(i + 1, gp_Pnt(x, y, z))
    interp = GeomAPI_Interpolate(arr, True, 1e-9)
    interp.Perform()
    if not interp.IsDone():
        raise RuntimeError("interpolate failed")
    edge = BRepBuilderAPI_MakeEdge(interp.Curve()).Edge()
    return BRepBuilderAPI_MakeWire(edge).Wire(), interp.Curve()


def sample_curve(curve, n=400):
    u0, u1 = curve.FirstParameter(), curve.LastParameter()
    out = []
    for k in range(n):
        p = curve.Value(u0 + (u1 - u0) * k / (n - 1))
        out.append((p.X(), p.Y(), p.Z()))
    return out


def dist_to_polyline(p, poly):
    best = float("inf")
    px, py, pz = p
    for i in range(len(poly)):
        qx, qy, qz = poly[i]
        d = math.dist((px, py, pz), (qx, qy, qz))
        if d < best:
            best = d
    return best


def bspline_faces(shape):
    exp = TopExp_Explorer(shape, TopAbs_FACE)
    out = []
    while exp.More():
        face = TopoDS.Face_s(exp.Current())
        ad = BRepAdaptor_Surface(face)
        if "BSpline" in str(ad.GetType()):
            out.append(ad.BSpline())
        exp.Next()
    return out


def law_value(law, xs, i, n):
    if law == "uniform":
        return i / (n - 1)
    total = xs[-1] - xs[0]
    if law == "chord":
        return (xs[i] - xs[0]) / total
    if law == "sqrt_chord":
        return math.sqrt(xs[i] - xs[0]) / math.sqrt(total)
    if law == "centripetal":
        return sum(math.sqrt(xs[j + 1] - xs[j]) for j in range(i)) / sum(
            math.sqrt(xs[j + 1] - xs[j]) for j in range(n - 1)
        )
    raise ValueError(law)


def main():
    print(f"{'N':>3} {'vdeg':>4} {'vkn':>4} {'ufaces':>6}  best-law  max-dev(section hit)")
    laws = ("uniform", "chord", "sqrt_chord", "centripetal")
    for N in (4, 5, 6, 8, 10, 12, 16, 20, 24):
        xs = []
        wires = []
        curves = []
        section_polys = []
        for k in range(N):
            # non-uniform spacing preserved from probe v1 style
            x = 0.0 if k == 0 else xs[-1] + (60.0 + 40.0 * ((k * 7) % 5))
            xs.append(x)
            hw = 300.0 + 2.0 * k
            hh = 200.0 + 1.5 * k
            pts = rounded_section_points(x, 0.0, 400.0, hw, hh, 40.0 + k)
            w, cv = section_wire(pts)
            wires.append(w)
            curves.append(cv)
            section_polys.append(sample_curve(cv))

        ts = BRepOffsetAPI_ThruSections(True, False, TOL)
        for w in wires:
            ts.AddWire(w)
        ts.CheckCompatibility(False)
        ts.Build()
        if not ts.IsDone():
            print(f"{N:>3}  BUILD FAILED")
            continue
        faces = bspline_faces(ts.Shape())
        if len(faces) != 1:
            print(f"{N:>3}  {len(faces)} bspline faces (unexpected)")
            continue
        bs = faces[0]
        vdeg = bs.VDegree()
        vkn = bs.LastVKnotIndex()
        v0, v1 = bs.VKnot(1), bs.VKnot(vkn)
        u0, u1 = bs.UKnot(1), bs.UKnot(bs.LastUKnotIndex())

        best = None
        for law in laws:
            maxdev = 0.0
            for i in range(N):
                v = v0 + (v1 - v0) * law_value(law, xs, i, N)
                iso = bs.VIso(v)
                poly = sample_curve(iso, n=200)
                for p in section_polys[i][:60]:
                    d = dist_to_polyline(p, poly)
                    if d > maxdev:
                        maxdev = d
            if best is None or maxdev < best[1]:
                best = (law, maxdev)
        print(f"{N:>3} {vdeg:>4} {vkn:>4} {len(faces):>6}  {best[0]:>9}  {best[1]:.3e}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

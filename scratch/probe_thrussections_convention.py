"""Probe: characterize OCC BRepOffsetAPI_ThruSections station parameterization.

Synthetic closed sections (rounded-rectangle-ish closed BSplines) placed at
NON-uniform x stations. Dump the resulting surface's structure: face count,
u/v degrees, u/v knot vectors. The v-knot vector is the datum: it reveals how
OCC assigned the v parameter to each station (uniform / chord-length /
sqrt-chord), and what degree it interpolated with.

No corpus file, no reference, no manifest is touched. This is measurement
infrastructure for the MONO-CLOSURE program booking.
"""

import math
import sys

from OCP.BRepBuilderAPI import (
    BRepBuilderAPI_MakeEdge,
    BRepBuilderAPI_MakeWire,
)
from OCP.BRepOffsetAPI import BRepOffsetAPI_ThruSections
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.Geom import Geom_BSplineSurface
from OCP.GeomAPI import GeomAPI_Interpolate
from OCP.TColgp import TColgp_HArray1OfPnt
from OCP.gp import gp_Pnt


def rounded_section_points(cx, cy, cz, hw, hh, corner, n_per_side=8):
    """Deterministic closed outline: rectangle with rounded corners,
    sampled CCW starting at (cx+hw, cy). Returns list[(x, y, z)] with
    x = cx constant (planar section in the y-z plane at x=cx)."""
    # four straight sides + four corner arcs, 90 deg each
    pts = []
    x, y0, z0 = cx, cy, cz
    # side segments as straight runs, corner arcs as quarter circles
    def arc(cy_, cz_, r, a0, a1):
        for i in range(1, n_per_side + 1):
            a = a0 + (a1 - a0) * i / n_per_side
            pts.append((x, cy_ + r * math.cos(a), cz_ + r * math.sin(a)))
    # start right side going up
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
    # periodic interpolation forbids duplicating the closing point
    return pts[:-1]


def closed_spline_wire(points):
    """Interpolate a closed C2 BSpline through the points, return a wire."""
    n = len(points)
    arr = TColgp_HArray1OfPnt(1, n)
    for i, (x, y, z) in enumerate(points):
        arr.SetValue(i + 1, gp_Pnt(x, y, z))
    interp = GeomAPI_Interpolate(arr, True, 1e-9)  # periodic (closed)
    interp.Perform()
    if not interp.IsDone():
        raise RuntimeError("interpolate failed")
    curve = interp.Curve()
    edge = BRepBuilderAPI_MakeEdge(curve).Edge()
    wire = BRepBuilderAPI_MakeWire(edge).Wire()
    return wire, curve


def main():
    # non-uniform station spacing on purpose: v-knot deltas will reveal the law
    stations = [
        # (x, hw, hh, corner)
        (0.0,   300.0, 200.0, 40.0),
        (100.0, 310.0, 205.0, 42.0),
        (300.0, 330.0, 215.0, 46.0),
        (600.0, 350.0, 225.0, 50.0),
        (1100.0, 340.0, 220.0, 48.0),
    ]
    wires = []
    curves = []
    for x, hw, hh, c in stations:
        pts = rounded_section_points(x, 0.0, 400.0, hw, hh, c)
        w, cv = closed_spline_wire(pts)
        wires.append(w)
        curves.append(cv)

    for ruled in (False, True):
        ts = BRepOffsetAPI_ThruSections(True, ruled, 1e-6)  # solid=True
        for w in wires:
            ts.AddWire(w)
        ts.CheckCompatibility(False)
        ts.Build()
        if not ts.IsDone():
            print(f"ruled={ruled}: ThruSections NOT DONE")
            continue
        shape = ts.Shape()
        print(f"=== ruled={ruled} ===")
        # walk faces, find the BSpline surface(s)
        from OCP.TopExp import TopExp_Explorer
        from OCP.TopAbs import TopAbs_FACE
        from OCP.TopoDS import TopoDS
        exp = TopExp_Explorer(shape, TopAbs_FACE)
        iface = 0
        while exp.More():
            face = TopoDS.Face_s(exp.Current())
            iface += 1
            adaptor = BRepAdaptor_Surface(face)
            surf = adaptor.Surface()
            st = adaptor.GetType()
            print(f" face {iface}: type={st}")
            is_bspline = "BSpline" in str(st)
            if is_bspline:
                bs = adaptor.BSpline()
                udeg, vdeg = bs.UDegree(), bs.VDegree()
                vkn = [bs.VKnot(i) for i in range(1, bs.LastVKnotIndex() + 1)]
                ukn = [bs.UKnot(i) for i in range(1, bs.LastUKnotIndex() + 1)]
                print(f"  degree u={udeg} v={vdeg}")
                print(f"  v-knots ({len(vkn)}): {[round(v, 6) for v in vkn]}")
                print(f"  u-knots ({len(ukn)}): {[round(u, 6) for u in ukn[:12]]}{'...' if len(ukn) > 12 else ''}")
                # v parametrization of the stations:
                # section i sits at v = some value; with periodic-off solid,
                # first/last sections at v0/vN. Compare deltas to uniform /
                # chord (x-spacing) / sqrt(chord).
                inner = [vkn[i + 1] - vkn[i] for i in range(len(vkn) - 1)]
                print(f"  v-knot deltas: {[round(d, 6) for d in inner]}")
                xs = [stations[j + 1][0] - stations[j][0] for j in range(len(stations) - 1)]
                print(f"  x deltas      : {xs}")
                print(f"  dx  normalized: {[round(d / sum(xs), 6) for d in xs]}")
                if sum(inner) > 0:
                    print(f"  dv  normalized: {[round(d / sum(inner), 6) for d in inner]}")
            exp.Next()
    return 0


if __name__ == "__main__":
    sys.exit(main())

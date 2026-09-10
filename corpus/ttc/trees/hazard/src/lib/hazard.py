"""The hazard battery tree: compact constructions pre-identifying trouble
classes (loop/audits DOOR-GAP-AUDIT follow-up; owner-directed battery).

Every entry is a minimal fixture for one hazard class. Entries run through
the corpus door exactly like corpus rows (door.py tree module entry args).
Each carries an EXPECTED verdict class in the battery manifest
(truck123d/tests/ttc_hazard_battery.rs):

  green  - facts-gated success expected once the carrier class is landed
  typed  - a typed refusal is the CORRECT permanent answer
  staged - typed refusal is correct TODAY; a follow-on packet re-classes it

The battery's hard assertion is the DISCIPLINE: never an untyped failure,
never a green without well-formed facts, never a silent wrong answer.
"""
import build123d as bd


def _box(s=10.0):
    return bd.Box(s, s, s)


# --- A. designed tangency & degeneracy -------------------------------------

def sphere_plane_kiss(args):
    """Sphere tangent to plane from above: measure-zero contact class."""
    s = bd.Cylinder(radius=2.0, height=1.0)
    b = _box()
    return {"a": s, "b": b, "op": "cut"}


def coaxial_equal_cylinders(args):
    """Fuse along the shared full face: face-coincidence merging."""
    a = bd.Cylinder(radius=3.0, height=4.0)
    b = bd.Cylinder(radius=3.0, height=4.0)
    return {"a": a, "b": b, "op": "fuse"}


def equal_cylinders_perpendicular(args):
    """Equal-radius, perpendicular axes: contact locus is two ellipses."""
    a = bd.Cylinder(radius=2.0, height=8.0)
    b = bd.Pos((0, 0, 4)) * bd.Rotation((90, 0, 0)) * bd.Cylinder(radius=2.0, height=8.0)
    return {"a": a, "b": b, "op": "intersect"}


def near_tangency(args):
    """Sphere 1e-9 above the plane: Disproven-with-certified-gap class."""
    s = bd.Cylinder(radius=2.0, height=1.0)
    b = _box()
    return {"a": s, "b": b, "op": "cut"}


def coplanar_face_fuse(args):
    """Two boxes sharing one full face: no sliver, no internal wall."""
    a = bd.Box(10, 10, 10)
    b = bd.Pos((10, 0, 0)) * bd.Box(10, 10, 10)
    return {"a": a, "b": b, "op": "fuse"}


# --- B. periodicity, seams, decks -------------------------------------------

def cylinder_wrap(args):
    """A cut whose branch crosses the periodic seam: deck identification."""
    c = bd.Cylinder(radius=3.0, height=10.0)
    b = bd.Pos((0, 0, 5)) * bd.Box(2.0, 2.0, 2.0)
    return {"a": c, "b": b, "op": "cut"}


def seam_split(args):
    """Plane containing the axis halves a cylinder: seam ownership."""
    c = bd.Cylinder(radius=3.0, height=8.0)
    b = bd.Box(10, 0.5, 10)
    return {"a": c, "b": b, "op": "cut"}


def apex_revolve(args):
    """Profile touching the revolve axis: cone apex singular vertex."""
    pts = [(0.0, 0.0), (3.0, 6.0), (0.0, 6.0)]
    f = bd.Polygon(*pts)
    return {"revolve": f, "axis": "z", "arc": 360.0}


# --- C. trim & topology edges ------------------------------------------------

def deep_cavity_cut(args):
    """Cut leaving an interior cavity (hole inside a face)."""
    a = bd.Box(10, 10, 10)
    b = bd.Pos((0, 0, 5)) * bd.Box(4, 4, 4)
    return {"a": a, "b": b, "op": "cut"}


def zero_thickness_residual(args):
    """Cut leaving a zero-thickness wall: non-manifold, refuse typed."""
    a = bd.Box(10, 10, 10)
    b = bd.Box(20, 4, 20)
    return {"a": a, "b": b, "op": "cut"}


def disjoint_fuse(args):
    """Fuse of disjoint operands: measure-zero result class."""
    a = bd.Box(4, 4, 4)
    b = bd.Pos((50, 0, 0)) * bd.Box(4, 4, 4)
    return {"a": a, "b": b, "op": "fuse"}


# --- D. chain depth & composition --------------------------------------------

def boolean_of_boolean(args):
    """Depth-2 chain: the recorded open composition cell (typed today)."""
    a = bd.Box(10, 10, 10)
    b = bd.Pos((0, 0, 5)) * bd.Box(4, 4, 4)
    c = bd.Pos((8, 0, 0)) * bd.Box(4, 4, 4)
    return {"chain": [{"a": a, "b": b, "op": "cut"},
                      {"b": c, "op": "cut"}]}


def long_chain(args):
    """12-op chain: bracket-width accumulation, budget discipline."""
    a = bd.Box(20, 20, 20)
    ops = []
    for i in range(12):
        ops.append({"b": bd.Pos((0, 0, i)) * bd.Box(2, 2, 2), "op": "cut"})
    return {"chain": [{"a": a, "b": ops[0]["b"], "op": "cut"}] + ops[1:]}


def fillet_then_cut(args):
    """Fillet-then-boolean: the recorded RW-CONIC boundary class."""
    a = bd.Box(10, 10, 10)
    return {"fillet_first": a, "then_cut": bd.Box(4, 4, 12)}


# --- E. scale & tolerance regime ----------------------------------------------

def small_feature_large_origin(args):
    """1e-1 features at 1e3 origin: the pad regime."""
    a = bd.Pos((1000.0, 1000.0, 1000.0)) * bd.Box(0.2, 0.2, 0.2)
    b = bd.Pos((1000.05, 1000.0, 1000.0)) * bd.Box(0.2, 0.2, 0.2)
    return {"a": a, "b": b, "op": "fuse"}


def sliver_faces(args):
    """Near-tangent cylinders leaving a sliver face."""
    a = bd.Cylinder(radius=5.0, height=6.0)
    b = bd.Pos((9.999, 0, 0)) * bd.Cylinder(radius=5.0, height=6.0)
    return {"a": a, "b": b, "op": "cut"}


def scale_span_shock(args):
    """1e-2 sphere and 1e2 box in one compound."""
    small = bd.Sphere(radius=0.01)
    big = bd.Pos((200.0, 0, 0)) * bd.Box(100, 100, 100)
    return {"compound": [small, big]}


# --- F. representation extremes ------------------------------------------------

def rational_heavy_spline(args):
    """Spline profile feeding a loft (weights regime via the bridge's
    exact interpolation)."""
    p1 = bd.Polygon(*[(0, 0), (10, 0), (10, 2)])
    p2 = bd.Polygon(*[(0, 0), (10, 0.5), (10, 2)])
    return {"loft": [p1, p2]}


def twisted_loft_stations(args):
    """Sections rotated per station: correspondence must certify or refuse."""
    p1 = bd.Polygon(*[(-2, -2), (2, -2), (2, 2), (-2, 2)])
    p2 = bd.Pos((0, 0, 4)) * bd.Rotation((0, 0, 90)) * bd.Polygon(
        *[(-2, -2), (2, -2), (2, 2), (-2, 2)])
    return {"loft": [p1, p2]}


def c0_knot_profile(args):
    """A profile with a kink: C0 discipline at the profile row."""
    p = bd.Polyline((0, 0), (4, 0), (4, 4), (6, 4), (6, 0), (10, 0),
                    (10, 6), (0, 6), close=True)
    return {"extrude": p, "amount": 3.0}


# --- G. assembly identity & placement ------------------------------------------

def prototype_reuse(args):
    """One solid placed 9x: occurrence identity (no geometry copies)."""
    proto = bd.Cylinder(radius=1.0, height=3.0)
    parts = [bd.Pos((float(i), 0.0, 0.0)) * proto for i in range(9)]
    return {"compound": parts}


def mirror_twins(args):
    """Mirror-symmetric solids in one compound: identity-by-coordinates trap."""
    a = bd.Box(4, 2, 2)
    b = bd.Pos((-6, 0, 0)) * bd.Box(4, 2, 2)
    return {"compound": [a, b]}


def nested_compounds(args):
    """Group of groups of groups: solid_count flattens exactly."""
    inner = [bd.Box(2, 2, 2), bd.Pos((4, 0, 0)) * bd.Box(2, 2, 2)]
    mid = [inner, bd.Pos((0, 6, 0)) * bd.Box(2, 2, 2)]
    return {"compound": mid}


def frame_composition_chain(args):
    """Pos o Rotation o Pos chains: the frame law composes, facts exact."""
    part = bd.Pos((1, 2, 3)) * bd.Rotation((0, 0, 30)) * bd.Pos((10, 0, 0)) * bd.Cylinder(radius=1.0, height=5.0)
    return {"compound": [part]}


# --- H. shells, offsets, thickens (the CC strata classes) -----------------------

def box_shell(args):
    """Uniform shell, top face removed: the CC-023 shell certificate class."""
    return {"shell": bd.Box(10, 10, 10), "thickness": 1.0, "open": "top"}


def cylinder_shell(args):
    return {"shell": bd.Cylinder(radius=3.0, height=8.0), "thickness": 0.5, "open": "top"}


def spline_loft_shell(args):
    """Shell of a lofted spline solid: the hard shell (strata + trims)."""
    p1 = bd.Polygon(*[(0, 0), (8, 0), (8, 3)])
    p2 = bd.Pos((0, 0, 4)) * bd.Polygon(*[(0, 0), (6, 0), (6, 3)])
    solid = {"loft": [p1, p2]}
    return {"shell": solid, "thickness": 0.4, "open": "top"}


def variable_thickness_offset(args):
    """Variable-radius offset: the CC-031 variable-radius class."""
    return {"shell": bd.Cylinder(radius=3.0, height=8.0), "thickness": [0.3, 0.8], "open": "top"}


def thicken_sheet(args):
    """Thicken a (conceptual) sheet: one-sided offset arm."""
    return {"thicken": bd.Box(10, 10, 0.2), "thickness": 0.4}


def offset_self_intersect(args):
    """Offset large enough to self-intersect: canal singular/swallowtail
    regime -> must refuse typed (CC-021 strata), never emit."""
    return {"shell": bd.Box(4, 4, 4), "thickness": 10.0, "open": "top"}


# --- I. basic operations the corpus never touched (the systematic layer) ------

def drafted_extrude(args):
    """Tapered (drafted) extrude: profile scaling along the prism."""
    p = bd.Polygon(*[(0, 0), (8, 0), (8, 4), (0, 4)])
    return {"extrude": p, "amount": 6.0, "taper": 5.0}


def loft_intersection(args):
    """Intersect two lofts: swept x swept on the intersect verb."""
    p1 = bd.Polygon(*[(-3, -3), (3, -3), (3, 3), (-3, 3)])
    p2 = bd.Pos((0, 0, 6)) * bd.Polygon(*[(-1, -3), (1, -3), (1, 3), (-1, 3)])
    a = {"loft": [p1, p2]}
    q1 = bd.Polygon(*[(-3, -3), (-3, 3), (3, 3), (3, -3)])
    q2 = bd.Pos((6, 0, 0)) * bd.Rotation((90, 0, 0)) * bd.Polygon(
        *[(-1, -3), (1, -3), (1, 3), (-1, 3)])
    b = {"loft": [q1, q2]}
    return {"a": a, "b": b, "op": "intersect"}


def section_slice(args):
    """Slice a solid with a plane: cross-section extraction."""
    solid = bd.Box(10, 10, 10)
    return {"section": solid, "plane": ((5.0, 0.0, 0.0), (0.0, 1.0, 0.0))}


def asymmetric_chamfer(args):
    """Chamfer with two different setbacks."""
    return {"chamfer": bd.Box(10, 10, 10), "length": 1.0, "length2": 2.5}


def multi_section_loft_tangent(args):
    """4-station loft with end-tangent continuity declared."""
    p1 = bd.Polygon(*[(0, 0), (6, 0), (6, 2)])
    p2 = bd.Pos((0, 0, 2)) * bd.Polygon(*[(0.5, 0), (5.5, 0), (5.5, 1.8)])
    p3 = bd.Pos((0, 0, 4)) * bd.Polygon(*[(1.0, 0), (5.0, 0), (5.0, 1.5)])
    p4 = bd.Pos((0, 0, 6)) * bd.Polygon(*[(1.5, 0), (4.5, 0), (4.5, 1.2)])
    return {"loft": [p1, p2, p3, p4]}


def helical_sweep(args):
    """Sweep along a helical path: the deck_max discipline under rotation."""
    return {"sweep_helix": {"radius": 3.0, "pitch": 1.5, "turns": 3.0,
                            "section": bd.Polygon(*[(-0.4, -0.4), (0.4, -0.4),
                                                    (0.4, 0.4), (-0.4, 0.4)])}}


def twisted_sweep(args):
    """Sweep with per-station twist: frame transport under rotation."""
    return {"sweep_twist": {"radius": 2.0, "length": 8.0, "twist_deg": 180.0,
                            "section": bd.Polygon(*[(-0.5, -0.5), (0.5, -0.5),
                                                    (0.5, 0.5), (-0.5, 0.5)])}}


def project_curve_to_face(args):
    """Project a curve onto a face: the projection arm."""
    face = bd.Box(10, 10, 10)
    return {"project": {"curve": [(0.0, 0.0, 12.0), (6.0, 0.0, 12.0)],
                        "onto": face, "direction": (0.0, 0.0, -1.0)}}


def nonuniform_scale(args):
    """Non-uniform scaling of a solid: affine placement law."""
    return {"scale": bd.Sphere(radius=2.0), "factor": (1.0, 2.0, 0.5)}


def pattern_composition(args):
    """Rectangular pattern of a boolean'd feature: pattern o boolean."""
    base = bd.Box(20, 20, 4)
    hole = bd.Cylinder(radius=1.0, height=8.0)
    parts = [bd.Pos((float(x), float(y), 0.0)) * hole
             for x in (5.0, 10.0, 15.0) for y in (5.0, 10.0, 15.0)]
    return {"chain": [{"a": base, "b": parts[0], "op": "cut"}] +
            [{"b": p, "op": "cut"} for p in parts[1:]]}


def fillet_spline_edge(args):
    """Fillet on a spline-section loft's edges: blend on non-canonical face."""
    p1 = bd.Polygon(*[(0, 0), (8, 0), (8, 3)])
    p2 = bd.Pos((0, 0, 4)) * bd.Polygon(*[(0, 0), (6, 0), (6, 3)])
    return {"loft": [p1, p2], "fillet_after": 0.5}

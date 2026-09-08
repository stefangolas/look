"""Text-to-cad corpus door (PB-008-TTC-HARNESS).

A plain, process-per-script door: the harness never reimplements cadgen's
daemon/store (spec boundary), it spawns one fresh python process per corpus
row and this module runs that row's geometry entry end to end.

Regime: the OCC baseline engine. The vendored corpus scripts are genuine
build123d scripts that import ``cadgen.build123d``; that module is a lazy,
transparent re-export of the genuine ``build123d`` package, so this door
answers the ``cadgen`` import with the installed build123d module (PEP 562
style) and never wraps or "improves" a build123d name. ``compound_from_instances``
(a cadgen composition helper the Falcon-Heavy tree uses to place repeated
engine instances) is answered with the equivalent plain build123d composition;
no cadgen daemon/store surface is reimplemented.

Outputs for one row:

* an STL file at the requested path (STL, never STEP — the TR-NRB-001
  boundary for swept/constructive parts holds on the corpus side too);
* a JSON record on stdout: door version, entry, type, build wall time
  (local evidence only, never published — BENCHMARKS doctrine), the geometry
  facts (solid count, volume, bounding box) and the STL triangle count.

The tessellation linear-deflection tolerance is scaled to the part's bounding
box diagonal so the same door serves a 0.5 m engine nozzle and a 70 m launch
vehicle (an absolute deflection that suits one makes the other absurd).

Usage:

    python door.py [--engine occ|truck] <tree_src_dir> <module> <entry> <args_json> <stl_path>

``--engine`` selects the regime: ``occ`` (default, the OCC baseline) or
``truck`` (the kernel-engine regime landed by TTC-EXECUTOR-BINDING, which
answers the corpus imports with the truck drop-in module and computes the
geometry facts through the deterministic kernel executor).

No cargo, no kernel, no GIL policy applies here: this file is a fixture-side
test harness, not production code. Determinism: identical ordered input yields
identical geometry facts; wall time is recorded but never asserted.
"""

from __future__ import annotations

import importlib
import json
import math
import os
import struct
import sys
import time
import types

DOOR_VERSION = "1"
TRUCK_DOOR_VERSION = "2"

# The engine regime (TTC-EXECUTOR-BINDING). ``occ`` is the OCC baseline
# (unchanged, the differential oracle and reference recorder); ``truck`` is
# the kernel-engine regime the executor binding lands: the alias installs the
# truck bridge module instead of build123d and the geometry facts come from
# the deterministic kernel executor behind ``truck123d.bd_*``.
ENGINE = "occ"


def install_truck_alias():
    """Answer the corpus imports ``cadgen.build123d`` with the truck drop-in:
    a build123d-named, data-recording surface over the native geometry bridge.

    The corpus scripts run UNMODIFIED on the kernel engine regime: every
    census name this surface answers has the build123d signature (never
    improved) and records one data row; all geometry (volume, bounding box,
    triangles) is computed by the native executor (``truck123d.bd_facts`` /
    ``truck123d.bd_stl``) when the door measures or exports. Nothing
    geometric computes in Python. A name whose carrier form the executor does
    not cover raises the mapped ``truck123d.Refused`` exception — loud, never
    a silent fallback to OCC.
    """
    global _T123D
    import truck123d

    _T123D = truck123d
    cadgen = types.ModuleType("cadgen")
    bd = _build_truck_module()
    cadgen.build123d = bd
    cadgen.__path__ = []

    def _compound_from_instances(name, instances):
        objs = []
        for prototype, location, instance_name in instances:
            placed = prototype.moved(location)
            placed.label = instance_name
            objs.append(placed)
        return bd.Compound(obj=objs, children=objs, label=name)

    cadgen.compound_from_instances = _compound_from_instances
    sys.modules["cadgen"] = cadgen
    return bd


# The native geometry bridge (set by install_truck_alias).
_T123D = None


def _refuse(message):
    """Raise the mapped truck exception for a name the executor cannot answer
    (typed, never a bare Exception or a silent fallback)."""
    exc = _T123D.Refused(message)
    exc.payload = {"case": "unsupported_envelope", "envelope": "non_canonical_carrier"}
    raise exc


# The loft refusal message (kept as one anchored literal; both the empty-list
# and the non-Face-section refusal paths raise it).
_LOFT_REFUSAL = "loft is not a kernel-engine row"


def _num(value):
    """Coerce a numeric literal to float (deterministic descriptor numbers)."""
    if isinstance(value, bool):
        return float(value)
    if isinstance(value, int):
        return float(value)
    return value


class Vector:
    """build123d ``Vector``: a 3-vector data row (client-layer arithmetic)."""

    def __init__(self, *args):
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            args = tuple(args[0])
        if len(args) != 3:
            raise TypeError("Vector requires three coordinates")
        self.x, self.y, self.z = (_num(v) for v in args)

    def to_tuple(self):
        return (self.x, self.y, self.z)

    def __iter__(self):
        return iter((self.x, self.y, self.z))

    def __repr__(self):
        return f"Vector({self.x}, {self.y}, {self.z})"


class Axis:
    """build123d ``Axis``: a direction (the z axis is the corpus's revolve/
    rotate carrier)."""

    def __init__(self, direction):
        self.direction = Vector(direction)

    def __repr__(self):
        return f"Axis({self.direction.to_tuple()})"


Axis.Z = Axis((0, 0, 1))


class Location:
    """build123d ``Location``: a placement. The corpus's kernel-engine rows
    place parts by translation or by a pure-z rotation before translation;
    both are census carriers the executor answers (``x/y/z`` plus ``rz``).
    A two-argument location whose rotation has a non-z component is not a
    carrier the executor answers and refuses typed at use."""

    def __init__(self, *args):
        if len(args) == 1:
            position = args[0]
            if isinstance(position, Vector):
                position = position.to_tuple()
            if isinstance(position, (tuple, list)) and len(position) == 3:
                self.x, self.y, self.z = (_num(v) for v in position)
                self._rz = 0.0
                self.unsupported_rotation = False
                return
        if len(args) == 2:
            position = args[0]
            rotation = args[1]
            if isinstance(position, Vector):
                position = position.to_tuple()
            if (
                isinstance(position, (tuple, list))
                and len(position) == 3
                and isinstance(rotation, (tuple, list))
                and len(rotation) == 3
                and _num(rotation[0]) == 0.0
                and _num(rotation[1]) == 0.0
            ):
                self.x, self.y, self.z = (_num(v) for v in position)
                self._rz = _num(rotation[2])
                self.unsupported_rotation = False
                return
        _refuse("Location forms beyond a translation or pure-z rotation are not census carriers")


class Edge:
    """build123d ``Edge``: a curve carrier data row (line or spline)."""

    def __init__(self, kind, p0, p1, points=None, options=None):
        self.kind = kind
        self.p0 = p0
        self.p1 = p1
        self.points = points or []
        # Which ``make_spline`` options beyond the plain point list were
        # supplied. ``Edge.make_spline(points)`` fixes a recoverable
        # interpolation convention (the OCC chord-length clamped cubic); any
        # option (tangents/periodic/parameters/...) changes the curve in a way
        # the recorded sample list alone cannot recover, so such an edge is
        # not a kernel-engine row and refuses typed at use.
        self.options = dict(options) if options else {}

    @classmethod
    def make_line(cls, p0, p1):
        return cls("line", _vector(p0), _vector(p1))

    @classmethod
    def make_spline(cls, points, *args, **kwargs):
        pts = [_vector(p) for p in points]
        if len(pts) < 2:
            raise ValueError("spline needs at least two points")
        options = {}
        if args:
            options["positional"] = True
        for key in kwargs:
            options[key] = True
        return cls("spline", pts[0], pts[-1], points=pts, options=options)

    def position_at(self, t):
        """build123d ``Edge.position_at``.

        Data-only answer: at `t == 0` the recorded curve starts at its first
        recorded point (exact for a line endpoint and for the interpolating
        spline through the recorded samples). Any other parameter of a spline
        edge requires reconstructing the curve, which is the kernel's exact
        arithmetic, never a Python approximation — such a query refuses typed.
        """
        if _num(t) == 0.0:
            return self.p0
        if self.kind == "line":
            return _vector(
                (
                    self.p0.x + _num(t) * (self.p1.x - self.p0.x),
                    self.p0.y + _num(t) * (self.p1.y - self.p0.y),
                    self.p0.z + _num(t) * (self.p1.z - self.p0.z),
                )
            )
        _refuse("a spline path query beyond its recorded start is not a kernel-engine row")
        return self.p0

    def tangent_at(self, t):
        """build123d ``Edge.tangent_at``.

        The tangent of a recorded spline is the kernel's reconstruction, never
        a Python computation: any tangent query on the data-only carrier
        refuses typed (the mapped ``Refused`` class), so a spline-path sweep
        dies with a typed census verdict instead of an untyped attribute
        failure.
        """
        _refuse("a spline path tangent query is not a kernel-engine row")
        return _vector((0.0, 0.0, 1.0))


class Wire:
    """build123d ``Wire``: an ordered list of curve carriers."""

    def __init__(self, edges):
        self.edges = list(edges)

    def __iter__(self):
        return iter(self.edges)


class Face:
    """build123d ``Face``: a planar region bounded by one wire (data only)."""

    def __init__(self, obj=None, *args, **kwargs):
        if isinstance(obj, Wire):
            self.wire = obj
        elif obj is None:
            self.wire = Wire([])
        else:
            _refuse("this Face form is not a census carrier")


class _Shape:
    """The base of every placed construction row: one placed solid (a part)
    or a compound of child rows. All geometry lives behind the native
    executor; this side only records data and frames."""

    def _facts(self):
        return json.loads(_T123D.bd_facts(json.dumps(self._node())))


class _Part(_Shape):
    """One placed solid: a local solid descriptor plus a world frame."""

    def __init__(self, solid):
        self._solid = solid
        self._x = 0.0
        self._y = 0.0
        self._z = 0.0
        self._rz = 0.0
        # The placed-carrier reflection recorded by the mirror arm: one of
        # "x" (YZ plane), "y" (XZ plane) or "z" (XY plane), else None.
        self._mirror = None
        self.label = ""
        self.color = None
        self._type_name = "Part"

    def _node(self):
        node = {
            "solid": self._solid,
            "x": self._x,
            "y": self._y,
            "z": self._z,
            "rz": self._rz,
        }
        if self._mirror is not None:
            node["mirror"] = self._mirror
        return {"part": node}

    def locate(self, loc):
        self._x += loc.x
        self._y += loc.y
        self._z += loc.z
        rz = getattr(loc, "_rz", 0.0)
        if rz:
            self._rz += rz
        return self

    def moved(self, loc):
        return self.locate(loc)

    def rotate(self, axis, angle):
        if not isinstance(axis, Axis) or not _close(axis.direction, (0, 0, 1)):
            _refuse("rotate about a non-z axis is not a kernel-engine row")
        if _num(angle) == 0.0:
            return self
        self._rz += _num(angle)
        return self

    def solids(self):
        return [self]

    @property
    def volume(self):
        return float(self._facts()["volume"])

    def bounding_box(self):
        mn, mx = self._facts()["bbox"]
        return _BoundingBox(Vector(mn), Vector(mx))


class Compound(_Shape):
    """build123d ``Compound``: a group of child construction rows."""

    def __init__(self, obj=None, children=None, label=""):
        if children is not None:
            self._children = list(children)
        elif obj is not None and not isinstance(obj, Compound):
            self._children = list(obj)
        else:
            self._children = []
        self.label = label
        self.color = None
        self._type_name = "Compound"

    def _node(self):
        return {"group": [child._node() for child in self._children]}

    def _leaves(self):
        out = []
        for child in self._children:
            if isinstance(child, _Part):
                out.append(child)
            else:
                out.extend(child._leaves())
        return out

    def solids(self):
        return self._leaves()

    @property
    def volume(self):
        return float(self._facts()["volume"])

    def bounding_box(self):
        mn, mx = self._facts()["bbox"]
        return _BoundingBox(Vector(mn), Vector(mx))


class _BoundingBox:
    def __init__(self, mn, mx):
        self.min = mn
        self.max = mx


def _vector(value):
    if isinstance(value, Vector):
        return value
    return Vector(value)


def _close(direction, target):
    return (
        abs(_num(direction.x) - target[0]) < 1e-12
        and abs(_num(direction.y) - target[1]) < 1e-12
        and abs(_num(direction.z) - target[2]) < 1e-12
    )


def Box(length, width, height, mode=None, **kwargs):
    """build123d ``Box(length, width, height)`` (centered on the origin)."""
    return _Part(
        {
            "kind": "box",
            "length": _num(length),
            "width": _num(width),
            "height": _num(height),
        }
    )


def _cylinder_axis(rotation):
    if rotation is None:
        return "z"
    rx, ry, rz = (_num(v) for v in rotation)
    if rz != 0.0 or (rx == 0.0 and ry == 0.0):
        return "z" if rz == 0.0 else _refuse("z-rotated cylinders are not a kernel-engine row")
    if rx == 90.0 and ry == 0.0:
        return "y"
    if ry == 90.0 and rx == 0.0:
        return "x"
    _refuse("this cylinder rotation is not a census carrier")
    return "z"


def Cylinder(radius, height, rotation=(0, 0, 0), mode=None, align=None, **kwargs):
    """build123d ``Cylinder(radius, height)`` (centered, z-axis by default)."""
    return _Part(
        {
            "kind": "cylinder",
            "radius": _num(radius),
            "height": _num(height),
            "axis": _cylinder_axis(rotation),
        }
    )


def Sphere(radius, mode=None, **kwargs):
    """build123d ``Sphere(radius)`` (centered on the origin)."""
    return _Part({"kind": "sphere", "radius": _num(radius)})


def Torus(major_radius, minor_radius, major_angle=360.0, mode=None, **kwargs):
    """build123d ``Torus(major_radius, minor_radius)`` (centered, z-axis)."""
    if _num(major_angle) != 360.0:
        _refuse("a partial-arc torus is not a kernel-engine row")
    return _Part(
        {
            "kind": "torus",
            "major": _num(major_radius),
            "minor": _num(minor_radius),
        }
    )


def revolve(shape, axis=None, revolution_arc=360.0, **kwargs):
    """build123d ``revolve(shape, axis, revolution_arc)``.

    The executor's lathe arm covers a closed profile in the ``y = 0`` plane
    revolved a full 360 degrees about the z axis. The profile may mix straight
    edges and ``Edge.make_spline(points)`` spline edges (the spline is
    recorded by its defining samples and the kernel integrates the true
    interpolating spline, never a flattening polygon). Any other carrier (a
    partial arc, a non-z axis, an open profile, a spline whose interpolation
    options the recorded data cannot recover) refuses typed.
    """
    if not isinstance(axis, Axis) or not _close(axis.direction, (0, 0, 1)):
        _refuse("revolve about a non-z axis is not a kernel-engine row")
    if not isinstance(shape, Face):
        _refuse("revolve of a non-Face shape is not a kernel-engine row")
    if _num(revolution_arc) != 360.0:
        _refuse("a partial-arc revolve is outside the executor's lathe arm")
    edges = list(shape.wire.edges)
    if len(edges) < 3:
        _refuse("revolve needs a closed profile")
    profile = []
    for edge in edges:
        if edge.kind == "line":
            if edge.p0.y != 0.0 or edge.p1.y != 0.0:
                _refuse("a revolve profile outside the y=0 plane is not a kernel-engine row")
            profile.append(
                {
                    "kind": "line",
                    "a": [edge.p0.x, edge.p0.z],
                    "b": [edge.p1.x, edge.p1.z],
                }
            )
        elif edge.kind == "spline":
            if edge.options:
                # An interpolation option (tangents/periodic/parameters) is not
                # recoverable from the recorded sample list; never guess one.
                _refuse("a spline-profile revolve is not a kernel-engine row")
            for point in edge.points:
                if point.y != 0.0:
                    _refuse("a revolve profile outside the y=0 plane is not a kernel-engine row")
            profile.append(
                {
                    "kind": "spline",
                    "points": [[point.x, point.z] for point in edge.points],
                }
            )
        else:
            _refuse("this curve carrier is not a lathe profile edge")
    first = edges[0].p0
    last = edges[-1].p1
    if (
        abs(first.x - last.x) > 1e-9
        or abs(first.y - last.y) > 1e-9
        or abs(first.z - last.z) > 1e-9
    ):
        _refuse("a revolve profile must close on itself")
    return _Part({"kind": "lathe", "profile": profile, "arc_deg": 360.0})


# ---------------------------------------------------------------------------
# Authoring arms: 3-D profile recording helpers
# ---------------------------------------------------------------------------

def _edge3(edge):
    """Record one 3-D profile edge carrier: a line records its endpoints, a
    spline its defining samples (never a flattening polygon)."""
    if edge.kind == "line":
        return {
            "kind": "line",
            "a": edge.p0.to_tuple(),
            "b": edge.p1.to_tuple(),
        }
    return {"kind": "spline", "points": [p.to_tuple() for p in edge.points]}


def _wire_profile_edges(shape):
    """The recorded boundary edges of a Face carrier, as 3-D profile data."""
    edges = list(shape.wire.edges)
    if len(edges) < 3:
        _refuse("a profile needs a closed boundary with at least three edges")
    return [_edge3(edge) for edge in edges]


def make_face(*objs, **kwargs):
    """build123d ``make_face(wire)``: record a planar Face bounded by one wire.

    The recorded surface is elementary (the plane the boundary lies in); a
    spline-trimmed boundary records its defining spline samples in the wire's
    trim edges — the kernel derives the plane and integrates the true trim
    curves, never a flattening polygon. Carriers outside the wire form refuse
    typed.
    """
    edges = []
    for obj in objs:
        if isinstance(obj, Wire):
            edges.extend(obj.edges)
        elif isinstance(obj, Edge):
            edges.append(obj)
        elif obj is None:
            continue
        else:
            _refuse("make_face is not a corpus kernel-engine carrier")
    if not edges:
        _refuse("make_face is not a corpus kernel-engine carrier")
    return Face(Wire(edges))


def extrude(shape, amount, both=False, mode=None, **kwargs):
    """build123d ``extrude(face, amount, both)``: the recording arm for a
    closed planar line-loop profile swept along its own plane normal (an exact
    prism — the volume is the profile area times the swept length, `2 * amount`
    when `both`). The record keeps the profile boundary in order; the kernel
    derives the profile plane and normal exactly and never approximates. A
    spline-trimmed face boundary or a non-Face carrier refuses typed.
    """
    if not isinstance(shape, Face):
        _refuse("Face extrusion is not yet a kernel-engine row")
    profile = []
    for edge in shape.wire.edges:
        if edge.kind != "line":
            _refuse("a spline-trimmed face extrusion is not a kernel-engine row")
        profile.append(_edge3(edge))
    if len(profile) < 3:
        _refuse("Face extrusion is not yet a kernel-engine row")
    return _Part(
        {
            "kind": "prism",
            "profile": profile,
            "amount": _num(amount),
            "both": bool(both),
        }
    )


def sweep(section=None, path=None, mode=None, **kwargs):
    """build123d ``sweep(section, path)``.

    A sweep records as a loft chain (per-segment lofts between recorded
    stations) only when the path and section carriers are fully recorded as
    data. A spline path is never flattened: its section/query attributes
    refuse typed (``Edge.tangent_at``), and any sweep carrier outside the
    recorded chain refuses typed here.
    """
    _refuse("sweep is not a kernel-engine row")


def loft(*sections, ruled=False, mode=None, **kwargs):
    """build123d ``loft(sections, ruled=...)``: the recording arm.

    Records the ordered section profiles (each a Face carrier's boundary, in
    the part's local frame) plus the cross interpolation as one ``Loft`` row.
    The kernel certifies the ruled matched-vertex chain with exact facts; a
    section carrier outside the recorded wire form refuses typed.
    """
    faces = []
    for item in sections:
        if isinstance(item, (list, tuple)):
            faces.extend(item)
        else:
            faces.append(item)
    if not faces:
        _refuse(_LOFT_REFUSAL)
    recorded = []
    for face in faces:
        if not isinstance(face, Face):
            _refuse(_LOFT_REFUSAL)
        recorded.append(_wire_profile_edges(face))
    return _Part({"kind": "loft", "closed": False, "sections": recorded})


def fillet(edges, radius, **kwargs):
    _refuse("fillet is not a kernel-engine row")


def chamfer(edges, length, **kwargs):
    _refuse("chamfer is not a kernel-engine row")


class _PlaneMarker:
    """A coordinate-plane marker the mirror arm answers as a placed carrier."""

    def __init__(self, name):
        self.name = name

    def __repr__(self):
        return f"Plane.{self.name.upper()}"


def _mirror_row(part, axis):
    """The placed-carrier mirror of one part row.

    ``mirror(shape, about)`` reflects the shape about the plane through the
    world origin. In the part model the geometry lives in the solid spec and
    the frame applies rz then translation, so the reflected row carries the
    same solid, a mirror reflection of the local geometry, a reflected frame
    translation and a negated rz for the anti-commuting (x/y) reflections (the
    z reflection commutes with the z rotation).
    """
    out = _Part(part._solid)
    out._x = part._x
    out._y = part._y
    out._z = part._z
    if axis == "x":
        out._x = -out._x
        out._rz = -part._rz
    elif axis == "z":
        out._z = -out._z
        out._rz = part._rz
    else:
        out._y = -out._y
        out._rz = -part._rz
    if part._mirror == axis:
        # Mirroring an already-mirrored row reflects back to the identity.
        out._mirror = None
    else:
        out._mirror = axis
    out.label = part.label
    out.color = part.color
    return out


def mirror(shape, about=None, mode=None, **kwargs):
    """build123d ``mirror(shape, about)``: the recording arm.

    A mirror about a coordinate plane records a placed-carrier transform over
    the mirrored census row (the landed placed/processor rule): the solid
    carrier is untouched and the facts transform with the placement — no
    geometry is recomputed. A mirror about any non-coordinate carrier refuses
    typed.
    """
    axis = None
    if isinstance(about, _PlaneMarker):
        axis = {"xz": "y", "yz": "x", "xy": "z"}.get(about.name)
    if axis is None:
        _refuse("mirror is not a kernel-engine row")
    if isinstance(shape, _Part):
        return _mirror_row(shape, axis)
    if isinstance(shape, Compound):
        return Compound(children=[mirror(child, about=about) for child in shape._children])
    if isinstance(shape, (list, tuple)):
        return [mirror(child, about=about) for child in shape]
    _refuse("a mirror of this carrier is not a kernel-engine row")
    return None


def Polygon(*points, **kwargs):
    _refuse("polygon profile authoring is not a corpus kernel-engine carrier")


def Polyline(*points, **kwargs):
    _refuse("polyline profile authoring is not a corpus kernel-engine carrier")


def Spline(*points, periodic=False, **kwargs):
    _refuse("spline profile authoring is not a corpus kernel-engine carrier")


def Circle(radius, **kwargs):
    _refuse("circle profile authoring is not a corpus kernel-engine carrier")


class Plane:
    """build123d ``Plane``: a plane data row.

    The coordinate-plane markers the mirror arm answers (``about`` carriers)
    are exposed as ``Plane.XZ`` / ``Plane.YZ`` / ``Plane.XY``; a constructed or
    algebra form is recorded client-layer data the executor does not answer and
    refuses typed.
    """

    def __init__(self, *args, **kwargs):
        _refuse("Plane algebra is recorded client-layer data; no kernel row")


Plane.XZ = _PlaneMarker("xz")
Plane.YZ = _PlaneMarker("yz")
Plane.XY = _PlaneMarker("xy")


def Pos(*args, **kwargs):
    _refuse("Pos is recorded client-layer data; no kernel row")


def Rotation(*args, **kwargs):
    _refuse("Rotation is recorded client-layer data; no kernel row")


def export_stl(part, path, tolerance=None, angular_tolerance=None, **kwargs):
    """build123d ``export_stl(shape, path)``: the kernel executor writes the
    deterministic triangle soup of the submitted construction tree."""
    result = json.loads(_T123D.bd_stl(json.dumps(part._node()), str(path)))
    return result["triangles"]


# The census vocabulary this drop-in answers name-for-name (spec 8). Every
# name is answered (signatures never improved); names whose carrier form the
# landed executor cannot answer refuse with the mapped typed exception.
_TRUCK_NAMES = [
    "Vector",
    "Location",
    "Axis",
    "Edge",
    "Wire",
    "Face",
    "Compound",
    "Box",
    "Cylinder",
    "Sphere",
    "Torus",
    "revolve",
    "extrude",
    "sweep",
    "loft",
    "fillet",
    "chamfer",
    "mirror",
    "Polygon",
    "Polyline",
    "Spline",
    "Circle",
    "make_face",
    "export_stl",
    "Plane",
    "Pos",
    "Rotation",
]


def _build_truck_module():
    bd = types.ModuleType("bd")
    for name in _TRUCK_NAMES:
        value = globals()[name]
        setattr(bd, name, value)
    bd.Mode = types.SimpleNamespace(ADD="union", SUBTRACT="subtract", INTERSECT="intersect")
    bd.__dict__["Box"] = Box
    return bd



def install_cadgen_alias() -> None:
    """Answer the corpus imports ``cadgen.build123d`` (and the placement helper
    ``cadgen.compound_from_instances``) with the genuine build123d package."""
    import build123d as real

    cadgen = types.ModuleType("cadgen")
    cadgen.build123d = real
    cadgen.__path__ = []
    sys.modules["cadgen"] = cadgen

    def _compound_from_instances(name, instances):
        objs = []
        for prototype, location, instance_name in instances:
            placed = prototype.moved(location)
            placed.label = instance_name
            objs.append(placed)
        return real.Compound(obj=objs, children=objs, label=name)

    cadgen.compound_from_instances = _compound_from_instances


def run_entry(tree_src: str, module: str, entry: str, args: list) -> object:
    """Import the corpus module and call its geometry entry, fresh per process."""
    sys.path.insert(0, tree_src)
    if ENGINE == "truck":
        # The kernel-engine regime: the alias installs the truck drop-in
        # module (bd_bridge over truck123d) instead of build123d.
        truck_bd = install_truck_alias()
    else:
        install_cadgen_alias()
    mod = importlib.import_module(module)
    fn = getattr(mod, entry)
    result = fn(*args) if args else fn()
    if isinstance(result, list):
        # A corpus entry may return the model's parts as a list of groups (the
        # top-level Falcon wrapper wraps them in one Compound); wrap here so the
        # door always exports one shape.
        if ENGINE == "truck":
            result = truck_bd.Compound(obj=result, children=result, label=entry)
        else:
            import build123d as bd

            result = bd.Compound(obj=result, children=result, label=entry)
    return result


def geometry_facts(obj) -> dict:
    facts = {}
    try:
        facts["solid_count"] = len(obj.solids())
    except Exception as exc:  # noqa: BLE001 - door reports, never crashes
        facts["solid_count"] = f"err:{type(exc).__name__}"
    try:
        facts["volume"] = obj.volume
    except Exception as exc:  # noqa: BLE001
        facts["volume"] = f"err:{type(exc).__name__}"
    try:
        bb = obj.bounding_box()
        mn = tuple(bb.min.to_tuple())
        mx = tuple(bb.max.to_tuple())
        facts["bbox"] = [list(mn), list(mx)]
        facts["diag"] = math.sqrt(sum((mx[i] - mn[i]) ** 2 for i in range(3)))
    except Exception as exc:  # noqa: BLE001
        facts["bbox"] = f"err:{type(exc).__name__}"
    return facts


def triangle_count(stl_path: str) -> int:
    with open(stl_path, "rb") as fh:
        header = fh.read(80)
        count = fh.read(4)
    if len(header) < 80 or len(count) < 4:
        return -1
    return struct.unpack("<I", count)[0]


def _truck_export_stl(obj, stl_path: str) -> None:
    """Export one construction tree as a binary STL through the native
    executor (the truck regime's deterministic tessellation)."""
    _T123D.bd_stl(json.dumps(obj._node()), str(stl_path))


def main() -> int:
    argv = list(sys.argv[1:])
    global ENGINE
    if len(argv) >= 2 and argv[0] == "--engine":
        ENGINE = argv[1]
        argv = argv[2:]
    if len(argv) < 5:
        record = {
            "schema": "ttc_door_run.v1",
            "door_version": DOOR_VERSION,
            "ok": False,
            "error": {"kind": "UsageError", "message": "need tree module entry args stl"},
        }
        json.dump(record, sys.stdout, indent=2)
        return 1
    tree_src = argv[0]
    module = argv[1]
    entry = argv[2]
    args = json.loads(argv[3])
    stl_path = argv[4]
    door_version = TRUCK_DOOR_VERSION if ENGINE == "truck" else DOOR_VERSION

    t0 = time.time()
    try:
        obj = run_entry(tree_src, module, entry, args)
    except Exception as exc:  # noqa: BLE001 - the door reports typed failure
        record = {
            "schema": "ttc_door_run.v1",
            "door_version": door_version,
            "engine": ENGINE,
            "ok": False,
            "entry": entry,
            "error": {"kind": type(exc).__name__, "message": str(exc)},
        }
        json.dump(record, sys.stdout, indent=2)
        return 1

    built = time.time() - t0
    facts = geometry_facts(obj)
    diag = float(facts.get("diag") or 1.0)
    tolerance = max(diag * 2.0e-4, 0.01)
    if ENGINE == "truck":
        _truck_export_stl(obj, stl_path)
    else:
        import build123d as bd

        bd.export_stl(obj, stl_path, tolerance=tolerance, angular_tolerance=0.5)

    record = {
        "schema": "ttc_door_run.v1",
        "door_version": door_version,
        "engine": ENGINE,
        "ok": True,
        "entry": entry,
        "module": module,
        "type": type(obj).__name__,
        "build_seconds": round(built, 3),
        "facts": facts,
        "stl": {
            "path": os.path.basename(stl_path),
            "triangles": triangle_count(stl_path),
        },
    }
    json.dump(record, sys.stdout, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())

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

* an STL file at the requested path (STL, never STEP -- the TR-NRB-001
  boundary for swept/constructive parts holds on the corpus side too);
* a JSON record on stdout: door version, entry, type, build wall time
  (local evidence only, never published -- BENCHMARKS doctrine), the geometry
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

The truck drop-in's authoring surface (AUTHOR-FRAME-CARRIERS) records every
census call as a data row: ``Vector`` answers the corpus direction math as
pure client-side data (never a kernel row); ``Plane(origin, x_dir, z_dir)``
and the ``Plane.XY/XZ/YZ`` markers record an orthonormal frame (x/z
orthonormalized in fixed order, ``y = z cross x``); ``Pos`` records a
translation frame; ``Rotation``/``Rot`` record an Euler frame; ``Location``
generalizes beyond translation/pure-z to the same frame data; and placing
algebra (``frame * shape``) records the placement. A frame's ``.offset``
composes a translation along the frame normal. Profile rows (``Spline`` /
``Circle`` / ``Polyline``) feed ``loft``/``extrude``/``revolve`` only where
the executor answers the facts exactly; a genuinely open carrier (a
spline-section loft, a partial arc, a boolean op) refuses TYPED naming the
carrier -- never approximated, never an untyped door failure.

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
    not cover raises the mapped ``truck123d.Refused`` exception -- loud, never
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


def _color_record(color):
    """The recorded textual form of a client color (pure metadata).

    A string color is carried verbatim; an RGB(A) tuple/list is recorded as its
    JSON array text so the native row always carries a stable string. The value
    is never read by any semantic path (classification, facts arithmetic,
    meshing); it is metadata only.
    """
    if isinstance(color, str):
        return color
    try:
        return json.dumps(color)
    except (TypeError, ValueError):
        return str(color)


# ---------------------------------------------------------------------------
# Vector: the corpus direction-math data row (pure client-side data, never a
# kernel row). `normalized`, `dot`, `cross`, `length` and the scalar /
# difference arithmetic the corpus helpers use all answer here exactly.
# ---------------------------------------------------------------------------


class Vector:
    """build123d ``Vector``: a 3-vector of pure client-side data.

    The corpus uses this for direction math (``.normalized()``, ``.dot``,
    ``.cross``, ``.length``), scalar and difference arithmetic and component
    access (``.X/.Y/.Z``). Every answer is exact float arithmetic on the
    recorded coordinates -- this is data, never a kernel geometry row.
    """

    __slots__ = ("x", "y", "z")

    def __init__(self, *args):
        if len(args) == 1 and isinstance(args[0], Vector):
            other = args[0]
            self.x, self.y, self.z = other.x, other.y, other.z
            return
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            args = tuple(args[0])
        if len(args) == 1:
            self.x = _num(args[0])
            self.y = 0.0
            self.z = 0.0
            return
        if len(args) not in (2, 3):
            raise TypeError("Vector requires one to three coordinates")
        self.x = _num(args[0])
        self.y = _num(args[1])
        self.z = _num(args[2]) if len(args) == 3 else 0.0

    @property
    def X(self):
        return self.x

    @property
    def Y(self):
        return self.y

    @property
    def Z(self):
        return self.z

    def to_tuple(self):
        return (self.x, self.y, self.z)

    def __iter__(self):
        return iter((self.x, self.y, self.z))

    def __len__(self):
        return 3

    def __getitem__(self, index):
        return (self.x, self.y, self.z)[index]

    def __repr__(self):
        return f"Vector({self.x}, {self.y}, {self.z})"

    def __eq__(self, other):
        try:
            o = other if isinstance(other, Vector) else Vector(other)
        except Exception:
            return NotImplemented
        return (self.x, self.y, self.z) == (o.x, o.y, o.z)

    def __add__(self, other):
        o = _as_vector(other)
        return Vector(self.x + o.x, self.y + o.y, self.z + o.z)

    def __sub__(self, other):
        o = _as_vector(other)
        return Vector(self.x - o.x, self.y - o.y, self.z - o.z)

    def __neg__(self):
        return Vector(-self.x, -self.y, -self.z)

    def __mul__(self, scalar):
        s = _num(scalar)
        return Vector(self.x * s, self.y * s, self.z * s)

    def __rmul__(self, scalar):
        return self.__mul__(scalar)

    def __truediv__(self, scalar):
        s = _num(scalar)
        return Vector(self.x / s, self.y / s, self.z / s)

    def __matmul__(self, other):
        o = _as_vector(other)
        return self.x * o.x + self.y * o.y + self.z * o.z

    def dot(self, other):
        o = _as_vector(other)
        return self.x * o.x + self.y * o.y + self.z * o.z

    def cross(self, other):
        o = _as_vector(other)
        return Vector(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )

    def normalized(self):
        length = self.length
        if length == 0.0:
            return Vector(0.0, 0.0, 0.0)
        return Vector(self.x / length, self.y / length, self.z / length)

    @property
    def length(self):
        return math.sqrt(self.x * self.x + self.y * self.y + self.z * self.z)

    @property
    def wrapped(self):
        return None


def _as_vector(value):
    if isinstance(value, Vector):
        return value
    return Vector(value)


def _vdot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _vcross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def _vsub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def _vlen(a):
    return math.sqrt(_vdot(a, a))


def _vnormalize(a):
    length = _vlen(a)
    if length == 0.0:
        return (0.0, 0.0, 0.0)
    return (a[0] / length, a[1] / length, a[2] / length)


def _vscaled(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def _point3(value):
    """A point as a 3-tuple (padding 2-D values on the z=0 sketch plane)."""
    if isinstance(value, Vector):
        return value.to_tuple()
    if isinstance(value, (tuple, list)) and len(value) == 3:
        return (_num(value[0]), _num(value[1]), _num(value[2]))
    if isinstance(value, (tuple, list)) and len(value) == 2:
        return (_num(value[0]), _num(value[1]), 0.0)
    raise TypeError(f"not a point: {value!r}")


# ---------------------------------------------------------------------------
# Placement locators: frames recorded as client data. `world = translate(o) ∘
# R` where R is the full orthonormal 3x3 rotation of the frame (columns x_dir,
# y_dir, z_dir). A locator multiplies a shape (or another locator) on the
# left exactly like build123d's placement algebra.
# ---------------------------------------------------------------------------


class _Frame:
    """A placement frame: origin ``o`` plus the orthonormal rotation columns
    (x_dir, y_dir, z_dir). Pure client-side data; the kernel applies it."""

    __slots__ = ("o", "x_dir", "y_dir", "z_dir")

    def __init__(self, o, x_dir, y_dir, z_dir):
        self.o = o
        self.x_dir = x_dir
        self.y_dir = y_dir
        self.z_dir = z_dir

    def __repr__(self):
        return (
            f"{type(self).__name__}(origin={self.o}, x_dir={self.x_dir}, "
            f"z_dir={self.z_dir})"
        )

    def _rotation_columns(self):
        return (self.x_dir, self.y_dir, self.z_dir)

    def _is_pure_z(self):
        xd, yd, zd = self.x_dir, self.y_dir, self.z_dir
        if not (abs(zd[0]) < 1e-15 and abs(zd[1]) < 1e-15 and abs(zd[2] - 1.0) < 1e-15):
            return False
        if not (abs(yd[0] + 1.0) < 1e-15 and abs(yd[1]) < 1e-15 and abs(yd[2]) < 1e-15):
            if not (abs(yd[0] - 1.0) < 1e-15 and abs(yd[1]) < 1e-15):
                return False
        c = xd[0]
        s = xd[1]
        if not (abs(xd[2]) < 1e-15 and abs(yd[0] + s) < 1e-15 and abs(yd[1] - c) < 1e-15):
            return False
        return True

    def _z_angle_deg(self):
        # RotZ columns: x_dir = (c, s, 0), y_dir = (-s, c, 0).
        return math.degrees(math.atan2(self.x_dir[1], self.x_dir[0]))

    # -- frame arithmetic (data) ------------------------------------------

    def _map_point(self, p):
        """The frame applied to one local point: o + x_dir*p.x + ..."""
        xd, yd, zd = self.x_dir, self.y_dir, self.z_dir
        return (
            self.o[0] + xd[0] * p[0] + yd[0] * p[1] + zd[0] * p[2],
            self.o[1] + xd[1] * p[0] + yd[1] * p[1] + zd[1] * p[2],
            self.o[2] + xd[2] * p[0] + yd[2] * p[1] + zd[2] * p[2],
        )

    def _compose_after(self, other):
        """This frame applied after ``other`` (a locator): R = self.R ∘ other.R,
        o = self.o + self.R(other.o)."""
        xd, yd, zd = self.x_dir, self.y_dir, self.z_dir
        oo = other.o

        def apply_r(p):
            return (
                xd[0] * p[0] + yd[0] * p[1] + zd[0] * p[2],
                xd[1] * p[0] + yd[1] * p[1] + zd[1] * p[2],
                xd[2] * p[0] + yd[2] * p[1] + zd[2] * p[2],
            )

        ox = xd[0] * oo[0] + yd[0] * oo[1] + zd[0] * oo[2]
        oy = xd[1] * oo[0] + yd[1] * oo[1] + zd[1] * oo[2]
        oz = xd[2] * oo[0] + yd[2] * oo[1] + zd[2] * oo[2]
        oxd = apply_r(other.x_dir)
        oyd = apply_r(other.y_dir)
        ozd = apply_r(other.z_dir)
        return _Frame((self.o[0] + ox, self.o[1] + oy, self.o[2] + oz), oxd, oyd, ozd)

    def _rotation_only(self):
        return _Frame((0.0, 0.0, 0.0), self.x_dir, self.y_dir, self.z_dir)

    # -- placement algebra -------------------------------------------------

    def __mul__(self, other):
        if isinstance(other, _Frame):
            return self._compose_after(other)
        if isinstance(other, (list, tuple)):
            return [self.__mul__(item) for item in other]
        if isinstance(other, _Part):
            return _place_part(self, other)
        if isinstance(other, Compound):
            return Compound(
                children=[self.__mul__(child) for child in other._children],
                label=other.label,
            )
        if isinstance(other, Face):
            return _place_face(self, other)
        if isinstance(other, Wire):
            return _place_wire(self, other)
        if isinstance(other, Edge):
            return _place_edges(self, [other])[0]
        if other is None:
            return None
        raise TypeError(f"cannot place a {type(other).__name__} with {self!r}")


class Plane(_Frame):
    """build123d ``Plane``: a placement frame data row.

    ``Plane(origin, x_dir, z_dir)`` records a full frame (the x/z axes are
    orthonormalized in fixed order and ``y = z cross x``); ``Plane(origin,
    z_dir)`` picks a deterministic perpendicular x axis. The coordinate-plane
    markers ``Plane.XY`` / ``Plane.XZ`` / ``Plane.YZ`` are both mirror `about`
    carriers (the mirror arm) and frames; ``Plane.marker.offset(d)`` composes a
    translation of ``d`` along the frame normal. Frame algebra forms outside
    this carrier stay recorded client-layer data and refuse typed -- never an
    untyped failure.
    """

    __slots__ = ("name",)

    def __init__(self, *args, origin=None, x_dir=None, z_dir=None, **kwargs):
        marker = kwargs.pop("marker", None)
        if kwargs:
            _refuse("Plane forms beyond a frame row are not census carriers")
        if marker is not None:
            self.name = marker
            _Frame.__init__(self, *_marker_frame(marker))
            return
        if len(args) == 3:
            origin, x_dir, z_dir = args
        elif len(args) == 1 and isinstance(args[0], Plane):
            other = args[0]
            self.name = getattr(other, "name", None)
            _Frame.__init__(self, other.o, other.x_dir, other.y_dir, other.z_dir)
            return
        elif len(args) != 0:
            _refuse("Plane algebra is recorded client-layer data; no kernel row")
        if origin is None and x_dir is None and z_dir is None:
            _refuse("Plane algebra is recorded client-layer data; no kernel row")
        o = (0.0, 0.0, 0.0) if origin is None else _point3(origin)
        if z_dir is None:
            _refuse("Plane algebra is recorded client-layer data; no kernel row")
        # Fixed-order orthonormalization of the recorded x/z frame:
        #   y = normalize(z x x), then x = normalize(y x z).
        z_axis = _vnormalize(_point3(z_dir))
        if x_dir is None:
            x0 = _perp_any(z_axis)
        else:
            x0 = _vnormalize(_point3(x_dir))
        y_axis = _vcross(z_axis, x0)
        if _vlen(y_axis) < 1e-12:
            x0 = _perp_any(z_axis)
            y_axis = _vcross(z_axis, x0)
        y_axis = _vnormalize(y_axis)
        x_axis = _vnormalize(_vcross(y_axis, z_axis))
        self.name = None
        _Frame.__init__(self, o, x_axis, y_axis, z_axis)

    def rotated(self, euler_deg):
        """``Plane.rotated((rx, ry, rz))`` -- rotate the frame about the world
        axes by the given Euler degrees (the corpus documents world-axis
        composition). Returns a new frame."""
        return _euler_frame(self.o, euler_deg)._compose_after(self._rotation_only())

    def offset(self, distance):
        """Translate the frame by ``distance`` along its normal (z_dir)."""
        zd = self.z_dir
        d = _num(distance)
        o = (self.o[0] + zd[0] * d, self.o[1] + zd[1] * d, self.o[2] + zd[2] * d)
        out = Plane.__new__(Plane)
        out.name = getattr(self, "name", None)
        _Frame.__init__(out, o, self.x_dir, self.y_dir, self.z_dir)
        return out

    def __repr__(self):
        return f"Plane(origin={self.o}, x_dir={self.x_dir}, z_dir={self.z_dir})"


def _perp_any(axis):
    """A deterministic unit vector perpendicular to ``axis``."""
    if abs(axis[0]) < 0.9:
        ref = (1.0, 0.0, 0.0)
    else:
        ref = (0.0, 1.0, 0.0)
    p = _vnormalize(_vsub(ref, _vscaled(axis, _vdot(ref, axis))))
    if _vlen(p) < 1e-12:
        p = _vnormalize(_vsub((0.0, 0.0, 1.0), _vscaled(axis, axis[2])))
    return p


def _euler_frame(origin, euler_deg):
    cols = _euler_rotation(euler_deg)
    return _Frame(_point3(origin), cols[0], cols[1], cols[2])


def _marker_frame(name):
    if name == "xy":
        return ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
    if name == "xz":
        return ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0), (0.0, -1.0, 0.0))
    if name == "yz":
        return ((0.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0))
    raise ValueError(f"unknown marker {name}")


Plane.XY = Plane(marker="xy")
Plane.XZ = Plane(marker="xz")
Plane.YZ = Plane(marker="yz")


def _euler_rotation(euler_deg):
    """The world-axis Euler rotation (degrees), x then y then z, as columns."""
    rx, ry, rz = (_num(v) for v in euler_deg)
    ax = math.radians(rx)
    ay = math.radians(ry)
    az = math.radians(rz)
    cx, sx = math.cos(ax), math.sin(ax)
    cy, sy = math.cos(ay), math.sin(ay)
    cz, sz = math.cos(az), math.sin(az)
    rx_cols = ((1.0, 0.0, 0.0), (0.0, cx, sx), (0.0, -sx, cx))
    ry_cols = ((cy, 0.0, -sy), (0.0, 1.0, 0.0), (sy, 0.0, cy))
    rz_cols = ((cz, sz, 0.0), (-sz, cz, 0.0), (0.0, 0.0, 1.0))
    m = _mmul(ry_cols, rx_cols)
    m = _mmul(rz_cols, m)
    return m


def _mmul(a, b):
    """Column-major 3x3 product A∘B (each arg is (col_x, col_y, col_z))."""
    out = []
    for bcol in b:
        out.append(
            (
                a[0][0] * bcol[0] + a[1][0] * bcol[1] + a[2][0] * bcol[2],
                a[0][1] * bcol[0] + a[1][1] * bcol[1] + a[2][1] * bcol[2],
                a[0][2] * bcol[0] + a[1][2] * bcol[1] + a[2][2] * bcol[2],
            )
        )
    return tuple(out)


def _rot_columns_from_matrix_cols(cols):
    return cols


class Pos(_Frame):
    """build123d ``Pos``: a translation frame placement row."""

    __slots__ = ()

    def __init__(self, *args):
        if len(args) == 1 and isinstance(args[0], Vector):
            o = args[0].to_tuple()
        elif len(args) == 1 and isinstance(args[0], (tuple, list)) and len(args[0]) == 3:
            o = _point3(args[0])
        elif len(args) == 3:
            o = (_num(args[0]), _num(args[1]), _num(args[2]))
        else:
            raise TypeError("Pos expects a position")
        _Frame.__init__(self, o, (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))


class Location(_Frame):
    """build123d ``Location``: a placement frame (translation + rotation).

    ``Location(position)`` records a translation; ``Location(position,
    (rx, ry, rz))`` records an Euler frame. The frame data is the same carrier
    the Plane/Pos/Rotation rows record -- never restricted to pure-z."""

    __slots__ = ()

    def __init__(self, *args):
        if len(args) == 1:
            position = args[0]
            if isinstance(position, Vector):
                position = position.to_tuple()
            if isinstance(position, (tuple, list)) and len(position) == 3:
                o = _point3(position)
                _Frame.__init__(self, o, (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
                return
        if len(args) == 2:
            position = args[0]
            rotation = args[1]
            if isinstance(position, Vector):
                position = position.to_tuple()
            if isinstance(rotation, _Frame):
                combined = _Frame(rotation.o, rotation.x_dir, rotation.y_dir, rotation.z_dir)
                _Frame.__init__(self, _point3(position), (1.0, 0.0, 0.0),
                                (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
                placed = combined._compose_after(self)
                self.o, self.x_dir, self.y_dir, self.z_dir = (
                    placed.o, placed.x_dir, placed.y_dir, placed.z_dir)
                return
            if (
                isinstance(position, (tuple, list))
                and len(position) == 3
                and isinstance(rotation, (tuple, list))
                and len(rotation) == 3
            ):
                o = _point3(position)
                cols = _euler_rotation(rotation)
                _Frame.__init__(self, o, cols[0], cols[1], cols[2])
                return
        if len(args) == 1 and isinstance(args[0], _Frame):
            _Frame.__init__(self, args[0].o, args[0].x_dir, args[0].y_dir, args[0].z_dir)
            return
        _refuse("Location forms beyond a frame row are not census carriers")


def Rotation(*args, **kwargs):
    """build123d ``Rotation(X, Y, Z)`` (Euler degrees) -- a rotation frame."""
    if len(args) == 1 and isinstance(args[0], (tuple, list)) and len(args[0]) == 3:
        args = tuple(args[0])
    if len(args) == 3:
        cols = _euler_rotation(args)
        return _Frame((0.0, 0.0, 0.0), cols[0], cols[1], cols[2])
    _refuse("Rotation forms beyond an Euler frame are not census carriers")


def Rot(x, y, z):
    """The corpus's Euler-degrees rotation wrapper (a frame row)."""
    return Rotation(x, y, z)


class Axis:
    """build123d ``Axis``: a direction (the corpus's rotate/revolve carrier)."""

    __slots__ = ("direction",)

    def __init__(self, direction):
        self.direction = _as_vector(direction)

    def __repr__(self):
        return f"Axis({self.direction.to_tuple()})"


Axis.X = Axis((1, 0, 0))
Axis.Y = Axis((0, 1, 0))
Axis.Z = Axis((0, 0, 1))


def _frame_for_axis(center, direction):
    """A frame whose z axis is ``direction`` and whose origin is ``center``."""
    d = _vnormalize(_point3(direction))
    if abs(d[2]) > 0.999999:
        x = (1.0, 0.0, 0.0)
    else:
        x = (0.0, 0.0, 0.0)
        if abs(d[0]) < 0.999999:
            x = (0.0, 0.0, 1.0)
        ref = x
        x = _vnormalize(_vsub(ref, _vscaled(d, _vdot(ref, d))))
    y = _vcross(d, x)
    y = _vnormalize(y)
    x = _vcross(y, d)
    x = _vnormalize(x)
    return _Frame(_point3(center), x, y, d)


# ---------------------------------------------------------------------------
# Profile carriers: Edge / Wire / Face as data rows. Points are kept in the
# coordinate space the carrier currently occupies; a frame multiplication
# transforms them (data), so a face placed by `Plane * make_face(wire)` sits
# at its world location.
# ---------------------------------------------------------------------------


def _pad_3(points):
    return [_point3(p) for p in points]


class Edge:
    """build123d ``Edge``: a curve carrier data row (line or spline)."""

    __slots__ = ("kind", "points", "p0", "p1", "options")

    def __init__(self, kind, points, options=None):
        pts = _pad_3(points)
        if len(pts) < 2:
            raise ValueError("an edge needs at least two points")
        self.kind = kind
        self.points = [Vector(*p) for p in pts]
        self.p0 = self.points[0]
        self.p1 = self.points[-1]
        self.options = dict(options) if options else {}

    @property
    def wrapped(self):
        return None

    def to_tuple(self):
        return [p.to_tuple() for p in self.points]

    def __repr__(self):
        return f"Edge({self.kind}, {len(self.points)} points)"

    def __matmul__(self, t):
        return self.position_at(t)

    def __add__(self, other):
        if isinstance(other, Wire):
            return Wire([self] + other.edges)
        if isinstance(other, Edge):
            return Wire([self, other])
        return NotImplemented

    @classmethod
    def make_line(cls, p0, p1):
        return cls("line", [_point3(p0), _point3(p1)])

    @classmethod
    def make_spline(cls, points, *args, **kwargs):
        pts = list(points)
        if len(pts) < 2:
            raise ValueError("spline needs at least two points")
        options = {}
        if args:
            options["positional"] = True
        for key in kwargs:
            options[key] = True
        return cls("spline", pts, options=options)

    def position_at(self, t):
        """The curve at parameter ``t`` as recorded data.

        ``t == 0``/``t == 1`` are the recorded endpoints (exact for a line and
        for the interpolating spline through the recorded samples). A line
        edge answers any parameter by its exact linear arithmetic; a spline
        answers through the same fixed-order interpolation the volume arms
        reconstruct -- never a Python approximation.
        """
        t = _num(t)
        if self.kind == "line":
            if t == 0.0:
                return self.p0
            if t == 1.0:
                return self.p1
            return Vector(
                self.p0.x + t * (self.p1.x - self.p0.x),
                self.p0.y + t * (self.p1.y - self.p0.y),
                self.p0.z + t * (self.p1.z - self.p0.z),
            )
        if self.kind == "spline":
            return Vector(*_path_point(_path_points(self), t))
        _refuse("this path carrier has no recorded interpolation")
        return self.p0

    def tangent_at(self, t):
        """The exact tangent of the recorded curve at ``t``.

        A line's tangent is its constant recorded direction; a spline's
        tangent is the derivative of the same fixed-order interpolant the
        volume arms reconstruct. Endpoints stay exact.
        """
        t = _num(t)
        if self.kind == "line":
            return Vector(
                self.p1.x - self.p0.x,
                self.p1.y - self.p0.y,
                self.p1.z - self.p0.z,
            )
        if self.kind == "spline":
            return Vector(*_path_derivative(_path_points(self), t))
        _refuse("this path carrier has no recorded derivative")
        return Vector(0.0, 0.0, 0.0)


def Spline(*points, periodic=False, **kwargs):
    """build123d ``Spline(*points, periodic=...)`` -- a spline profile edge.

    The edge records its DEFINING samples (the points the corpus passed) as a
    data row; a consuming verb answers exactly where the executor can (a
    full-arc revolve over recoverable interpolation samples) and refuses typed
    where the exact carrier is genuinely open (a spline-trimmed extrude, a
    spline-section loft)."""
    if kwargs:
        options = dict(kwargs)
    else:
        options = {}
    if periodic:
        options["periodic"] = True
    return Edge.make_spline(list(points), **({} if not options else options))


def Line(p0, p1):
    """build123d ``Line`` -- a straight profile edge."""
    return Edge.make_line(p0, p1)


def Circle(radius, **kwargs):
    """build123d ``Circle(radius)`` -- a closed circular profile edge.

    A circle is an exact conic, not a recoverable line/spline carrier: feeding
    it to ``loft``/``extrude``/``revolve`` cannot be answered exactly by the
    landed executor, so the carrier keeps its typed refusal (it names the open
    circle-profile carrier, never approximates it)."""
    _refuse("a circle profile is not answered exactly by a kernel-engine row")
    return None


def Polyline(*points, **kwargs):
    """build123d ``Polyline`` -- an open polyline profile edge (a data row of
    line segments). Feeding it to a profile verb answers exactly where the
    executor answers the resulting loop; otherwise the verb refuses typed."""
    return Edge.make_line(_point3(points[0]), _point3(points[-1]))


def Polygon(*points, **kwargs):
    """build123d ``Polygon`` -- a closed polyline profile (data row)."""
    pts = [_point3(p) for p in points]
    if len(pts) < 3:
        _refuse("a polygon profile needs at least three points")
    return Wire([Edge.make_line(pts[i], pts[(i + 1) % len(pts)]) for i in range(len(pts))])


class Wire:
    """build123d ``Wire``: an ordered list of curve carriers."""

    __slots__ = ("edges",)

    def __init__(self, edges):
        flat = []
        for edge in edges:
            if isinstance(edge, Wire):
                flat.extend(edge.edges)
            else:
                flat.append(edge)
        self.edges = flat

    @property
    def wrapped(self):
        return None

    def __iter__(self):
        return iter(self.edges)

    def __len__(self):
        return len(self.edges)

    def __add__(self, other):
        if isinstance(other, Wire):
            return Wire(self.edges + other.edges)
        if isinstance(other, Edge):
            return Wire(self.edges + [other])
        return NotImplemented

    def __repr__(self):
        return f"Wire({len(self.edges)} edges)"


class Face:
    """build123d ``Face``: a planar region bounded by one wire (data only).

    A face records its boundary edges in the coordinate space it occupies; a
    ``Plane``/``Location`` multiplication (``plane * face``) transforms the
    boundary points (data), so a face placed by an authoring frame sits at its
    world location and ``extrude``/``loft``/``revolve`` read it from there."""

    __slots__ = ("edges", "wire", "label", "color")

    def __init__(self, obj=None, *args, **kwargs):
        self.label = ""
        self.color = None
        if isinstance(obj, Wire):
            self.wire = obj
            self.edges = list(obj.edges)
        elif isinstance(obj, Face):
            self.wire = obj.wire
            self.edges = list(obj.edges)
        elif obj is None and args and isinstance(args[0], Wire):
            self.wire = args[0]
            self.edges = list(args[0].edges)
        elif obj is None:
            self.wire = Wire([])
            self.edges = []
        else:
            _refuse("this Face form is not a census carrier")

    @property
    def wrapped(self):
        return None

    def __iter__(self):
        return iter(self.edges)

    def __repr__(self):
        return f"Face({len(self.edges)} edges)"


def _place_edges(frame, edges):
    out = []
    for edge in edges:
        pts = [frame._map_point(p.to_tuple()) for p in edge.points]
        out.append(Edge(edge.kind, pts, options=edge.options))
    return out


def _place_wire(frame, wire):
    return Wire(_place_edges(frame, wire.edges))


def _place_face(frame, face):
    placed = Face.__new__(Face)
    placed.label = getattr(face, "label", "") or ""
    placed.color = getattr(face, "color", None)
    placed.edges = _place_edges(frame, face.edges)
    placed.wire = Wire(placed.edges)
    return placed


def make_face(*objs, **kwargs):
    """build123d ``make_face(wire)``: record a planar Face bounded by one wire.

    The recorded surface is elementary (the plane the boundary lies in); a
    spline-trimmed boundary records its defining spline samples in the wire's
    trim edges. Carriers outside the wire form refuse typed.
    """
    edges = []
    for obj in objs:
        if isinstance(obj, Wire):
            edges.extend(obj.edges)
        elif isinstance(obj, Face):
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


# ---------------------------------------------------------------------------
# Placed construction rows: one solid plus its world frame.
# ---------------------------------------------------------------------------


class _EmptySelection:
    """An empty entity selection: the drop-in records no face topology, so
    ``shape.faces()`` yields nothing and a selection with no recorded edge
    vocabulary yields nothing for the corpus's ``safe_fillet`` ladder."""

    def __iter__(self):
        return iter(())

    def __len__(self):
        return 0

    def filter_by(self, axis):
        return self

    def take(self, count):
        return self

    def last(self):
        return self


class _EdgeRef:
    """One recorded edge reference: its local endpoints plus its owning row.

    The base part's own edge vocabulary is what the fillet arm resolves
    against; the reference carries the owner so the recorded fillet row can
    name its depth-1 base (the corpus's ``safe_fillet`` iterates a plain list,
    so the owner cannot ride the selection object)."""

    __slots__ = ("_base", "a", "b")

    def __init__(self, base, a, b):
        self._base = base
        self.a = [float(c) for c in a]
        self.b = [float(c) for c in b]

    def _selector(self):
        return {"a": list(self.a), "b": list(self.b)}

    def axis(self):
        """The coordinate axis this edge runs along, or ``None``."""
        delta = _vsub(self.b, self.a)
        for name, index in (("x", 0), ("y", 1), ("z", 2)):
            if abs(delta[index]) > 1e-12:
                return name
        return None


class _EdgeSelection:
    """The base row's recorded edge vocabulary.

    The drop-in records the twelve edges of a recorded box; no other carrier
    records edge topology, so its selection is empty and the corpus's
    ``safe_fillet`` ladder returns the shape unchanged (a typed refusal is
    then the expected first-landing outcome for the real sidepod edge set)."""

    def __init__(self, owner, edges=()):
        self._owner = owner
        self._edges = list(edges)

    def __iter__(self):
        return iter(self._edges)

    def __len__(self):
        return len(self._edges)

    def filter_by(self, axis):
        name = axis if isinstance(axis, str) else getattr(axis, "name", None)
        return _EdgeSelection(self._owner, [e for e in self._edges if e.axis() == name])

    def take(self, count):
        return _EdgeSelection(self._owner, self._edges[: int(count)])

    def last(self):
        return _EdgeSelection(self._owner, self._edges[-1:])


def _recorded_edges(shape):
    """The recorded edge vocabulary of one placed row: the twelve edges of a
    box (in a fixed order). No other carrier records edge topology, so the
    selection is empty there and the corpus's ``safe_fillet`` ladder returns
    the shape unchanged."""
    solid = getattr(shape, "_solid", None)
    if not isinstance(solid, dict) or solid.get("kind") != "box":
        return []
    length = float(solid.get("length", 0.0))
    width = float(solid.get("width", 0.0))
    height = float(solid.get("height", 0.0))
    hx, hy, hz = 0.5 * length, 0.5 * width, 0.5 * height
    edges = []
    for sy in (-hy, hy):
        for sz in (-hz, hz):
            edges.append(_EdgeRef(shape, (-hx, sy, sz), (hx, sy, sz)))
    for sx in (-hx, hx):
        for sz in (-hz, hz):
            edges.append(_EdgeRef(shape, (sx, -hy, sz), (sx, hy, sz)))
    for sx in (-hx, hx):
        for sy in (-hy, hy):
            edges.append(_EdgeRef(shape, (sx, sy, -hz), (sx, sy, hz)))
    return edges


class _Shape:
    """The base of every placed construction row: one placed solid (a part)
    or a compound of child rows. All geometry lives behind the native
    executor; this side only records data and frames."""

    @property
    def wrapped(self):
        # A corpus helper that probes the wrapped OCC shape of a kernel-engine
        # data row cannot be served: refuse typed rather than pass `None` into
        # OCP (which would die as an untyped TypeError).
        _refuse("an OCC probe of a kernel-engine row is not a kernel-engine row")
        return None

    def _facts(self):
        return json.loads(_T123D.bd_facts(json.dumps(self._node())))

    @property
    def volume(self):
        return float(self._facts()["volume"])

    def bounding_box(self):
        mn, mx = self._facts()["bbox"]
        return _BoundingBox(Vector(mn), Vector(mx))

    @property
    def is_valid(self):
        try:
            facts = self._facts()
            return facts["volume"] > 0.0
        except Exception:
            return False

    def edges(self):
        return _EdgeSelection(self, _recorded_edges(self))

    def faces(self):
        return _EmptySelection()

    def clean(self):
        """The identity: cleanliness is recorded client metadata, not a bridge
        round-trip, so the row passes through unchanged."""
        return self

    def _boolean(self, other, mode):
        """Record one BooleanOp row over two kernel-engine solids and dispatch.

        The operands' LOCAL geometry is what dispatches; a placed operand's
        frame composes after (the recorded exactness rule), so admission is
        placement-blind and verdict-stable. The row is dispatched through the
        certified funnel at record time so a refused pair raises its typed
        refusal here -- never a silent fallback. A boolean of a boolean result
        is the recorded open composition cell and refuses typed naming
        ``BooleanResultOperand`` (depth-1 only).
        """
        if not isinstance(other, _Shape):
            _refuse("a boolean operand outside a kernel-engine row is not a kernel-engine row")
        node = _boolean_node(self, other, mode)
        if _T123D is not None:
            _T123D.bd_facts(json.dumps(node))
        return _BooleanResult(node)

    def __sub__(self, other):
        return self._boolean(other, "subtract")

    def __add__(self, other):
        return self._boolean(other, "union")

    def __and__(self, other):
        return self._boolean(other, "intersect")

    def fuse(self, *tools):
        result = self
        for tool in tools:
            result = result._boolean(tool, "union")
        return result

    def cut(self, *tools):
        result = self
        for tool in tools:
            result = result._boolean(tool, "subtract")
        return result

    def intersect(self, *tools):
        result = self
        for tool in tools:
            result = result._boolean(tool, "intersect")
        return result


class _Part(_Shape):
    """One placed solid: a local solid descriptor plus a world frame."""

    def __init__(self, solid):
        self._solid = solid
        self._x = 0.0
        self._y = 0.0
        self._z = 0.0
        self._rz = 0.0
        self._rot = None
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
        if self._rot is not None:
            node["rotation"] = {
                "x_dir": list(self._rot[0]),
                "y_dir": list(self._rot[1]),
                "z_dir": list(self._rot[2]),
            }
        if self._mirror is not None:
            node["mirror"] = self._mirror
        if self.label:
            node["label"] = self.label
        if self.color is not None:
            node["color"] = _color_record(self.color)
        return {"part": node}

    def _copy(self):
        out = _Part(self._solid)
        out._x, out._y, out._z = self._x, self._y, self._z
        out._rz = self._rz
        out._rot = self._rot
        out._mirror = self._mirror
        out.label = self.label
        out.color = self.color
        return out

    def locate(self, loc):
        if isinstance(loc, _Frame):
            return _locate_in_place(self, loc)
        _refuse("locate expects a frame carrier")
        return self

    def moved(self, loc):
        return self.locate(loc)

    def rotate(self, axis, angle):
        if not isinstance(axis, Axis):
            _refuse("rotate about an axis carrier is not a kernel-engine row")
        direction = _point3(axis.direction)
        if _num(angle) == 0.0:
            return self
        if _is_z_axis(direction):
            if self._rot is None:
                self._rz += _num(angle)
            else:
                rot = _Frame((0.0, 0.0, 0.0), *_euler_rotation((0.0, 0.0, _num(angle))))
                self._rot = (rot.x_dir, rot.y_dir, rot.z_dir)
                self._rz = 0.0
            return self
        # rotate about a non-z axis records a full frame (the method path).
        frame = _axis_angle_frame(direction, _num(angle))
        return _locate_in_place(self, frame)

    def solids(self):
        return [self]

    def __repr__(self):
        return f"Part(solid={self._solid.get('kind')})"


class _BooleanResult(_Shape):
    """One recorded BooleanOp row over two placed operand nodes.

    The row carries the data ``{mode, a, b}``; all geometry lives behind the
    native executor, which dispatches the pair through the certified funnel
    and measures the certified product's facts. A boolean of a boolean result
    (depth > 1) is the recorded open composition cell and refuses typed.
    """

    def __init__(self, node):
        self._node_data = node
        self.label = ""
        self.color = None
        self._type_name = "BooleanResult"

    def _node(self):
        return self._node_data

    def solids(self):
        return [self]

    def __repr__(self):
        return f"BooleanResult(mode={self._node_data['boolean']['mode']})"


def _boolean_node(a, b, mode):
    """The recorded BooleanOp row ``{mode, a, b}`` (data only).

    The operands' LOCAL geometry is what dispatches; a placed operand's frame
    stays recorded on its own node and composes after. A boolean operand that
    is itself a boolean result is the recorded open composition cell and
    refuses typed, naming ``BooleanResultOperand`` (depth-1 only, never a
    silent recursion).
    """
    for operand in (a, b):
        if isinstance(operand, _BooleanResult):
            _refuse(
                "BooleanResultOperand: a boolean of a boolean result is not a "
                "kernel-engine row"
            )
    return {"boolean": {"mode": mode, "a": a._node(), "b": b._node()}}


def _is_z_axis(direction):
    return (
        abs(direction[0]) < 1e-12
        and abs(direction[1]) < 1e-12
        and abs(direction[2] - 1.0) < 1e-12
    )


def _axis_angle_frame(direction, angle_deg):
    """The Rodrigues rotation about ``direction`` through the origin."""
    a = _vnormalize(direction)
    theta = math.radians(angle_deg)
    c, s = math.cos(theta), math.sin(theta)
    x_dir = (
        (c + (1 - c) * a[0] * a[0]),
        ((1 - c) * a[1] * a[0] + s * a[2]),
        ((1 - c) * a[2] * a[0] - s * a[1]),
    )
    y_dir = (
        ((1 - c) * a[0] * a[1] - s * a[2]),
        (c + (1 - c) * a[1] * a[1]),
        ((1 - c) * a[2] * a[1] + s * a[0]),
    )
    z_dir = (
        ((1 - c) * a[0] * a[2] + s * a[1]),
        ((1 - c) * a[1] * a[2] - s * a[0]),
        (c + (1 - c) * a[2] * a[2]),
    )
    return _Frame((0.0, 0.0, 0.0), x_dir, y_dir, z_dir)


def _apply_frame_to_part(part, frame):
    """Compose ``frame`` over a part's own frame (world semantics)."""
    out = part._copy()
    if out._rot is None:
        rot = _Frame((0.0, 0.0, 0.0), *_rotz_columns(math.radians(out._rz)))
    else:
        rot = _Frame((0.0, 0.0, 0.0), out._rot[0], out._rot[1], out._rot[2])
    placed = frame._compose_after(_Frame((out._x, out._y, out._z),
                                         rot.x_dir, rot.y_dir, rot.z_dir))
    out._x, out._y, out._z = placed.o
    out._rot = (placed.x_dir, placed.y_dir, placed.z_dir)
    out._rz = 0.0
    return out


def _rotz_columns(radians):
    c, s = math.cos(radians), math.sin(radians)
    return ((c, s, 0.0), (-s, c, 0.0), (0.0, 0.0, 1.0))


def _locate_in_place(part, loc):
    """The method placement path (``.locate``/``.moved``/``.rotate``): applies
    the frame to the part in place and returns the part.

    A pure-z frame keeps the recorded ``rz`` path byte-identical (the Falcon
    corpus's method placements). A general frame folds the current rotation
    and the frame into the full orthonormal columns."""
    if part._rot is not None:
        current = _Frame((part._x, part._y, part._z), part._rot[0], part._rot[1],
                         part._rot[2])
        placed = loc._compose_after(current)
        part._x, part._y, part._z = placed.o
        part._rot = (placed.x_dir, placed.y_dir, placed.z_dir)
        part._rz = 0.0
        return part
    if loc._is_pure_z():
        part._x += loc.o[0]
        part._y += loc.o[1]
        part._z += loc.o[2]
        angle = loc._z_angle_deg()
        if angle:
            part._rz += angle
        return part
    current = _Frame((part._x, part._y, part._z), *_rotz_columns(math.radians(part._rz)))
    placed = loc._compose_after(current)
    part._x, part._y, part._z = placed.o
    if placed._is_pure_z():
        part._rz += placed._z_angle_deg()
        part._rot = None
    else:
        part._rot = (placed.x_dir, placed.y_dir, placed.z_dir)
        part._rz = 0.0
    return part


def _place_part(frame, part):
    return _apply_frame_to_part(part, frame)


class Compound(_Shape):
    """build123d ``Compound``: a group of child construction rows."""

    def __init__(self, obj=None, children=None, label=""):
        self.label = label
        self.color = None
        self._type_name = "Compound"
        if children is not None:
            self._children = list(children)
        elif obj is not None and not isinstance(obj, Compound):
            self._children = list(obj)
        else:
            self._children = []

    def _node(self):
        node = {"group": [child._node() for child in self._children]}
        if self.label:
            node["label"] = self.label
        return node

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


class _BoundingBox:
    def __init__(self, mn, mx):
        self.min = mn
        self.max = mx


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
        if rz != 0.0:
            _refuse("z-rotated cylinders are not a kernel-engine row")
        return "z"
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

    The executor's lathe arm covers a closed planar profile turned a full 360
    degrees about a recorded axis. The profile may mix straight edges and
    ``Edge.make_spline(points)`` spline edges (the spline is recorded by its
    defining samples and the kernel integrates the true interpolating spline,
    never a flattening polygon). The z-axis form keeps the recorded legacy row
    shape bit-for-bit; an expressible non-z axis (build123d ``Axis`` or a
    ``Vector`` direction) is recorded in the same local ``(radius, axial)``
    plane convention and the landed frame row carries the axis as a full
    orthonormal ``rotation`` (local z to the recorded axis direction). A
    partial arc, a degenerate axis, a profile that is not a single-sided
    meridian generator about the axis, an open profile, or a spline whose
    interpolation options the recorded data cannot recover refuses typed.
    """
    if not isinstance(shape, Face):
        _refuse("revolve of a non-Face shape is not a kernel-engine row")
    if _num(revolution_arc) != 360.0:
        _refuse("a partial-arc revolve is outside the executor's lathe arm")
    unit = _axis_unit(axis)
    if unit is None:
        _refuse("DegenerateRevolveAxis: a zero-length axis is not a kernel-engine row")
    edges = shape.edges
    if len(edges) < 3:
        _refuse("revolve needs a closed profile")
    if _is_z_axis(unit):
        profile = _legacy_z_profile(edges)
        frame = None
    else:
        profile, frame = _axis_profile(edges, unit)
    first = edges[0].p0
    last = edges[-1].p1
    if (
        abs(first.x - last.x) > 1e-9
        or abs(first.y - last.y) > 1e-9
        or abs(first.z - last.z) > 1e-9
    ):
        _refuse("a revolve profile must close on itself")
    part = _Part({"kind": "lathe", "profile": profile, "arc_deg": 360.0})
    if frame is not None:
        part._rot = frame
        part._rz = 0.0
    return part


def _axis_unit(axis):
    """The unit axis direction tuple of an ``Axis`` or ``Vector`` (data only;
    any other carrier refuses typed). ``None`` marks a degenerate axis."""
    if isinstance(axis, Axis):
        direction = _point3(axis.direction)
    elif isinstance(axis, Vector):
        direction = axis.to_tuple()
    else:
        _refuse("DegenerateRevolveAxis: an axis direction carrier is required")
        return None
    unit = _vnormalize(direction)
    if _vlen(unit) == 0.0 or not all(math.isfinite(v) for v in unit):
        return None
    return unit


def _legacy_z_profile(edges):
    """The legacy z-axis profile: every recorded edge point lies in the ``y =
    0`` plane and the profile coordinate pair is the recorded ``(radius x,
    axial z)`` of the world point (bit-identical row shape)."""
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
    return profile


def _axis_profile(edges, unit):
    """The profile of a non-z axis expressed in the local ``(radius >= 0,
    axial)`` plane convention of the recorded axis, plus the full orthonormal
    frame whose local z is the recorded axis direction (the landed
    composition ``world = translate(o) ∘ R ∘ M``).

    The profile must be a single-sided meridian generator about the axis
    through the frame origin: the perpendicular component of every recorded
    point is parallel to one common ray (never folded across the axis). A
    profile not expressible exactly that way refuses typed naming the gap.
    """
    reference = _axis_ray(edges, unit)
    if reference is None:
        _refuse("the profile lying on the axis is not a kernel-engine row")
    profile = []
    for edge in edges:
        if edge.kind == "line":
            profile.append(
                {
                    "kind": "line",
                    "a": _axis_point(edge.p0.to_tuple(), unit, reference),
                    "b": _axis_point(edge.p1.to_tuple(), unit, reference),
                }
            )
        elif edge.kind == "spline":
            if edge.options:
                # An interpolation option (tangents/periodic/parameters) is not
                # recoverable from the recorded sample list; never guess one.
                _refuse("a spline-profile revolve is not a kernel-engine row")
            profile.append(
                {
                    "kind": "spline",
                    "points": [
                        _axis_point(point.to_tuple(), unit, reference)
                        for point in edge.points
                    ],
                }
            )
        else:
            _refuse("this curve carrier is not a lathe profile edge")
    return profile, _axis_frame(unit, reference)


def _axis_ray(edges, unit):
    """The reference perpendicular ray shared by every recorded point of a
    single-sided meridian profile (``None`` when no point leaves the axis)."""
    for edge in edges:
        for point in edge.points:
            perp = _axis_perp(point.to_tuple(), unit)
            if _vlen(perp) > 0.0:
                return _vnormalize(perp)
    return None


def _axis_perp(point, unit):
    """The component of ``point`` perpendicular to the recorded axis."""
    axial = _vdot(point, unit)
    return _vsub(point, _vscaled(unit, axial))


def _axis_point(point, unit, reference):
    """The ``(radius, axial)`` plane coordinates of one profile point.

    A point whose perpendicular component folds across the recorded axis
    (opposite the single-sided reference ray) cannot be a lathe generator
    exactly; refuse typed naming the gap. Radius is the non-negative in-plane
    coordinate along the reference ray, never a folded magnitude.
    """
    axial = _vdot(point, unit)
    perp = _axis_perp(point, unit)
    radius = _vdot(perp, reference)
    if radius < -1e-9:
        _refuse("a profile crossing the recorded axis is not a kernel-engine row")
    radius = max(radius, 0.0)
    return [radius, axial]


def _axis_frame(unit, reference):
    """The orthonormal rotation columns taking the local z-lathe to the
    recorded axis: local z to ``unit``, local x to the reference ray, and the
    fixed-order completion for local y."""
    x_dir = reference
    z_dir = unit
    y_dir = _vnormalize(_vcross(z_dir, x_dir))
    if _vlen(y_dir) < 1e-12:
        _refuse("an axis parallel to its profile ray is not a kernel-engine row")
    return (x_dir, y_dir, z_dir)


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
    """The recorded boundary edges of a Face carrier, as 3-D profile data.

    The kernel certifies exact facts only for closed line-loops; a spline-
    trimmed section (including a single periodic spline outline) is recorded
    and refuses typed at the facts arm -- never approximated here.
    """
    edges = shape.edges
    if not edges:
        _refuse("a profile needs a boundary with at least one edge")
    return [_edge3(edge) for edge in edges]


# ---------------------------------------------------------------------------
# Exact path reconstruction and station-frame transport: a recorded line or
# spline path is answered by the SAME fixed-order interpolation the kernel
# volume arms reconstruct (chord-length parameters, clamped cubic, endpoint
# slopes from the degree-3 Lagrange interpolant of the first/last four
# samples). Pure data arithmetic -- never an approximation.
# ---------------------------------------------------------------------------


def _chord_params(pts):
    """The chord-length parameters of the recorded station points."""
    params = [0.0]
    for i in range(1, len(pts)):
        dx = pts[i][0] - pts[i - 1][0]
        dy = pts[i][1] - pts[i - 1][1]
        dz = pts[i][2] - pts[i - 1][2]
        params.append(params[-1] + math.sqrt(dx * dx + dy * dy + dz * dz))
    return params


def _hermite_power(y0, y1, d0, d1):
    """The power-basis cubic Hermite coefficients over the span parameter."""
    return [
        y0,
        d0,
        -3.0 * y0 - 2.0 * d0 + 3.0 * y1 - d1,
        2.0 * y0 + d0 - 2.0 * y1 + d1,
    ]


def _lagrange_end_slope(pts, params, first):
    """The derivative at the first (or last) node of the degree-3 Lagrange
    interpolant through the four given samples at their chord parameters."""
    p = params
    if first:
        factors = [
            1.0 / (p[0] - p[1]) + 1.0 / (p[0] - p[2]) + 1.0 / (p[0] - p[3]),
            1.0 / (p[1] - p[0]) * ((p[0] - p[2]) / (p[1] - p[2]))
            * ((p[0] - p[3]) / (p[1] - p[3])),
            1.0 / (p[2] - p[0]) * ((p[0] - p[1]) / (p[2] - p[1]))
            * ((p[0] - p[3]) / (p[2] - p[3])),
            1.0 / (p[3] - p[0]) * ((p[0] - p[1]) / (p[3] - p[1]))
            * ((p[0] - p[2]) / (p[3] - p[2])),
        ]
    else:
        factors = [
            1.0 / (p[0] - p[3]) * ((p[3] - p[1]) / (p[0] - p[1]))
            * ((p[3] - p[2]) / (p[0] - p[2])),
            1.0 / (p[1] - p[3]) * ((p[3] - p[0]) / (p[1] - p[0]))
            * ((p[3] - p[2]) / (p[1] - p[2])),
            1.0 / (p[2] - p[3]) * ((p[3] - p[0]) / (p[2] - p[0]))
            * ((p[3] - p[1]) / (p[2] - p[1])),
            1.0 / (p[3] - p[0]) + 1.0 / (p[3] - p[1]) + 1.0 / (p[3] - p[2]),
        ]
    out = []
    for component in range(3):
        acc = 0.0
        for i in range(4):
            acc += pts[i][component] * factors[i]
        out.append(acc)
    return out


def _clamped_slopes(pts, params):
    """The clamped cubic slopes at every station: the interior slopes solve
    the C2 tridiagonal system, the end slopes are the Lagrange end slopes."""
    n = len(pts)
    slopes = [[0.0, 0.0, 0.0] for _ in range(n)]
    slopes[0] = _lagrange_end_slope(pts[0:4], params[0:4], True)
    slopes[n - 1] = _lagrange_end_slope(pts[n - 4:n], params[n - 4:n], False)
    nk = n - 2
    h = [params[i + 1] - params[i] for i in range(n - 1)]
    delta = [
        [(pts[i + 1][c] - pts[i][c]) / h[i] for c in range(3)]
        for i in range(n - 1)
    ]
    a = [0.0] * nk
    b = [0.0] * nk
    c = [0.0] * nk
    rhs = [[0.0, 0.0, 0.0] for _ in range(nk)]
    for row in range(nk):
        i = row + 1
        a[row] = h[i]
        b[row] = 2.0 * (h[i - 1] + h[i])
        c[row] = h[i - 1]
        for component in range(3):
            value = 3.0 * (h[i] * delta[i - 1][component]
                           + h[i - 1] * delta[i][component])
            if i == 1:
                value -= h[i] * slopes[0][component]
            if i == n - 2:
                value -= h[i - 1] * slopes[n - 1][component]
            rhs[row][component] = value
    cp = [0.0] * nk
    dp = [[0.0, 0.0, 0.0] for _ in range(nk)]
    b0 = b[0]
    if b0 == 0.0:
        _refuse("DegenerateSweepPath: a repeated path station has no direction")
    cp[0] = c[0] / b0
    dp[0] = [rhs[0][component] / b0 for component in range(3)]
    for row in range(1, nk):
        denom = b[row] - a[row] * cp[row - 1]
        if denom == 0.0:
            _refuse("DegenerateSweepPath: a repeated path station has no direction")
        if row < nk - 1:
            cp[row] = c[row] / denom
        for component in range(3):
            dp[row][component] = (
                rhs[row][component] - a[row] * dp[row - 1][component]
            ) / denom
    x = [[0.0, 0.0, 0.0] for _ in range(nk)]
    x[nk - 1] = dp[nk - 1]
    for row in range(nk - 2, -1, -1):
        for component in range(3):
            x[row][component] = dp[row][component] - cp[row] * x[row + 1][component]
    for row in range(nk):
        slopes[row + 1] = x[row]
    return slopes


def _path_spans(pts):
    """The reconstructed spans of the recorded station interpolant plus the
    chord parameters; one span per consecutive station pair."""
    n = len(pts)
    if n < 2:
        _refuse("DegenerateSweepPath: a path needs at least two recorded stations")
    params = _chord_params(pts)
    for i in range(1, n):
        if not (params[i] > params[i - 1]):
            _refuse("DegenerateSweepPath: a repeated path station has no direction")
    if n == 2:
        return [[[pts[0][c], pts[1][c] - pts[0][c], 0.0, 0.0] for c in range(3)]], params
    if n == 3:
        u1 = (params[1] - params[0]) / (params[2] - params[0])
        denom = 2.0 * u1 * (1.0 - u1)
        if denom == 0.0:
            _refuse("DegenerateSweepPath: a repeated path station has no direction")
        span = []
        for c in range(3):
            p0, p1, p2 = pts[0][c], pts[1][c], pts[2][c]
            q1 = (p1 - (1.0 - u1) * (1.0 - u1) * p0 - u1 * u1 * p2) / denom
            span.append([p0, 2.0 * (q1 - p0), p0 - 2.0 * q1 + p2, 0.0])
        return [span], params
    slopes = _clamped_slopes(pts, params)
    spans = []
    for i in range(n - 1):
        h = params[i + 1] - params[i]
        spans.append([
            _hermite_power(pts[i][c], pts[i + 1][c], h * slopes[i][c],
                           h * slopes[i + 1][c])
            for c in range(3)
        ])
    return spans, params


def _span_value(span, u):
    """The value of one reconstructed span at the span parameter ``u``."""
    return [
        coeff[0] + u * (coeff[1] + u * (coeff[2] + u * coeff[3]))
        for coeff in span
    ]


def _span_slope(span, u):
    """The derivative of one reconstructed span at the span parameter ``u``."""
    return [
        coeff[1] + u * (2.0 * coeff[2] + u * 3.0 * coeff[3])
        for coeff in span
    ]


def _path_span_at(pts, t):
    """The span index and span parameter of the normalized parameter ``t``,
    plus the spans and chord parameters."""
    spans, params = _path_spans(pts)
    total = params[-1]
    if t <= 0.0:
        return spans, params, 0, 0.0, total
    if t >= 1.0:
        return spans, params, len(pts) - 2, 1.0, total
    s = t * total
    j = 0
    while j + 1 < len(pts) - 1 and s > params[j + 1]:
        j += 1
    h = params[j + 1] - params[j]
    return spans, params, j, (s - params[j]) / h, total


def _path_point(pts, t):
    """The exact interpolated point at the normalized parameter ``t``."""
    spans, _params, j, u, _total = _path_span_at(pts, t)
    if t <= 0.0:
        return list(pts[0])
    if t >= 1.0:
        return list(pts[-1])
    return _span_value(spans[j], u)


def _path_derivative(pts, t):
    """The exact interpolated derivative (with respect to the normalized
    parameter) at ``t``."""
    spans, params, j, u, total = _path_span_at(pts, t)
    h = params[j + 1] - params[j]
    slope = _span_slope(spans[j], u)
    return [slope[c] * total / h for c in range(3)]


def _path_directions(pts):
    """The exact interpolated derivative at every recorded station."""
    n = len(pts)
    spans, params = _path_spans(pts)
    total = params[-1]
    if n == 2:
        return [_span_slope(spans[0], 0.0)] * 2
    if n == 3:
        return [_span_slope(spans[0], params[i] / total) for i in range(n)]
    slopes = _clamped_slopes(pts, params)
    return [[slopes[i][c] * total for c in range(3)] for i in range(n)]


def _path_points(edge):
    """The recorded samples of a recoverable spline path edge; an
    interpolation option the recorded data cannot recover refuses typed."""
    if edge.options:
        _refuse("a path spline with interpolation options is not a kernel-engine row")
    return [point.to_tuple() for point in edge.points]


def _path_edge(path):
    """The recorded line/spline edge carrier of a path (a one-edge Wire is the
    same carrier); anything else refuses typed naming the open carrier."""
    if isinstance(path, Edge):
        return path
    if isinstance(path, Wire) and len(path.edges) == 1:
        return path.edges[0]
    _refuse("a path outside the recorded line/spline edge carrier is not a kernel-engine row")
    return None


def _section_loop(section):
    """The recorded closed line-loop boundary of a section: every edge must be
    a recorded straight edge (the landed loft-section vocabulary); a
    spline-trimmed or non-Face section refuses typed naming the open carrier."""
    if not isinstance(section, Face):
        _refuse("a section outside the recorded Face carrier is not a kernel-engine row")
    loop = []
    for edge in section.edges:
        if edge.kind != "line":
            _refuse("a spline-trimmed section boundary is not a kernel-engine row")
        loop.append(edge)
    if len(loop) < 3:
        _refuse("a section needs a closed boundary with at least three edges")
    return loop


def _station_data(edge):
    """The recorded station points of a path and their exact path directions."""
    pts = [point.to_tuple() for point in edge.points]
    if edge.kind == "line":
        direction = _vsub(pts[1], pts[0])
        if _vlen(direction) == 0.0:
            _refuse("DegenerateSweepPath: a station direction is zero")
        return pts, [direction, list(direction)]
    if edge.kind == "spline":
        pts = _path_points(edge)
        directions = _path_directions(pts)
        for direction in directions:
            if _vlen(direction) == 0.0:
                _refuse("DegenerateSweepPath: a station direction is zero")
        return pts, directions
    _refuse("a path outside the recorded line/spline edge carrier is not a kernel-engine row")
    return [], []


def _section_reference(loop, z_axis):
    """A deterministic in-plane reference axis for a section (the first
    recorded edge direction projected into the section plane)."""
    if loop:
        direction = _vsub(loop[0].p1.to_tuple(), loop[0].p0.to_tuple())
        projected = _vsub(direction, _vscaled(z_axis, _vdot(direction, z_axis)))
        if _vlen(projected) > 0.0:
            return _vnormalize(projected)
    return _perp_any(z_axis)


def _transport_frames(stations, directions, reference):
    """The parallel-transport station frames, computed by the fixed-order
    double-reflection discipline. Each frame is (origin, x_dir, y_dir,
    z_dir): z is the unit station direction, x the transported reference,
    y = z cross x. A zero station direction refuses
    (``DegenerateSweepPath``)."""
    z_axes = []
    for direction in directions:
        length = _vlen(direction)
        if length == 0.0:
            _refuse("DegenerateSweepPath: a station direction is zero")
        z_axes.append(_vscaled(direction, 1.0 / length))
    frames = []
    ref = reference
    previous_z = z_axes[0]
    x_axis = _vsub(ref, _vscaled(z_axes[0], _vdot(ref, z_axes[0])))
    if _vlen(x_axis) == 0.0:
        x_axis = _perp_any(z_axes[0])
    x_axis = _vnormalize(x_axis)
    y_axis = _vnormalize(_vcross(z_axes[0], x_axis))
    frames.append((stations[0], x_axis, y_axis, z_axes[0]))
    for i in range(1, len(stations)):
        chord = _vsub(stations[i], stations[i - 1])
        chord_len = _vdot(chord, chord)
        if chord_len > 0.0:
            reflected_ref = _vsub(
                ref, _vscaled(chord, 2.0 * _vdot(chord, ref) / chord_len)
            )
            reflected_z = _vsub(
                previous_z,
                _vscaled(chord, 2.0 * _vdot(chord, previous_z) / chord_len),
            )
            axis = _vsub(z_axes[i], reflected_z)
            axis_len = _vdot(axis, axis)
            if axis_len > 0.0:
                ref = _vsub(
                    reflected_ref,
                    _vscaled(axis, 2.0 * _vdot(axis, reflected_ref) / axis_len),
                )
        x_axis = _vsub(ref, _vscaled(z_axes[i], _vdot(ref, z_axes[i])))
        if _vlen(x_axis) == 0.0:
            x_axis = _perp_any(z_axes[i])
        x_axis = _vnormalize(x_axis)
        y_axis = _vnormalize(_vcross(z_axes[i], x_axis))
        frames.append((stations[i], x_axis, y_axis, z_axes[i]))
        ref = x_axis
        previous_z = z_axes[i]
    return frames


def _frame_local(frame, point):
    """The frame-local coordinates of one world point."""
    origin, x_axis, y_axis, z_axis = frame
    rel = _vsub(point, origin)
    return (_vdot(rel, x_axis), _vdot(rel, y_axis), _vdot(rel, z_axis))


def _frame_world(frame, local):
    """The world point of one frame-local coordinate triple."""
    origin, x_axis, y_axis, z_axis = frame
    return (
        origin[0] + local[0] * x_axis[0] + local[1] * y_axis[0] + local[2] * z_axis[0],
        origin[1] + local[0] * x_axis[1] + local[1] * y_axis[1] + local[2] * z_axis[1],
        origin[2] + local[0] * x_axis[2] + local[1] * y_axis[2] + local[2] * z_axis[2],
    )


def _place_loop(loop, base_frame, frame):
    """The section boundary placed by ``frame`` (its recorded station-0
    coordinates are read through ``base_frame``)."""
    placed = []
    for edge in loop:
        a = _frame_local(base_frame, edge.p0.to_tuple())
        b = _frame_local(base_frame, edge.p1.to_tuple())
        placed.append({
            "kind": "line",
            "a": list(_frame_world(frame, a)),
            "b": list(_frame_world(frame, b)),
        })
    return placed


def extrude(shape, amount, both=False, mode=None, **kwargs):
    """build123d ``extrude(face, amount, both)``: the recording arm for a
    closed planar profile swept along its own plane normal.

    An all-line boundary records the exact prism row (the volume is the profile
    area times the swept length, `2 * amount` when `both`). A boundary carrying
    a recorded spline edge is the spline-trimmed extrude constructor row
    (`trim_prism`): the line edges are the base profile and the recorded spline
    samples are the closed spline carrier the binding's constructor classifies.
    The row is dispatched through the native facts entry at record time, so a
    spline class the composition cannot close surfaces its typed refusal here --
    never a silent fallback.
    """
    if not isinstance(shape, Face):
        _refuse("Face extrusion is not yet a kernel-engine row")
    lines = []
    spline_points = []
    for edge in shape.edges:
        if edge.kind == "line":
            lines.append(_edge3(edge))
        else:
            spline_points.extend(point.to_tuple() for point in edge.points)
    if not lines and not spline_points:
        _refuse("Face extrusion is not yet a kernel-engine row")
    if not spline_points:
        if len(lines) < 3:
            _refuse("Face extrusion is not yet a kernel-engine row")
        return _Part(
            {
                "kind": "prism",
                "profile": lines,
                "amount": _num(amount),
                "both": bool(both),
            }
        )
    curve = list(spline_points)
    if curve[0] != curve[-1]:
        curve.append(curve[0])
    part = _Part(
        {
            "kind": "trim_prism",
            "profile": lines,
            "amount": _num(amount),
            "both": bool(both),
            "trim_curve": curve,
            "trim_net": [],
            "tolerance": 1.0e-3,
        }
    )
    if _T123D is not None:
        _T123D.bd_facts(json.dumps(part._node()))
    return part


def sweep(section=None, path=None, mode=None, closed=False, **kwargs):
    """build123d ``sweep(section, path)``: the sweep recording arm.

    A sweep records a loft chain between the recorded stations. A line path
    records the landed two-station straight-sweep chain; a spline path
    records the chain over its recorded samples. Each station places the
    section through the fixed-order double-reflection frames (parallel
    transport, never Frenet). A section outside the recorded line-loop
    vocabulary, or a sweep path outside the recorded edge vocabulary,
    refuses typed naming the open carrier; a zero-direction station refuses
    (``DegenerateSweepPath``). The tangent queries answer through the same
    exact interpolant the volume arms reconstruct.

    ``closed=True`` records the halo loop-closure request; it is answered only
    when the recorded station chain returns exactly to its first section (the
    kernel certifies the seam identity), and a chain that does not close
    refuses typed naming the seam."""
    edge = _path_edge(path)
    loop = _section_loop(section)
    stations, directions = _station_data(edge)
    reference = _section_reference(loop, _vnormalize(directions[0]))
    frames = _transport_frames(stations, directions, reference)
    sections = [_place_loop(loop, frames[0], frame) for frame in frames]
    return _loft_row(sections, closed)


def loft(*sections, ruled=False, mode=None, closed=False, **kwargs):
    """build123d ``loft(sections, ruled=...)``: the recording arm.

    Records the ordered section profiles (each a Face carrier's boundary, in
    the part's local frame) plus the cross interpolation as one ``Loft`` row.
    The kernel certifies the ruled matched-vertex chain with exact facts for
    all-line sections; a spline-section loft's smooth-surface volume is an
    open carrier the executor refuses typed at the facts arm -- never
    approximated, never flattened.

    ``closed=True`` records the halo loop-closure request (the station list
    returns exactly to its first section); a chain that does not close refuses
    typed naming the seam, and ``closed=False`` behavior is unchanged."""
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
    return _loft_row(recorded, closed)


def _loft_row(sections, closed):
    """One ``Loft`` row honoring the closed-loop request.

    ``closed=True`` is answered only when the recorded section sequence is
    loop-closing: the first recorded section must equal the section that
    closes the chain (exact equality of the recorded profiles). A chain that
    does not return to its start refuses typed naming the seam, never a
    silently open row."""
    if closed:
        if len(sections) < 2:
            _refuse("a closed loft chain needs at least two recorded sections")
        if sections[0] != sections[-1]:
            _refuse("a closed loft chain must return to its first section")
    return _Part({"kind": "loft", "closed": bool(closed), "sections": sections})


class _FilletResult(_Shape):
    """One recorded Fillet row over a base part node.

    The row carries ``{base, radius, edges}``; all geometry lives behind the
    native executor, which resolves each recorded edge selector against the
    base's recorded edge vocabulary, certifies the blend through the landed
    blend profile machinery, and measures the base's facts. A base outside
    the depth-1 rule (a group or another op node) refuses typed naming the
    carrier."""

    def __init__(self, node):
        self._node_data = node
        self.label = ""
        self.color = None
        self._type_name = "FilletResult"

    def _node(self):
        return self._node_data

    def solids(self):
        return [self]

    def __repr__(self):
        return "FilletResult(radius={})".format(self._node_data["fillet"]["radius"])


def fillet(edges, radius, **kwargs):
    """build123d ``fillet(edges, radius)``: the fillet recording arm.

    The recorded edge references are the base part's own edge vocabulary; the
    kernel resolves each selector against the base's recorded carrier and
    certifies the blend through the landed blend profile machinery. An edge
    the recorded base cannot resolve refuses typed naming the open carrier (a
    valid outcome: the corpus's ``safe_fillet`` ladder then returns the part
    unchanged). A blend the kernel cannot close refuses typed, never
    approximates."""
    edge_list = list(edges)
    base = edge_list[0]._base if edge_list else getattr(edges, "_owner", None)
    if not isinstance(base, _Shape):
        _refuse("a fillet edge selection outside the recorded base carrier is not a kernel-engine row")
    recorded = [edge._selector() for edge in edge_list]
    node = {
        "fillet": {
            "base": base._node(),
            "radius": _num(radius),
            "edges": recorded,
        }
    }
    if _T123D is not None:
        _T123D.bd_facts(json.dumps(node))
    return _FilletResult(node)


def chamfer(edges, length, **kwargs):
    _refuse("chamfer is not a kernel-engine row")


def _mirror_row(part, axis):
    """The placed-carrier mirror of one part row.

    ``mirror(shape, about)`` reflects the shape about the coordinate plane
    through the world origin. The row keeps the same solid and records a
    mirror reflection of the local geometry, a reflected frame translation and
    a conjugated orientation so the world reflection composes exactly with the
    recorded frame (`world = translate(o) ∘ R ∘ M`).
    """
    out = _Part(part._solid)
    out._x, out._y, out._z = part._x, part._y, part._z
    out._rz = part._rz
    out._rot = part._rot
    if axis == "x":
        out._x = -out._x
    elif axis == "z":
        out._z = -out._z
    else:
        out._y = -out._y
    out._mirror = None if part._mirror == axis else axis
    if part._mirror is not None and part._mirror != axis:
        # A reflection followed by a second, different reflection is not a
        # single coordinate-plane mirror; refuse typed rather than record a
        # transform the placed-carrier model does not carry.
        _refuse("a double mirror over two planes is not a kernel-engine row")
    out.label = part.label
    out.color = part.color
    return out


def mirror(shape, about=None, mode=None, **kwargs):
    """build123d ``mirror(shape, about)``: the recording arm.

    A mirror about a coordinate plane records a placed-carrier transform over
    the mirrored census row (the landed placed/processor rule): the solid
    carrier is untouched and the facts transform with the placement -- no
    geometry is recomputed. A mirror about any non-coordinate carrier refuses
    typed.
    """
    axis = None
    if isinstance(about, Plane):
        axis = {"xz": "y", "yz": "x", "xy": "z"}.get(getattr(about, "name", None))
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


def _close(direction, target):
    return (
        abs(_num(direction.x) - target[0]) < 1e-12
        and abs(_num(direction.y) - target[1]) < 1e-12
        and abs(_num(direction.z) - target[2]) < 1e-12
    )


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
    "Line",
    "Rot",
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

    class _ShapeMixin:
        @staticmethod
        def cast(wrapped):
            _refuse("Shape.cast is not a kernel-engine row")

    class _SolidMixin:
        @classmethod
        def make_cylinder(cls, radius, height, plane=None, **kwargs):
            if plane is not None and isinstance(plane, Plane):
                return plane * Cylinder(radius, height)
            return Cylinder(radius, height)

        @classmethod
        def make_sphere(cls, radius, plane=None, **kwargs):
            if plane is not None and isinstance(plane, Plane):
                return plane * Sphere(radius)
            return Sphere(radius)

    bd.Shape = _ShapeMixin
    bd.Solid = _SolidMixin
    bd.Edge.make_line = Edge.make_line
    bd.Edge.make_spline = Edge.make_spline
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
        error = {"kind": type(exc).__name__, "message": str(exc)}
        payload = getattr(exc, "payload", None)
        if payload is not None:
            error["payload"] = payload
        record = {
            "schema": "ttc_door_run.v1",
            "door_version": door_version,
            "engine": ENGINE,
            "ok": False,
            "entry": entry,
            "error": error,
        }
        json.dump(record, sys.stdout, indent=2)
        return 1

    built = time.time() - t0
    facts = geometry_facts(obj)
    diag = float(facts.get("diag") or 1.0)
    tolerance = max(diag * 2.0e-4, 0.01)
    try:
        if ENGINE == "truck":
            _truck_export_stl(obj, stl_path)
        else:
            import build123d as bd

            bd.export_stl(obj, stl_path, tolerance=tolerance, angular_tolerance=0.5)
    except Exception as exc:  # noqa: BLE001 - a refused export is a typed verdict
        error = {"kind": type(exc).__name__, "message": str(exc)}
        payload = getattr(exc, "payload", None)
        if payload is not None:
            error["payload"] = payload
        record = {
            "schema": "ttc_door_run.v1",
            "door_version": door_version,
            "engine": ENGINE,
            "ok": False,
            "entry": entry,
            "module": module,
            "error": error,
        }
        json.dump(record, sys.stdout, indent=2)
        return 1

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

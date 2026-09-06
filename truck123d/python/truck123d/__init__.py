"""truck123d facade sugar (PB-005-PYTHON-FACADE).

A thin, build123d-shaped Python layer over the pyo3 bridge core. The design
doctrine (spec 5, PB-000 1, PB-005): the Python side EDITS TABLES, then
submits. Builder-mode statefulness is Python-side only -- a context manager
collects operations into a session table and submits exactly once; an
exception mid-build submits nothing (no partial kernel state crosses the
boundary). NOTHING computes in Python: every call in this module records one
row into the session table, and the native entry ``truck123d.facade_submit``
runs the single Rust facade fn over the submitted table.

The vocabulary (corpus surface, spec 8): ``BuildPart`` / ``BuildSketch``
context managers, ``Mode`` algebra, the primitives ``Box``/``Cylinder``/
``Sphere``/``Torus`` and the sketch carriers ``Polygon``/``Polyline``/
``Spline``, ``make_face``, the verbs ``extrude``/``revolve``/``sweep``/
``loft``/``fillet``/``chamfer``/``mirror``, the fluent four-expression
selector vocabulary (``faces()``/``edges()``/``filter_by()``/``take()``), and
``export_stl``/``export_step``.

Exports are session rows validated at submit: an ``export_step`` on a
swept/constructive part raises ``truck123d.Refused`` (the TR-NRB-001 STEP
boundary) when the session exits, never a silent fallback.

This module is importable either as the installed ``truck123d`` package or,
under the embedded-interpreter integration tests, under an alias with
``truck123d`` resolving to the native bridge module.
"""

import json

import truck123d

__all__ = [
    "BuildPart",
    "BuildSketch",
    "Mode",
    "Box",
    "Cylinder",
    "Sphere",
    "Torus",
    "Polygon",
    "Polyline",
    "Spline",
    "make_face",
    "extrude",
    "revolve",
    "sweep",
    "loft",
    "fillet",
    "chamfer",
    "mirror",
    "export_stl",
    "export_step",
]

# Row tag names are the snake_case op vocabulary of the Rust facade table;
# key order within each row mirrors the serde struct field order so the
# Python-rendered JSON is byte-identical to the Rust-rendered JSON.


class Mode:
    """The build123d ``Mode`` algebra (marshaled as the section 3.2 strings)."""

    ADD = "union"
    SUBTRACT = "subtract"
    INTERSECT = "intersect"


class _FacadeError(RuntimeError):
    """Raised when a facade call cannot be expressed as a table row."""


def _num(value):
    """Coerce an integral literal to float so JSON rendering matches serde."""
    if isinstance(value, bool):
        return float(value)
    if isinstance(value, int):
        return float(value)
    return value


def _point2(point):
    return [_num(point[0]), _num(point[1])]


def _point3(point):
    return [_num(point[0]), _num(point[1]), _num(point[2])]


_stack = []


def _root():
    if not _stack:
        raise _FacadeError(
            "no open builder session; wrap construction in a "
            "'with BuildPart()' / 'with BuildSketch()' block"
        )
    return _stack[0]


def _record(row):
    _root()._ops.append(row)


def _axis(value):
    if value not in ("x", "y", "z"):
        raise _FacadeError(f"axis must be 'x', 'y' or 'z', got {value!r}")
    return value


def _maybe_mode(mode):
    if mode is not None and mode != Mode.ADD:
        _record({"op": "mode", "value": mode})


class _Selection:
    """A thin fluent selection handle.

    Each method records one selector row into the owning session's table; the
    kernel's PB-001 machinery resolves the actual entities at execution time.
    """

    def __init__(self, session):
        self._session = session

    def filter_by(self, axis):
        """Keep the entities whose census sits on ``axis`` (x, y or z)."""
        _record({"op": "select_filter", "axis": _axis(axis)})
        return self

    def take(self, count):
        """Keep the first ``count`` entities of the selection."""
        _record({"op": "select_take", "count": int(count)})
        return self

    def last(self):
        """Convenience: ``take(1)``."""
        return self.take(1)


class _Session:
    """A builder session: records rows, closes, and submits once on success.

    Statefulness is Python-side only: the session holds the ordered op table
    and hands it to the native facade exactly once at exit. No kernel state
    exists on this side.
    """

    _frame_op = None

    def __init__(self):
        self._ops = []
        self._closed = False
        self._submitted = False
        self._report = None

    def __enter__(self):
        _stack.append(self)
        _record({"op": self._frame_op})
        return self

    def __exit__(self, exc_type, _exc, _tb):
        if _stack and _stack[-1] is self:
            _stack.pop()
        if exc_type is None:
            if _stack:
                # A nested session: rows belong to the root session, which
                # owns the single submit. Record the frame close only.
                _record({"op": "pop"})
            else:
                # Root session exit: close the frame and submit once.
                self._ops.append({"op": "pop"})
                self._submit()
        return False

    def _check_open(self):
        if self._closed:
            raise _FacadeError("session is closed; construction is not re-openable")

    def _submit(self):
        self._closed = True
        report_json = truck123d.facade_submit(self.to_table_json())
        self._report = json.loads(report_json)
        self._submitted = True

    def to_table_json(self):
        """The session table as compact JSON, byte-identical to the Rust table."""
        return json.dumps(
            {"ops": self._ops},
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        )

    @property
    def submitted(self):
        """Whether the session has been submitted to the kernel facade."""
        return self._submitted

    def faces(self):
        """Record a faces census of this part's solid (selector vocabulary 1)."""
        self._check_open()
        _record({"op": "select_faces"})
        return _Selection(self)

    def edges(self):
        """Record an edges census of this part's solid (selector vocabulary 2)."""
        self._check_open()
        _record({"op": "select_edges"})
        return _Selection(self)


class BuildPart(_Session):
    """build123d-shaped ``BuildPart``: collects solid-building ops, submits once."""

    _frame_op = "push_part"


class BuildSketch(_Session):
    """build123d-shaped ``BuildSketch``: collects sketch-carrier rows."""

    _frame_op = "push_sketch"


def Box(length, width, height, mode=None):
    """Record a ``Box`` solid primitive."""
    _maybe_mode(mode)
    _record(
        {
            "op": "box",
            "length": _num(length),
            "width": _num(width),
            "height": _num(height),
        }
    )


def Cylinder(radius, height, mode=None):
    """Record a ``Cylinder`` solid primitive."""
    _maybe_mode(mode)
    _record({"op": "cylinder", "radius": _num(radius), "height": _num(height)})


def Sphere(radius, mode=None):
    """Record a ``Sphere`` solid primitive."""
    _maybe_mode(mode)
    _record({"op": "sphere", "radius": _num(radius)})


def Torus(major_radius, minor_radius, mode=None):
    """Record a ``Torus`` solid primitive."""
    _maybe_mode(mode)
    _record(
        {
            "op": "torus",
            "major_radius": _num(major_radius),
            "minor_radius": _num(minor_radius),
        }
    )


def Polygon(*points):
    """Record a closed ``Polygon`` sketch profile from 2-D points."""
    _record({"op": "polygon", "points": [_point2(point) for point in points]})


def Polyline(*points):
    """Record an open ``Polyline`` sketch carrier from 2-D points."""
    _record({"op": "polyline", "points": [_point2(point) for point in points]})


def Spline(*points, periodic=False):
    """Record a ``Spline`` sketch carrier (PB-002 authoring).

    A spline carrier is a non-canonical carrier: a part built over one is
    constructive for the STEP export boundary.
    """
    _record(
        {"op": "spline", "points": [_point3(point) for point in points],
         "periodic": bool(periodic)}
    )


def make_face():
    """Record a ``make_face`` of the current closed planar profile."""
    _record({"op": "make_face"})


def extrude(amount, mode=None):
    """Record an ``extrude`` of the current sketch."""
    _maybe_mode(mode)
    _record({"op": "extrude", "amount": _num(amount)})


def revolve(angle=360.0, mode=None):
    """Record a ``revolve`` of the current sketch about z (angle in degrees)."""
    _maybe_mode(mode)
    _record({"op": "revolve", "angle_deg": _num(angle)})


def sweep(mode=None):
    """Record a spine ``sweep`` over the current section (constructive)."""
    _maybe_mode(mode)
    _record({"op": "sweep"})


def loft(mode=None):
    """Record a ``loft`` over the current sections (constructive)."""
    _maybe_mode(mode)
    _record({"op": "loft"})


def fillet(edges, radius):
    """Record a ``fillet`` of the edges named by ``edges`` (a selection)."""
    _record({"op": "fillet", "radius": _num(radius)})


def chamfer(edges, length):
    """Record a ``chamfer`` of the edges named by ``edges`` (a selection)."""
    _record({"op": "chamfer", "length": _num(length)})


def mirror(axis="x", mode=None):
    """Record a ``mirror`` of the current solid about the axis-aligned plane."""
    _maybe_mode(mode)
    _record({"op": "mirror", "axis": _axis(axis)})


def export_stl(part, path):
    """Record an STL export request for ``part`` (any part is in envelope).

    The row is validated at session submit; the report's export entries carry
    the accepted export.
    """
    part._check_open()
    _record({"op": "export_stl", "path": str(path)})
    return path


def export_step(part, path):
    """Record a STEP export request for ``part``.

    STEP out stays behind the TR-NRB-001 boundary: for a swept/constructive
    part the session submit raises ``truck123d.Refused`` (typed), never a
    silent STL fallback. Prismatic parts are in envelope.
    """
    part._check_open()
    _record({"op": "export_step", "path": str(path)})
    return path

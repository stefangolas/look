"""Read the raw STEP entity graph, without going through the importer.

The imported Rust enum is not enough to classify a refusal. `truck-stepio`
imports a STEP `CIRCLE` and a STEP `ELLIPSE` to the *same* variant —
`Conic3D::Ellipse(Processor<TrimmedCurve<UnitCircle>, Matrix4>)` — so the
entity-type distinction the source made is gone before any classifier sees it.
Any census that reports only the imported variant would report that loss as a
property of the geometry. This module reads the entity chain the file actually
declares:

    ADVANCED_FACE -> FACE_(OUTER_)BOUND -> EDGE_LOOP -> ORIENTED_EDGE
                  -> EDGE_CURVE -> edge_geometry -> (wrapper chain) -> basis

It is a reader. It parses text and follows references; it decides nothing.

Part 21 details that cost time here and are handled below: `ORIENTED_EDGE`'s
`edge_start`/`edge_end` are the redeclaration placeholder `*` and carry no
reference, so the edge element is argument 3 and not argument 1; a `FACE_BOUND`
names a *loop*, not an edge list, so the `EDGE_LOOP` hop is mandatory;
apostrophes inside string arguments must not be counted as parentheses; and a
complex instance `#n = ( A(..) B(..) )` has no single leading type name.
"""

from __future__ import annotations

import re

# The 3D curve entities a band bound can carry, split by whether they wrap
# another curve. A wrapper is not a curve family — it is a level of indirection
# whose basis has to be followed before the family can be named at all.
WRAPPERS = {
    "TRIMMED_CURVE": 0,
    "CURVE_REPLICA": 0,
    "OFFSET_CURVE_3D": 0,
    "SURFACE_CURVE": 0,
    "SEAM_CURVE": 0,
    "INTERSECTION_CURVE": 0,
    "BOUNDED_SURFACE_CURVE": 0,
    "COMPOSITE_CURVE": 0,
}

ENTITY_START = re.compile(r"#(\d+)\s*=\s*")
TYPE_HEAD = re.compile(r"([A-Za-z_0-9]+)\s*\(")


class EntityIndex:
    """Byte-offset index of every entity instance in one exchange file."""

    def __init__(self, path: str):
        self.blob = open(path, "rb").read().decode("latin-1")
        self.starts: dict[int, int] = {}
        for match in ENTITY_START.finditer(self.blob):
            self.starts[int(match.group(1))] = match.end()

    def originating_system(self) -> str:
        """`FILE_NAME`'s `originating_system`, or `-`.

        Positional, not the last-but-one string: `author` and `organization`
        are *lists* whose length varies per file, so counting quoted strings
        from either end lands on a different field depending on the exporter.
        ISO 10303-21 fixes the argument order, so index 5 is the only stable
        way to read it — `(name, time_stamp, author, organization,
        preprocessor_version, originating_system, authorisation)`.

        An exporter association is a correlation, and the report says so.
        """
        head = self.blob[: self.blob.find("DATA;")] if "DATA;" in self.blob else self.blob[:8192]
        match = re.search(r"FILE_NAME\s*\(", head, re.I)
        if not match:
            return "-"
        args = toplevel(self._balanced(match.end()))
        if len(args) < 6:
            return "-"
        return args[5].strip().strip("'").strip() or "-"

    def exporter_association(self) -> dict:
        """What the header still says about where the file came from.

        The ABC pipeline blanks `originating_system` to `' '` on every file, so
        that field alone cannot associate a defect with an exporter. The
        `FILE_NAME` path and the `FILE_SCHEMA` survive, and on this corpus both
        are uniform — which is itself the finding: the corpus cannot separate
        exporters, so no correlation drawn from it could be evidence about one.
        """
        head = self.blob[: self.blob.find("DATA;")] if "DATA;" in self.blob else self.blob[:8192]
        name = re.search(r"FILE_NAME\s*\(", head, re.I)
        schema = re.search(r"FILE_SCHEMA\s*\(", head, re.I)
        path = "-"
        if name:
            args = toplevel(self._balanced(name.end()))
            if args:
                path = args[0].strip().strip("'")
        return {
            "originating_system": self.originating_system(),
            "file_name_path": re.sub(r"\d{6,}", "N", path),
            "schema": (
                re.findall(r"'([^']*)'", self._balanced(schema.end()))[0]
                if schema
                else "-"
            ),
        }

    def _balanced(self, start: int) -> str:
        """Text up to the parenthesis that closes the one just opened."""
        depth, index = 1, start
        blob = self.blob
        while depth:
            char = blob[index]
            if char == "'":
                index = blob.index("'", index + 1)
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
            index += 1
        return blob[start : index - 1]

    def entity(self, eid: int):
        """`(TYPE, raw-parameter-text)`, or `(None, None)` if absent."""
        start = self.starts.get(eid)
        if start is None:
            return None, None
        match = TYPE_HEAD.match(self.blob, start)
        if not match:
            return "_COMPLEX_", self._balanced(self.blob.index("(", start) + 1)
        return match.group(1).upper(), self._balanced(match.end())


def refs(text: str) -> list[int]:
    return [int(x) for x in re.findall(r"#(\d+)", text)]


def toplevel(text: str) -> list[str]:
    """Split a parameter list on commas that are not nested or quoted."""
    out, depth, current, index = [], 0, [], 0
    while index < len(text):
        char = text[index]
        if char == "'":
            end = text.index("'", index + 1)
            current.append(text[index : end + 1])
            index = end + 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        if char == "," and depth == 0:
            out.append("".join(current).strip())
            current = []
        else:
            current.append(char)
        index += 1
    out.append("".join(current).strip())
    return out


def unwrap_curve(index: EntityIndex, eid: int, seen=None):
    """Follow a curve reference to its basis, recording the wrapper chain.

    Returns `(basis_entity_id, basis_type, wrapper_chain)`. The chain is the
    ordered list of wrapper types traversed, so `TRIMMED_CURVE` around a
    `CIRCLE` reports as `('CIRCLE', ['TRIMMED_CURVE'])` and a bare circle as
    `('CIRCLE', [])` — the difference the recovery question turns on.
    """
    seen = seen or set()
    chain: list[str] = []
    while True:
        if eid in seen:
            return eid, "_CYCLE_", chain
        seen.add(eid)
        etype, params = index.entity(eid)
        if etype is None:
            return eid, "_MISSING_", chain
        if etype not in WRAPPERS:
            return eid, etype, chain
        chain.append(etype)
        inner = refs(toplevel(params)[WRAPPERS[etype]])
        if not inner:
            return eid, etype, chain
        eid = inner[0]


def face_edge_chains(index: EntityIndex, face_id: int):
    """Every edge use of one `ADVANCED_FACE`, with its raw entity chain.

    Yields one dict per edge use, in declared bound and edge order — the same
    order the importer walks, so occurrence counts line up with the probe's.
    """
    # `ADVANCED_FACE` is a subtype of `FACE_SURFACE` and adds no argument, so
    # both are read here. Accepting only the former dropped 895 corpus faces
    # into an "empty chain" bucket that looked like a parse failure and was
    # really a taxonomy assumption.
    ftype, fparams = index.entity(face_id)
    if ftype not in ("ADVANCED_FACE", "FACE_SURFACE"):
        return
    fargs = toplevel(fparams)
    for bound_index, bound_id in enumerate(refs(fargs[1])):
        btype, bparams = index.entity(bound_id)
        loop_refs = refs(toplevel(bparams)[1]) if bparams else []
        if not loop_refs:
            continue
        ltype, lparams = index.entity(loop_refs[0])
        if ltype != "EDGE_LOOP":
            continue
        for use_index, oriented_id in enumerate(refs(toplevel(lparams)[1])):
            otype, oparams = index.entity(oriented_id)
            if otype != "ORIENTED_EDGE":
                continue
            oargs = toplevel(oparams)
            edge_refs = refs(oargs[3])
            if not edge_refs:
                continue
            etype, eparams = index.entity(edge_refs[0])
            if etype != "EDGE_CURVE":
                continue
            eargs = toplevel(eparams)
            geom = refs(eargs[3])
            if not geom:
                continue
            raw_type, raw_params = index.entity(geom[0])
            basis_id, basis_type, chain = unwrap_curve(index, geom[0])
            # A p-curve is authoritative only if the source supplies one on
            # this face's own support. Record whether any is present at all;
            # deciding support identity is a later packet's obligation.
            pcurves = []
            if raw_type in ("SURFACE_CURVE", "SEAM_CURVE", "INTERSECTION_CURVE"):
                for item in toplevel(raw_params)[2:]:
                    for ref in refs(item):
                        itype, _ = index.entity(ref)
                        if itype == "PCURVE":
                            pcurves.append(ref)
            yield {
                "bound_index": bound_index,
                "bound_type": btype,
                "use_index": use_index,
                "oriented_edge_id": oriented_id,
                "oriented_edge_orientation": oargs[4].strip(),
                "edge_curve_id": edge_refs[0],
                "edge_curve_same_sense": eargs[4].strip(),
                "start_vertex_id": refs(eargs[1])[0] if refs(eargs[1]) else None,
                "end_vertex_id": refs(eargs[2])[0] if refs(eargs[2]) else None,
                "edge_geometry_id": geom[0],
                "raw_edge_geometry_type": raw_type,
                "wrapper_chain": chain,
                "basis_curve_id": basis_id,
                "basis_curve_type": basis_type,
                "pcurve_ids": pcurves,
            }

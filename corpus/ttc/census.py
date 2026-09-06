"""Re-derives the build123d-compat surface census of the vendored corpus trees.

The compat-surface doc (`docs/PY_BRIDGE_COMPAT_SURFACE.md`) quotes the corpus's
measured API surface (spec 8's table). Every number in that doc must be
re-derivable by command, so this module re-derives the counts from the vendored
trees (`corpus/ttc/trees/*/src`) with an AST census: comments, docstrings and
string literals are excluded, and only the surface families the compat table
tracks are counted. The doc records the census command and date.

Run:

    python corpus/ttc/census.py

Counters (each is the number of AST nodes/calls of that family across every
vendored corpus source file):

  algebra_ops      binary `+` `-` `&` operators (any operand; the corpus is
                   algebra-mode so shape algebra dominates; the count is the
                   upper bound of boolean algebra, see the doc note)
  plane_location   `Plane(`, `Location(`, `Pos(`/`Rotation(` name loads, and
                   `.offset(` attribute calls (all `Plane.offset` frame moves)
  primitives       `Box`/`Cylinder`/`Sphere`/`Torus`/`Compound` name loads
  topology         `make_face` calls and `Face`/`Edge`/`Wire`/`Solid`/`Shape`
                   name loads
  spline_sections  `make_spline`/`Spline` name loads (section authoring)
  sweep_verbs      `loft`/`revolve`/`extrude`/`sweep`/`fillet`/`chamfer`/
                   `mirror` name loads and calls
  selectors        `.faces(`/`.edges(`/`.filter_by(`/`.take(` attribute calls
"""

from __future__ import annotations

import ast
import os
import sys

TREES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "trees")


class Census(ast.NodeVisitor):
    def __init__(self) -> None:
        self.algebra_ops = 0
        self.plane_location = 0
        self.primitives = 0
        self.topology = 0
        self.spline_sections = 0
        self.sweep_verbs = 0
        self.selectors = 0

    def visit_BinOp(self, node: ast.BinOp) -> None:  # noqa: N802
        if isinstance(node.op, (ast.Add, ast.Sub, ast.BitAnd)):
            self.algebra_ops += 1
        self.generic_visit(node)

    def visit_Name(self, node: ast.Name) -> None:  # noqa: N802
        name = node.id
        if name in ("Plane", "Location", "Pos", "Rotation", "Axis"):
            self.plane_location += 1
        elif name in ("Box", "Cylinder", "Sphere", "Torus", "Compound"):
            self.primitives += 1
        elif name in ("Face", "Edge", "Wire", "Solid", "Shape"):
            self.topology += 1
        elif name in ("Spline", "make_spline"):
            self.spline_sections += 1
        elif name in (
            "loft", "revolve", "extrude", "sweep", "fillet", "chamfer", "mirror",
            "make_loft", "make_revolve", "make_extrude",
        ):
            self.sweep_verbs += 1

    def visit_Attribute(self, node: ast.Attribute) -> None:  # noqa: N802
        attr = node.attr
        if attr in ("Plane", "Location", "Pos", "Rotation", "Axis"):
            self.plane_location += 1
        elif attr in ("Box", "Cylinder", "Sphere", "Torus", "Compound"):
            self.primitives += 1
        elif attr in ("Face", "Edge", "Wire", "Solid", "Shape"):
            self.topology += 1
        elif attr in ("Spline", "make_spline"):
            self.spline_sections += 1
        elif attr in (
            "loft", "revolve", "extrude", "sweep", "fillet", "chamfer",
            "mirror", "make_loft", "make_revolve", "make_extrude",
        ):
            self.sweep_verbs += 1
        elif attr == "offset":
            self.plane_location += 1
        elif attr in ("faces", "edges", "filter_by", "take", "last"):
            self.selectors += 1
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call) -> None:  # noqa: N802
        fn = node.func
        if isinstance(fn, ast.Name) and fn.id == "make_face":
            self.topology += 1
        self.generic_visit(node)


def main() -> int:
    root = sys.argv[1] if len(sys.argv) > 1 else TREES
    if not os.path.isdir(root):
        print(f"no such tree dir: {root}", file=sys.stderr)
        return 1
    files = []
    for dirpath, _dirnames, filenames in os.walk(root):
        if "src" not in dirpath:
            continue
        for name in sorted(filenames):
            if name.endswith(".py"):
                files.append(os.path.join(dirpath, name))
    census = Census()
    for path in files:
        with open(path, encoding="utf-8") as fh:
            try:
                tree = ast.parse(fh.read(), filename=path)
            except SyntaxError:
                print(f"parse failed: {path}", file=sys.stderr)
                continue
        census.visit(tree)
    print(f"files={len(files)}")
    print(f"algebra_ops={census.algebra_ops}")
    print(f"plane_location={census.plane_location}")
    print(f"primitives={census.primitives}")
    print(f"topology={census.topology}")
    print(f"spline_sections={census.spline_sections}")
    print(f"sweep_verbs={census.sweep_verbs}")
    print(f"selectors={census.selectors}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

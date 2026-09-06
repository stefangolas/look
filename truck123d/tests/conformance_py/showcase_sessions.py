"""PB-007 conformance battery -- the three showcase facade sessions.

Each showcase model's canonical facade subset (the same ordered op table the
Rust harness builds on its side) is executed through the real facade sugar as a
standalone python door:

* build the session exactly as a build123d-style ``with facade.BuildPart()``
  block records it (pb_005 doctrine: python edits tables, nothing computes),
* render the submitted table JSON (``part.to_table_json()``),
* submit once through the native entry,
* write the table JSON on line 1 and the report JSON on line 2.

The Rust harness byte-compares line 1 against its own serialization of the
same FacadeTable and line 2 against the report its ``run_facade`` produces for
it -- the byte-equality gate. Numbers are the showcase table's real values
(foot prism radius/height, teapot body belly/rim, waterslide pool
radius/depth+rim) as decimal literals whose JSON rendering is byte-identical
to Rust's serde of the same f64s. This mirrors pb_assembly's teapot
decomposition (shape fidelity not under test; the op table is what runs).
"""

import sys

from conformance_runtime import facade, native


def amphora():
    with facade.BuildPart() as part:
        facade.Cylinder(radius=0.26, height=0.06)
        facade.export_stl(part, "amphora_foot.stl")
    return part


def teapot():
    with facade.BuildPart() as part:
        facade.Cylinder(radius=2.5, height=4.0)
        facade.export_stl(part, "teapot_body.stl")
    return part


def waterslide():
    with facade.BuildPart() as part:
        facade.Cylinder(radius=3.0, height=1.45)
        facade.export_stl(part, "waterslide_pool.stl")
    return part


SESSIONS = {
    "amphora": amphora,
    "teapot": teapot,
    "waterslide": waterslide,
}


def run(model):
    part = SESSIONS[model]()
    table_json = part.to_table_json()
    report_json = native.facade_submit(table_json)
    sys.stdout.write(table_json + "\n")
    sys.stdout.write(report_json + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(run(sys.argv[1]))

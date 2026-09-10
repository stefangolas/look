"""Row-closure static sweep: op inventory per F1 row script.

The first artifact of loop/row_closure.py. Counts carrier-class call sites
per corpus lib file so the per-row carrier demand is MEASURED, not guessed.
Regex-classification of a closed op vocabulary (the drop-in + surfaces.py
helpers). Dynamic dispatch is out of scope for v1; the trace-carriers door
mode supersedes it later.
"""

import re
from pathlib import Path

OPS = {
    "loft": r"loft\w*\(|body_loft\(|ruled_loft\(",
    "cut/fuse": r"\.cut\(|\.fuse\(|surfaces\.cut\(|surfaces\.fuse\(",
    "mirror": r"mirror_y\(|\.mirror\(",
    "sweep/blade": r"swept_plate\(|blade_path\(|blade_member\(",
    "bbox/obox": r"surfaces\.(bbox|obox)\(",
    "validity": r"is_valid_shape\(|is_valid\(",
    "sections": r"section_face\(|half_section_face\(|make_face\(|superellipse_pts\(",
    "spline-author": r"make_spline\(|Spline\(|spline\(",
    "extrude": r"extrude\(",
    "revolve": r"revolve\(",
    "trim": r"trim\(",
    "fillet/chamfer": r"\.fillet\(|\.chamfer\(",
}


def main():
    lib = Path("corpus/ttc/trees/f1/src/lib")
    files = sorted(
        p for p in lib.glob("*.py") if p.name not in ("surfaces.py", "spec.py", "__init__.py")
    )
    header = ["file"] + list(OPS)
    print(("{:<18}" + "{:>11}" * len(OPS)).format(*header))
    totals = dict.fromkeys(OPS, 0)
    for f in files:
        text = f.read_text(encoding="utf-8", errors="replace")
        counts = {k: len(re.findall(rx, text)) for k, rx in OPS.items()}
        for k, c in counts.items():
            totals[k] += c
        name = f.stem[:17]
        print(("{:<18}" + "{:>11}" * len(OPS)).format(name, *[counts[k] for k in OPS]))
    print(("{:<18}" + "{:>11}" * len(OPS)).format("TOTAL", *[totals[k] for k in OPS]))


if __name__ == "__main__":
    main()

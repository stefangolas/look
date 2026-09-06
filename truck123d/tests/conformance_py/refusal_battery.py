"""PB-007 conformance battery -- the typed-refusal mirror door.

Takes one submitted facade table JSON on the command line and submits it
through the native entry. If the facade refuses typed, the mapped python
exception is caught and its class plus the marshaled named cause are printed
(the same named cause the Rust side marshals from the refusal of the same
table):

    line 1: the exception class name (Refused / Unresolved)
    line 2: a JSON record of the flattened payload fields (case, envelope, ...)

If the table does not refuse, the run fails loudly (a mirror battery row that
does not refuse is a harness failure, not a skip).
"""

import json
import sys

from conformance_runtime import native


def record_from(exception):
    """The flattened named-cause fields the bridge attached to the exception."""
    record = {"class": type(exception).__name__}
    for field in (
        "case",
        "envelope",
        "stage",
        "prop",
        "left",
        "right",
        "reason",
        "bound",
        "allowed",
        "witness",
    ):
        if hasattr(exception, field):
            value = getattr(exception, field)
            if isinstance(value, (int, float)) or value is not None:
                record[field] = value
    return record


def run(table_json):
    try:
        native.facade_submit(table_json)
    except Exception as exc:  # noqa: BLE001 - the mirror records the raise
        sys.stdout.write(type(exc).__name__ + "\n")
        sys.stdout.write(json.dumps(record_from(exc), separators=(",", ":")) + "\n")
        return 0
    sys.stderr.write("refusal door: the table did not refuse\n")
    return 1


if __name__ == "__main__":
    sys.exit(run(sys.argv[1]))

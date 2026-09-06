"""PB-007 conformance battery shared runtime.

The battery executes the three showcase facade sessions as REAL python
processes (a fresh interpreter per run). This module wires the native
``truck123d`` extension module (staged by the Rust harness into
``T123D_NATIVE_DIR``) and loads the facade sugar package from source
(``T123D_SUGAR_DIR``) under the alias ``facade`` -- exactly the convention the
embedded tests use (``tests/pb_facade.rs`` loads the sugar file as the module
``facade``), applied to a standalone process.

Every session script imports ``facade`` from this module and records rows with
the sugar; submitting once through the native entry returns the facade report
whose JSON the Rust harness byte-compares against the Rust run of the same
table.
"""

import importlib.util
import os
import sys


def native_module():
    """Imports the native ``truck123d`` module from the staged directory."""
    native_dir = os.environ.get("T123D_NATIVE_DIR")
    if not native_dir:
        raise RuntimeError("conformance battery requires T123D_NATIVE_DIR")
    sys.path.insert(0, native_dir)
    import truck123d

    return truck123d


def facade_module():
    """Loads the facade sugar package source as the module ``facade``.

    The sugar file does ``import truck123d`` at its top; in a standalone
    process that import resolves to the staged native module (its directory is
    first on ``sys.path``), mirroring how the embedded tests register the
    native module before loading the sugar under the ``facade`` alias.
    """
    sugar_dir = os.environ.get("T123D_SUGAR_DIR")
    if not sugar_dir:
        raise RuntimeError("conformance battery requires T123D_SUGAR_DIR")
    sugar_file = os.path.join(sugar_dir, "truck123d", "__init__.py")
    if not os.path.isfile(sugar_file):
        raise RuntimeError("facade sugar not found at %s" % sugar_file)
    spec = importlib.util.spec_from_file_location("facade", sugar_file)
    facade = importlib.util.module_from_spec(spec)
    sys.modules["facade"] = facade
    spec.loader.exec_module(facade)
    return facade


native = native_module()
facade = facade_module()

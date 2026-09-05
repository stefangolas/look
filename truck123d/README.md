# truck123d

The pyo3 core of the Truck123d Python Bridge — work packet PB-004-PYO3-CORE.

Scope: the bridge crate's module init, the frozen two-class `Refusal` →
exception mapping (`Refused` / `Unresolved`, payloads built from the witness
structs via serde — never pickled internals), serde round-trip of the v1
showcase tables, and the GIL policy. Zero geometric content (spec §5): the
only kernel call exercised here is the landed evidence algebra's
`Modulus::compose` (a `truck-base` entry). No kernel type crosses the boundary
except behind the opaque `TruckSolid` handle; no `#[pyclass]` sits on a kernel
type.

## The typed exception hierarchy

```text
TruckError (bridge base; never raised directly)
├── Refused      # a definitive kernel refusal
└── Unresolved   # could not certify within budget
```

Every `Refusal` variant maps to one of the two classes (PB-000 §2). A raised
exception carries `payload` — the serde rendering of the kernel witness — plus
flattened attributes (`case`, `envelope`, `stage`, `prop`/`left`/`right`,
`reason`/`certificate`, `bound`/`allowed`, or `kappa`/`witness`).

## GIL policy

Every kernel call runs with the GIL released through the single choke point
`truck123d.gil::with_kernel_gil_released` (`pyo3::Python::detach`). The
GIL-released closure never panics across the FFI: it converts errors
(`Refusal` → typed exception), never unwinds.

## Building

The crate is a normal Cargo workspace member. `cargo test -p truck123d` runs
the Rust-side suite (the four PB-004 tests); it embeds a CPython interpreter
on this machine for the exception/GIL tests, so a Python 3 with development
headers must be discoverable by pyo3 (this machine: Python 3.14).

For the Python extension module itself, build with maturin (CI step, out of
scope for PB-004):

```console
maturin build --manifest-path truck123d/Cargo.toml --features wheel-abi3
```

`wheel-abi3` enables pyo3's `abi3` feature, so the wheel is a PEP 384
limited-API wheel portable across CPython 3.x. maturin ≥ 1.9.4 sets
`PYO3_BUILD_EXTENSION_MODULE` itself (the old `extension-module` cargo feature
is obsolete on pyo3 0.29). The `auto-initialize` pyo3 feature is a
dev-dependency only: it lets the Rust test binary embed an interpreter without
leaking into the shipped module.

## Python smoke test

After the module is built and installed (e.g. `maturin develop`), run:

```python
import truck123d
assert issubclass(truck123d.Refused, truck123d.TruckError)
assert issubclass(truck123d.Unresolved, truck123d.TruckError)
```

or, from a shell:

```console
python -c "import truck123d"
```

If the environment lacks the interpreter/toolchain to build the wheel, the
Rust-side suite (`cargo test -p truck123d`) is the done-when; record that the
Python smoke could not be executed and do not install toolchains.

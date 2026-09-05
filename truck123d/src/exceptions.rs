//! The typed exception hierarchy (PB-000 §2):
//!
//! ```text
//! BaseError
//! ├── Refused
//! └── Unresolved
//! ```
//!
//! `TruckError` is the bridge base (never raised directly); every kernel error
//! is `Refused` or `Unresolved`, each carrying the typed serde payload the
//! kernel produced. Nothing degrades to a bare `Exception`.

use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(
    truck123d,
    TruckError,
    PyException,
    "Base class for truck123d bridge errors (PB-000 §2). Never raised directly."
);
create_exception!(
    truck123d,
    Refused,
    TruckError,
    "A definitive kernel refusal: the operation was asked something outside the supported \
     envelope, empty, contradictory, collapsed, or past a margin/backward/forward bound. \
     Carries the typed serde payload as `payload`."
);
create_exception!(
    truck123d,
    Unresolved,
    TruckError,
    "The kernel could not certify within budget: its three-valued verdict came back 'cannot \
     decide'. Carries the κ budget ledger and the witness as `payload`."
);

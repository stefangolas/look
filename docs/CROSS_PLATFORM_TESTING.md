# Cross-platform rendering tests

GPU tests are split by purpose because a container cannot emulate DirectX 12,
Metal, or a useful range of physical GPUs.

## Required lanes

1. `cross-platform.yml` runs format, compile, parser, camera, cache, and unit tests
   on ordinary Windows, Linux, and macOS hosted runners for every change.
2. `gpu-metal.yml` runs the ignored GPU smoke test on GitHub's Apple-silicon
   `macos-15-xlarge` runner weekly and on demand. It captures `v3 doctor --json`
   and the rendered fixture as build artifacts. This is correctness coverage,
   not a performance baseline.
3. A Linux Mesa/Lavapipe lane will provide deterministic software-Vulkan image
   comparisons once golden-image comparison is introduced.
4. Dedicated bare-metal GPU runners provide publishable performance and driver
   coverage.

## Hardware runner labels

Use GitHub runner labels that describe facts rather than a generic `gpu` label:

```text
self-hosted,windows,x64,nvidia-rtx,dx12,driver-current,physical
self-hosted,linux,x64,amd-rdna,vulkan,mesa-current,physical
self-hosted,linux,x64,nvidia-t4,vulkan,driver-current,partitioned
self-hosted,macOS,ARM64,apple-m4,metal,physical
```

Every run must retain:

- renderer commit and build profile
- operating-system version
- backend, adapter, vendor/device ID, and driver from `v3 doctor --json`
- source fixture hash
- resolved render configuration
- image and machine-readable manifest
- wall-clock and internal stage timings
- execution class: `physical`, `partitioned`, `software`, or `unknown`

Only `physical` bare-metal runs may publish performance claims or gate latency
regressions. Virtual, partitioned, hosted-unknown, and software GPU timing is
diagnostic only. Correctness tests may be compared across hardware using image
metrics and tolerances, never byte-for-byte PNG equality.

## Driver matrix

Keep `driver-current` as the required release lane. Add `driver-candidate` and
selected pinned versions as scheduled lanes on reimaged physical machines.
Changing a container image changes user-space libraries but not the host kernel
driver, DirectX stack, or Metal implementation, so it is not counted as driver
coverage.

Self-hosted GPU runners must accept only trusted workflows. Do not expose them
to arbitrary code from fork pull requests.


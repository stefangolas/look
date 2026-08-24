#!/bin/bash
# Anchor verification for BG-FID-005-SRF, run against the integration tip
# (main worktree). Counts measured 2026-08-23, post f96bd5c + 835f4cd.
cd "$(dirname "$0")/.." || exit 1
E=vendor/truck/truck-evidence/src
check() { # id expect cmd
  local got
  got=$(eval "$3")
  if [ "$got" = "$2" ]; then
    echo "OK   $1 expect=$2 got=$got  [$3]"
  else
    echo "FAIL $1 expect=$2 got=$got  [$3]"
  fi
}
check S1 1 "grep -c 'pub fn rep_curve' $E/fid/rep.rs"
check S2 1 "grep -c 'pub struct HermiteCurve' $E/fid/rep.rs"
check S3 1 "grep -c 'pub enum RepError' $E/fid/rep.rs"
check S4 1 "grep -c 'pub fn curvature_radius_lower' $E/fid/lfs.rs"
check S5 4 "grep -c '^pub mod' $E/fid/mod.rs"
check S6 1 "grep -c 'pub mod rep' $E/fid/mod.rs"
check S7 1 "grep -c 'pub fn krawczyk' $E/num/krawczyk.rs"
check S8 1 "grep -c 'pub enum KrawczykProof' $E/num/krawczyk.rs"
check S9 1 "grep -c 'fn uniform_cells' $E/fid/isotopy.rs"
check S10 1 "grep -c 'pub(crate) fn box_distance' $E/fid/isotopy.rs"
check S11 1 "grep -c 'pub(crate) fn sup_distance_box' $E/fid/isotopy.rs"
check S12 1 "grep -c 'pub(crate) fn angle_pass_form' $E/fid/isotopy.rs"
check S13 1 "grep -c 'pub(crate) fn dot_box' $E/fid/isotopy.rs"
check S14 1 "grep -c 'pub fn face_scale_components' $E/fid/lfs.rs"
check S15 1 "grep -c 'pub fn fibre_degree_one_auto' $E/fid/one_sheet.rs"
check S16 0 "grep -c 'pub fn rep_surface' $E/fid/rep.rs"
check S17 0 "grep -c 'pub struct HermiteSurface' $E/fid/rep.rs"
check S18 0 "grep -c 'SurfaceBoundary' $E/fid/rep.rs"
check S19 0 "grep -c 'pub enum RepSurfaceError' $E/fid/rep.rs"
echo "---- done"

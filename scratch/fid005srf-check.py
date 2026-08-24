"""Machine-check for BG-FID-005-SRF (rep_surface) witness numbers.

Evaluates the EXACT formulas the packet mandates, in outward-rounded interval
arithmetic: per-sub-cell box-to-box sup distances, the normal-box (ii) form,
lfs's curvature formula per cell, wrapped-gap self-separation, the bivariate
grid-vertex Krawczyk system, and the 2D non-adjacent separation check with
Chebyshev wrap adjacency. Never a true-error surrogate (the session-21 trap).

Run: python scratch/fid005srf-check.py
"""
import math
import sys

sys.path.insert(0, "scratch")
from fid005srf_lib import (  # noqa: E402
    Iv, B3, PI, TWO_PI, dot_box, cross_box, box_distance, sup_distance_box,
    norm_sup, norm_inf, angle_pass_form, mig, immersion_lower_bound_box,
    hull_pts, SphereBelt, GraphZ, DoubleCover, build_cell, SurfCell,
    HermiteSurface, iv_sin, iv_cos, dn, up,
)

SUB = 4            # per-axis sub-cell count of the measurement (the mandated SURF_MEASURE_SUB)
CERT_CONV_REL = 0.05  # relative span-helper convergence threshold (witnesses span scales)
LEVEL_CAP = 7      # surface span-helper level cap: uniform quad refinement is 4^level cells
W_FLOOR = 8.0 * 2.0 ** -52


def iv(lo, hi):
    return Iv(dn_lo(lo), up_hi(hi))


def dn_lo(x):
    return math.nextafter(x, -math.inf)


def up_hi(x):
    return math.nextafter(x, math.inf)


def width_floor(i):
    return 8.0 * 2.0 ** -52 * max(abs(i.lo), abs(i.hi), 1.0)


def can_sub(i):
    return i.wid() > width_floor(i)


def split(i):
    m = 0.5 * i.lo + 0.5 * i.hi
    return Iv(i.lo, m), Iv(m, i.hi)


# ---------------- measurement ----------------

def measure(surf, exact):
    """(eps_now, theta_now, cell_eps[row-major], ext_u, ext_v) over the grid."""
    nu, nv = surf.nu, surf.nv
    eps_now = 0.0
    theta_now = math.inf
    cell_eps = []
    ext_u = 0.0
    ext_v = 0.0
    for iu in range(nu):
        for iv_ in range(nv):
            cell = surf.cell(iu, iv_)
            uu = Iv(cell.a, cell.b)
            vv = Iv(cell.c, cell.d)
            cmax = 0.0
            for su in range(SUB):
                lo_u = cell.a + cell.hu * su / SUB
                hi_u = cell.a + cell.hu * (su + 1) / SUB
                for sv in range(SUB):
                    lo_v = cell.c + cell.hv * sv / SUB
                    hi_v = cell.c + cell.hv * (sv + 1) / SUB
                    sub_u = Iv(lo_u, hi_u)
                    sub_v = Iv(lo_v, hi_v)
                    eb = exact.enclose(sub_u, sub_v)
                    hb = cell.enclose(lo_u, hi_u, lo_v, hi_v)
                    sup = sup_distance_box(hb, eb)
                    cmax = max(cmax, sup)
                    eps_now = max(eps_now, sup)
                    # normal boxes
                    du_e = exact.enclose_der(1, 0, sub_u, sub_v)
                    dv_e = exact.enclose_der(0, 1, sub_u, sub_v)
                    du_h = cell.enclose_du(lo_u, hi_u, lo_v, hi_v)
                    dv_h = cell.enclose_dv(lo_u, hi_u, lo_v, hi_v)
                    n_e = cross_box(du_e, dv_e)
                    n_h = cross_box(du_h, dv_h)
                    ratio = angle_pass_form(n_h, n_e)
                    theta_now = min(theta_now, ratio)
                    ext_u = max(ext_u, (hi_u - lo_u) * norm_sup(du_e))
                    ext_v = max(ext_v, (hi_v - lo_v) * norm_sup(dv_e))
            cell_eps.append(cmax)
    return eps_now, theta_now, cell_eps, ext_u, ext_v


# ---------------- scale components ----------------

def curvature_of_cell(exact, uu, vv):
    """lfs::curvature_radius_lower's formula per cell. None = refusal."""
    su = exact.enclose_der(1, 0, uu, vv)
    sv = exact.enclose_der(0, 1, uu, vv)
    s2u = exact.enclose_der(2, 0, uu, vv)
    s12 = exact.enclose_der(1, 1, uu, vv)
    s2v = exact.enclose_der(0, 2, uu, vv)
    n_raw = cross_box(su, sv)
    iota = immersion_lower_bound_box(n_raw)
    if iota <= 0.0:
        return None
    e = dot_box(su, su)
    f = dot_box(su, sv)
    g = dot_box(sv, sv)

    def mag_up(i):
        return max(abs(i.lo), abs(i.hi))

    l_up = mag_up(dot_box(s2u, n_raw)) / iota
    m_up = mag_up(dot_box(s12, n_raw)) / iota
    n_up = mag_up(dot_box(s2v, n_raw)) / iota
    delta_mag = max(abs(e.hi - g.lo), abs(g.hi - e.lo))
    f_mag = max(abs(f.lo), abs(f.hi))
    disc_up = math.sqrt(delta_mag * delta_mag + 4.0 * f_mag * f_mag)
    lam_min_lo = 0.5 * (e.lo + g.lo - disc_up)
    if lam_min_lo <= 0.0:
        return None
    k_up = (l_up + m_up + n_up) / lam_min_lo
    if k_up == 0.0:
        return math.inf
    return 1.0 / k_up


def curvature_span(exact, budget=None):
    (u0, u1), (v0, v1) = exact.range()
    # incremental: entries are (uu, vv, value-or-None); children computed once.
    # Uniform quad-refinement is 4^level cells and the lfs bound's deficit is
    # LINEAR in cell width, so the helper stops at min(convergence, LEVEL_CAP):
    # the capped value is a certified (more conservative) lower bound.
    def val(uu, vv):
        return (uu, vv, curvature_of_cell(exact, uu, vv))

    cells = [val(Iv(u0, u1), Iv(v0, v1))]
    prev = math.inf
    spend = 0
    level = 0
    while True:
        best = math.inf
        had_err = False
        err_at_floor = False
        for (uu, vv, r) in cells:
            if r is None:
                had_err = True
                if not (can_sub(uu) or can_sub(vv)):
                    err_at_floor = True
                continue
            best = min(best, r)
        if err_at_floor:
            return None, spend, "REFUSE_ERR_AT_FLOOR"
        if best == math.inf and not had_err:
            return math.inf, spend, "FLAT"
        change = math.inf if (math.isinf(prev) or math.isinf(best)) else abs(best - prev)
        if (change < CERT_CONV_REL * best and best != 0.0) or level >= LEVEL_CAP:
            return best, spend, "CONV" if change < CERT_CONV_REL * best else "CAP"
        prev = best
        level += 1
        nxt = []
        refined = False
        for (uu, vv, _) in cells:
            if can_sub(uu) or can_sub(vv):
                spend += 1
                if budget is not None and spend > budget:
                    return None, spend, "BUDGET"
                (ua, ub) = split(uu)
                (va, vb) = split(vv)
                nxt.append(val(ua, va))
                nxt.append(val(ua, vb))
                nxt.append(val(ub, va))
                nxt.append(val(ub, vb))
                refined = True
            else:
                nxt.append((uu, vv, curvature_of_cell(exact, uu, vv)))
        cells = nxt
        if not refined:
            return best, spend, "FLOOR"


def axis_farthest_gap(a, b, closed, period):
    """Farthest point gap between 1D intervals on the axis."""
    d_min = max(0.0, a.lo - b.hi, b.lo - a.hi)
    d_max = max(abs(a.lo - b.hi), abs(a.hi - b.lo),
                abs(a.lo - b.lo), abs(a.hi - b.hi))
    if not closed:
        return d_max
    half = 0.5 * period
    if d_max <= half:
        return d_max
    if d_min >= half:
        return period - d_min
    return half


def pair_qualifies(c1, c2, closed_u, closed_v, pu, pv, gap):
    gu = axis_farthest_gap(c1[0], c2[0], closed_u, pu)
    gv = axis_farthest_gap(c1[1], c2[1], closed_v, pv)
    return max(gu, gv) >= gap


def axis_candidate_deltas(n, w, closed, period, gap):
    """Index distances m whose cell pairs COULD qualify on this axis."""
    out = set()
    if not closed:
        m_lo = gap / w - 1.0
        for m in range(1, n):
            if m + 1.0 >= m_lo and (m + 1.0) * w >= gap:
                out.add(m)
        return out
    half = 0.5 * period
    for m in range(1, n):
        d_min = (m - 1.0) * w
        d_max = (m + 1.0) * w
        if d_max <= half:
            farthest = d_max
        elif d_min >= half:
            farthest = period - d_min
        else:
            farthest = half
        if farthest >= gap:
            out.add(m)
    return out


def separation_span(exact, closed_u, closed_v, gap, budget=None):
    (u0, u1), (v0, v1) = exact.range()
    pu, pv = (u1 - u0), (v1 - v0)
    prev = math.inf
    spend = 0
    for level in range(LEVEL_CAP + 1):
        nu = 2 ** level
        nv = 2 ** level
        wu = pu / nu
        wv = pv / nv
        cells = []
        for iu in range(nu):
            for iv_ in range(nv):
                cells.append((Iv(u0 + iu * wu, u0 + (iu + 1) * wu),
                              Iv(v0 + iv_ * wv, v0 + (iv_ + 1) * wv)))
        boxes = [exact.enclose(uu, vv) for (uu, vv) in cells]
        cand_u = axis_candidate_deltas(nu, wu, closed_u, pu, gap)
        cand_v = axis_candidate_deltas(nv, wv, closed_v, pv, gap)
        best = math.inf
        for i in range(len(cells)):
            iu, iv_ = i // nv, i % nv
            partners = set()
            for m in cand_v:
                partners.add((iu, (iv_ + m) % nv))
                partners.add((iu, (iv_ - m) % nv))
            for m in cand_u:
                partners.add(((iu + m) % nu, iv_))
                partners.add(((iu - m) % nu, iv_))
            for (ku, kv) in partners:
                j = ku * nv + kv
                if j <= i:
                    continue
                if not pair_qualifies(cells[i], cells[j], closed_u, closed_v, pu, pv, gap):
                    continue
                d = box_distance(boxes[i], boxes[j])
                if d < best:
                    best = d
        change = math.inf if (math.isinf(prev) or math.isinf(best)) else abs(best - prev)
        if change < CERT_CONV_REL * best and best != 0.0:
            return best, spend, "CONV"
        if level >= LEVEL_CAP:
            return best, spend, "CAP"
        prev = best
        spend += nu * nv  # one split per cell of this level
        if budget is not None and spend > budget:
            return None, spend, "BUDGET"
    return prev, spend, "FLOOR"


# ---------------- bivariate Krawczyk ----------------

class VertexSystem:
    def __init__(self, exact, phi):
        self.exact = exact
        self.phi = phi

    def f_point(self, s, t):
        e = self.exact.enclose(Iv.c(s), Iv.c(t))
        eu = self.exact.enclose_der(1, 0, Iv.c(s), Iv.c(t))
        ev = self.exact.enclose_der(0, 1, Iv.c(s), Iv.c(t))
        dx = Iv(dn(e.x.lo - self.phi[0]), up(e.x.hi - self.phi[0]))
        dy = Iv(dn(e.y.lo - self.phi[1]), up(e.y.hi - self.phi[1]))
        dz = Iv(dn(e.z.lo - self.phi[2]), up(e.z.hi - self.phi[2]))
        dbox = B3(dx, dy, dz)
        return [dot_box(dbox, eu), dot_box(dbox, ev)]

    def jacobian(self, q):
        uu, vv = q
        e = self.exact.enclose(uu, vv)
        su = self.exact.enclose_der(1, 0, uu, vv)
        sv = self.exact.enclose_der(0, 1, uu, vv)
        suu = self.exact.enclose_der(2, 0, uu, vv)
        suv = self.exact.enclose_der(1, 1, uu, vv)
        svv = self.exact.enclose_der(0, 2, uu, vv)
        d = B3(Iv(dn(e.x.lo - self.phi[0]), up(e.x.hi - self.phi[0])),
               Iv(dn(e.y.lo - self.phi[1]), up(e.y.hi - self.phi[1])),
               Iv(dn(e.z.lo - self.phi[2]), up(e.z.hi - self.phi[2])))
        return [[dot_box(d, suu) - dot_box(su, su), dot_box(d, suv) - dot_box(su, sv)],
                [dot_box(d, suv) - dot_box(sv, su), dot_box(d, svv) - dot_box(sv, sv)]]

    def preconditioner(self, s, t):
        q = (Iv.c(s), Iv.c(t))
        j = self.jacobian(q)
        a, b = j[0][0].mid(), j[0][1].mid()
        c, d = j[1][0].mid(), j[1][1].mid()
        det = a * d - b * c
        if not math.isfinite(det) or det == 0.0:
            return None
        return [[d / det, -b / det], [-c / det, a / det]]


def krawczyk2(system, start, max_split=200):
    """K(Q) = m - Y F(m) + (I - Y J(Q))(Q - m); returns 'Unique'/'NoRoot'/'Err'."""
    stack = [start]
    splits = 0
    while stack:
        q = stack.pop()
        if any(c.wid() == 0.0 for c in q):
            return "Err", splits
        m = (q[0].mid(), q[1].mid())
        ok = q[0].lo <= m[0] <= q[0].hi and q[1].lo <= m[1] <= q[1].hi
        if not ok:
            return "Err", splits
        y = system.preconditioner(m[0], m[1])
        if y is None:
            if splits >= max_split:
                return "Err", splits
            splits += 1
            stack.extend(_bisect(q))
            continue
        f = system.f_point(m[0], m[1])
        j = system.jacobian(q)
        strict = True
        empty = False
        kimg = []
        for r in range(2):
            acc = Iv.c(m[r])
            for c in range(2):
                acc = acc - y[r][c] * f[c]
            for c in range(2):
                d_iv = Iv(dn(1.0 if r == c else 0.0), up(1.0 if r == c else 0.0))
                yj = y[r][c] * j[r][c]
                dd = d_iv - yj
                qm = Iv(dn(q[c].lo - m[c]), up(q[c].hi - m[c]))
                acc = acc + dd * qm
            kimg.append(acc)
        for r in range(2):
            kv, qv = kimg[r], q[r]
            if kv.lo <= qv.lo and kv.hi >= qv.hi:
                pass
            if not (kv.lo > qv.lo and kv.hi < qv.hi):
                strict = False
            if max(kv.lo, qv.lo) > min(kv.hi, qv.hi):
                empty = True
        if strict:
            return "Unique", splits
        if empty:
            continue
        if splits >= max_split:
            return "Err", splits
        splits += 1
        stack.extend(_bisect(q))
    return "NoRoot", splits


def _bisect(q):
    wu, wv = q[0].wid(), q[1].wid()
    if wu >= wv:
        (a, b) = split(q[0])
        return [(a, q[1]), (b, q[1])]
    (a, b) = split(q[1])
    return [(q[0], a), (q[0], b)]


# ---------------- surface (iv-b) discharge ----------------

def adjacent2d(iu1, iv1, iu2, iv2, nu, nv, closed_u, closed_v):
    def wrap_d(d, n, closed):
        if closed and n > 2:
            wd = min(d, n - d)
        else:
            wd = d
        return wd

    du = abs(iu1 - iu2)
    dv = abs(iv1 - iv2)
    du = wrap_d(du, nu, closed_u)
    dv = wrap_d(dv, nv, closed_v)
    return max(du, dv) <= 1


def surface_ivb(surf, exact, closed_u, closed_v, cell_eps, check_vertices=True, budget_vertices=4000):
    """Returns ('Pass', None) or ('MultiSheet', (j, k)) or ('ProjectionFailure', None)."""
    nu, nv = surf.nu, surf.nv
    # (b) grid-vertex projection correspondence
    if check_vertices:
        for iu in range(1, nu):
            for iv_ in range(1, nv):
                u_star = surf.us[iu]
                v_star = surf.vs[iv_]
                wu = max(u_star - surf.us[iu - 1], surf.us[iu + 1] - u_star)
                wv = max(v_star - surf.vs[iv_ - 1], surf.vs[iv_ + 1] - v_star)
                phi = exact.subs(u_star, v_star)
                sysv = VertexSystem(exact, phi)
                start = (Iv(u_star - wu, u_star + wu), Iv(v_star - wv, v_star + wv))
                verdict, _ = krawczyk2(sysv, start)
                if verdict != "Unique":
                    return ("ProjectionFailure", None)
                budget_vertices -= 1
                if budget_vertices < 0:
                    return ("ProjectionFailure", None)
    # (c) non-adjacent separation, whole-cell boxes (float-fast scan)
    nu, nv = surf.nu, surf.nv
    n = nu * nv
    eh = [[None] * 3 for _ in range(n)]
    el = [[None] * 3 for _ in range(n)]
    xh = [[None] * 3 for _ in range(n)]
    xl = [[None] * 3 for _ in range(n)]
    for iu in range(nu):
        for iv_ in range(nv):
            j = iu * nv + iv_
            hb = surf.cell(iu, iv_).enclose(surf.us[iu], surf.us[iu + 1], surf.vs[iv_], surf.vs[iv_ + 1])
            eb = exact.enclose(Iv(surf.us[iu], surf.us[iu + 1]), Iv(surf.vs[iv_], surf.vs[iv_ + 1]))
            for k, b in enumerate((hb.x, hb.y, hb.z)):
                el[j][k], eh[j][k] = b.lo, b.hi
            for k, b in enumerate((eb.x, eb.y, eb.z)):
                xl[j][k], xh[j][k] = b.lo, b.hi
    for j in range(n):
        iu, iv_ = j // nv, j % nv
        epsj = cell_eps[j]
        for k in range(n):
            if k == j:
                continue
            ku, kv = k // nv, k % nv
            if adjacent2d(iu, iv_, ku, kv, nu, nv, closed_u, closed_v):
                continue
            d2 = 0.0
            for c in range(3):
                g = max(xl[k][c] - eh[j][c], el[j][c] - xh[k][c], 0.0)
                d2 += g * g
            if math.sqrt(d2) <= epsj:
                return ("MultiSheet", (j, k))
    return ("Pass", None)


# ---------------- refine-loop simulation ----------------

def simulate_rep(exact, closed_u, closed_v, tau, gap, initial_depth=0, max_attempts=40, verbose=True):
    (u0, u1), (v0, v1) = exact.range()
    curv, spend_c, msg_c = curvature_span(exact)
    sep, spend_s, msg_s = separation_span(exact, closed_u, closed_v, gap)
    if verbose:
        print("  curvature_span: %.6f (%s, spend %d)" % (curv, msg_c, spend_c))
        print("  separation_span: %.6f (%s, spend %d)" % (sep, msg_s, spend_s))
    if curv is None or sep is None:
        print("  -> scale refusal (collapse route)")
        return None
    tube = min(curv, 0.5 * sep)
    target = min(tau, 0.5 * tube)
    if verbose:
        print("  tube_scale_lower = %.6f, target_eps = %.6f" % (tube, target))
    du = initial_depth
    dv = initial_depth
    stalls = 0
    prev_eps = math.inf
    spent = 0
    for attempt in range(max_attempts):
        spent += 1
        surf = HermiteSurface(exact, du, dv, u0, u1, v0, v1)
        eps_now, theta_now, cell_eps, ext_u, ext_v = measure(surf, exact)
        if eps_now > target:
            if prev_eps < math.inf and eps_now >= prev_eps - 0.01 * prev_eps:
                stalls += 1
                if stalls >= 2:
                    print("  -> Unresolved (stall) at (du=%d, dv=%d), eps %.6f" % (du, dv, eps_now))
                    return ("Unresolved", spent)
            else:
                stalls = 0
            prev_eps = eps_now
            if verbose:
                print("  attempt (du=%d, dv=%d): eps %.4f > target %.4f -> refine" % (du, dv, eps_now, target))
            if ext_u >= ext_v:
                du += 1
            else:
                dv += 1
            continue
        s = target / tube
        if theta_now <= s:
            if verbose:
                print("  attempt (du=%d, dv=%d): theta %.4f <= s %.4f -> refine" % (du, dv, theta_now, s))
            if ext_u >= ext_v:
                du += 1
            else:
                dv += 1
            continue
        outcome, pair = surface_ivb(surf, exact, closed_u, closed_v, cell_eps)
        if outcome == "Pass":
            # margins summary: separation margin = min over non-adjacent pairs
            # of (box_distance - cell_eps[j]); theta margin = theta - s.
            min_gap_margin = math.inf
            nu, nv = surf.nu, surf.nv
            n = nu * nv
            boxes_h = [surf.cell(iu, iv_).enclose(surf.us[iu], surf.us[iu + 1], surf.vs[iv_], surf.vs[iv_ + 1])
                       for iu in range(nu) for iv_ in range(nv)]
            boxes_e = [exact.enclose(Iv(surf.us[iu], surf.us[iu + 1]), Iv(surf.vs[iv_], surf.vs[iv_ + 1]))
                       for iu in range(nu) for iv_ in range(nv)]
            for j in range(n):
                iu, iv_ = j // nv, j % nv
                for k in range(n):
                    if k == j:
                        continue
                    ku, kv = k // nv, k % nv
                    if adjacent2d(iu, iv_, ku, kv, nu, nv, closed_u, closed_v):
                        continue
                    m = box_distance(boxes_h[j], boxes_e[k]) - cell_eps[j]
                    if m < min_gap_margin:
                        min_gap_margin = m
            print("  EMIT at (du=%d, dv=%d): eps %.6f (target %.6f, margin %.2fx), theta %.6f (s %.6f, margin %.2fx), sep margin %.6f, spent %d"
                  % (du, dv, eps_now, target, target / eps_now, theta_now, s, theta_now / s if s > 0 else math.inf, min_gap_margin, spent))
            return ("Ok", (du, dv, eps_now, theta_now, spent, curv, sep))
        if outcome == "MultiSheet":
            j, k = pair
            nv = surf.nv
            ju, jv = j // nv, j % nv
            ku, kv = k // nv, k % nv
            if verbose:
                print("  attempt (du=%d, dv=%d): MultiSheet cells (%d,%d)x(%d,%d) -> refine" % (du, dv, ju, jv, ku, kv))
            if ju == ku:
                du += 1  # failing pair separates only in v: refine u
            elif jv == kv:
                dv += 1
            elif ext_u >= ext_v:
                du += 1
            else:
                dv += 1
            continue
        if verbose:
            print("  attempt (du=%d, dv=%d): ProjectionFailure -> refine" % (du, dv))
        if ext_u >= ext_v:
            du += 1
        else:
            dv += 1
        continue
    print("  -> attempts exhausted")
    return ("Unresolved", spent)


def main():
    print("== corner reproduction of the 16-point net (twist-sign check) ==")
    belt = SphereBelt(2.0, PI / 4, 3 * PI / 4, 0.0, TWO_PI)
    hu = PI / 4
    hv = PI / 2
    Q = build_cell(belt, PI / 4, PI / 4 + hu, 0.0, hv)
    errs = []
    for (i, j) in [(0, 0), (3, 0), (0, 3), (3, 3)]:
        u = PI / 4 + (hu if i == 3 else 0.0)
        v = 0.0 + (hv if j == 3 else 0.0)
        p = Q[i][j]
        errs.append(max(abs(p[k] - belt.subs(u, v)[k]) for k in range(3)))
        if i == 0:
            tu = tuple(3.0 * (Q[1][j][k] - Q[0][j][k]) / hu for k in range(3))
        else:
            tu = tuple(3.0 * (Q[3][j][k] - Q[2][j][k]) / hu for k in range(3))
        eu = belt.der_mn(1, 0, u, v)
        errs.append(max(abs(tu[k] - eu[k]) for k in range(3)))
        if j == 0:
            tv = tuple(3.0 * (Q[i][1][k] - Q[i][0][k]) / hv for k in range(3))
        else:
            tv = tuple(3.0 * (Q[i][3][k] - Q[i][2][k]) / hv for k in range(3))
        ev = belt.der_mn(0, 1, u, v)
        errs.append(max(abs(tv[k] - ev[k]) for k in range(3)))
    # twists: the mixed-difference relation at each corner's adjacent block
    rels = [((1, 1), (1, 0), (0, 1), (0, 0), (0.0, 0.0)),
            ((3, 1), (2, 1), (3, 0), (2, 0), (hu, 0.0)),
            ((1, 3), (1, 2), (0, 3), (0, 2), (0.0, hv)),
            ((3, 3), (2, 3), (3, 2), (2, 2), (hu, hv))]
    for (pa, pb, pc, pd, (cu_, cv_)) in rels:
        u = PI / 4 + cu_
        v = 0.0 + cv_
        tw = tuple(9.0 * (Q[pa[0]][pa[1]][k] - Q[pb[0]][pb[1]][k] - Q[pc[0]][pc[1]][k] + Q[pd[0]][pd[1]][k]) / (hu * hv) for k in range(3))
        ew = belt.der_mn(1, 1, u, v)
        errs.append(max(abs(tw[k] - ew[k]) for k in range(3)))
    print("  max corner data reproduction error (pos/tan/twist, 16 conditions): %.3e" % max(errs))

    print()
    print("== belt witness: R=2, u in [pi/4, 3pi/4], v in [0,2pi], ClosedV, tau=0.3, gap=pi ==")
    r = simulate_rep(belt, False, True, 0.3, PI)
    print()

    print("== open patch: [pi/4,3pi/4]^2, Open, tau=0.3, gap=pi ==")
    patch = SphereBelt(2.0, PI / 4, 3 * PI / 4, PI / 4, 3 * PI / 4)
    r = simulate_rep(patch, False, False, 0.3, PI)
    print()

    print("== small belt: R=0.3, same spans, ClosedV, tau=0.3, gap=pi ==")
    small = SphereBelt(0.3, PI / 4, 3 * PI / 4, 0.0, TWO_PI)
    r = simulate_rep(small, False, True, 0.3, PI)
    print()

    print("== graph fixture: (u, v, 0.5 + 0.5 sin u sin v) over [pi/4,3pi/4]^2, Open, tau=0.3 ==")
    graph = GraphZ(0.5, 0.5, 1.0, PI / 4, 3 * PI / 4, PI / 4, 3 * PI / 4)
    r = simulate_rep(graph, False, False, 0.3, PI)
    print()

    print("== pole patch: R=2, u in [0, pi/3], v in [pi/4, 3pi/4] (touches the north pole) ==")
    pole = SphereBelt(2.0, 0.0, PI / 3, PI / 4, 3 * PI / 4)
    # the pole cell's immersion bound is 0 at every level -> budget-bounded refusal
    for lvl in [0, 1, 2, 3]:
        nu = 2 ** lvl
        uu = Iv(0.0, (PI / 3) / nu)
        vv = Iv(PI / 4, PI / 4 + (PI / 2) / max(nu, 1))
        su = pole.enclose_der(1, 0, uu, vv)
        sv = pole.enclose_der(0, 1, uu, vv)
        iota = immersion_lower_bound_box(cross_box(su, sv))
        print("  level %d pole cell immersion lower bound: %.6f" % (lvl, iota))
    curv, spend, msg = curvature_span(pole, budget=2 ** 12)
    print("  curvature_span with budget 4096: %s (%s, spend %d) -> rep routes to ReachTooSmall"
          % (curv, msg, spend))
    print()

    print("== double-cover witness: D(u,v) = (R + a cos(u/2))(sin v cos u, sin v sin u, cos v) ==")
    print("   R=2, a=0.025 (=eps/2 with eps=0.05), u in [0,4pi] ClosedU, v in [pi/4, 3pi/4]")
    dbl = DoubleCover(2.0, 0.025, PI / 4, 3 * PI / 4)
    # (1) deviation from the sphere STRICTLY inside eps (the test-3 trap)
    dev = 0.0
    N = 400
    for i in range(N + 1):
        u = 4 * PI * i / N
        for j in range(4):
            v = PI / 4 + (PI / 2) * j / 3.0
            p = dbl.subs(u, v)
            rad = math.sqrt(p[0] * p[0] + p[1] * p[1] + p[2] * p[2])
            dev = max(dev, abs(rad - 2.0))
    print("  max |radius - R| over dense sample: %.6f  (a = 0.025 < eps = 0.05: margin %.2fx)"
          % (dev, 0.05 / max(dev, 1e-12)))
    # (2) tangent planes on BOTH sheets agree with the sphere's (normal |cos|)
    worst = 1.0
    for i in range(N + 1):
        u = 4 * PI * i / N
        for j in range(4):
            v = PI / 4 + (PI / 2) * j / 3.0
            du = dbl.der_mn(1, 0, u, v)
            dv = dbl.der_mn(0, 1, u, v)
            nrm = (du[1] * dv[2] - du[2] * dv[1],
                   du[2] * dv[0] - du[0] * dv[2],
                   du[0] * dv[1] - du[1] * dv[0])
            nn = math.sqrt(sum(c * c for c in nrm))
            sph = (math.sin(v) * math.cos(u), math.sin(v) * math.sin(u), math.cos(v))
            cosv = abs(sum(nrm[k] * sph[k] for k in range(3))) / nn
            worst = min(worst, cosv)
    print("  min |cos| between sheet normals and the sphere normal (both sheets): %.6f" % worst)
    # (3) the scale components: separation ~0 (the two sheets coincide) -> tube 0
    sep_d, spend_d, msg_d = separation_span(dbl, True, False, PI)
    print("  separation_span at gap=pi: %.6f (%s, spend %d) -> tube ~0 -> loop stalls Unresolved"
          % (sep_d, msg_d, spend_d))
    curv_d, spend_c, msg_c = curvature_span(dbl)
    print("  curvature_span: %.6f (%s, spend %d)" % (curv_d, msg_c, spend_c))
    # (4) the direct discharge at a fixed grid reports MultiSheet
    for (fdu, fdv) in [(6, 5), (7, 5)]:
        surf = HermiteSurface(dbl, fdu, fdv, 0.0, 4 * PI, PI / 4, 3 * PI / 4)
        eps_now, theta_now, cell_eps, _, _ = measure(surf, dbl)
        outcome, pair = surface_ivb(surf, dbl, True, False, cell_eps)
        nu, nv = surf.nu, surf.nv
        detail = ""
        if pair is not None:
            j, k = pair
            ju, jv = j // nv, j % nv
            ku, kv = k // nv, k % nv
            detail = " cells (%d,%d) x (%d,%d): du_index=%d (half grid = %d)" % (ju, jv, ku, kv, abs(ju - ku), nu // 2)
        print("  fixed grid (%d,%d): eps %.4f theta %.4f -> %s%s" % (fdu, fdv, eps_now, theta_now, outcome, detail))
    print()

    print("== re-rep of the emission (idempotence): rep(belt) then rep(&emission) ==")
    # the emission itself as the exact input: it must re-emit at eps <= tau
    (u0, u1), (v0, v1) = belt.range()
    em = HermiteSurface(belt, 5, 6, u0, u1, v0, v1)
    r2 = simulate_rep(em, False, True, 0.3, PI, initial_depth=4)
    print()

    print("== transposed parameterization: rep(S) vs rep(S^T) ==")
    class Transposed:
        def __init__(self, base):
            self.base = base

        def range(self):
            (a0, a1), (b0, b1) = self.base.range()
            return (b0, b1), (a0, a1)

        def subs(self, u, v):
            return self.base.subs(v, u)

        def der_mn(self, m, n, u, v):
            return self.base.der_mn(n, m, v, u)

        def enclose(self, uu, vv):
            return self.base.enclose(vv, uu)

        def enclose_der(self, m, n, uu, vv):
            return self.base.enclose_der(n, m, vv, uu)

    patch = SphereBelt(2.0, PI / 4, 3 * PI / 4, PI / 4, 3 * PI / 4)
    tr = Transposed(patch)
    r3 = simulate_rep(tr, False, False, 0.3, PI)
    print()

    print("== budget spend of the scale components (for the budget-exhaustion test) ==")
    belt = SphereBelt(2.0, PI / 4, 3 * PI / 4, 0.0, TWO_PI)
    curv_b, spend_curv, _ = curvature_span(belt)
    sep_b, spend_sep, _ = separation_span(belt, False, True, PI)
    print("  belt scale spend: curvature %d + separation %d = %d"
          % (spend_curv, spend_sep, spend_curv + spend_sep))
    print()
    print("ALL CHECKS COMPLETE")


if __name__ == "__main__":
    main()

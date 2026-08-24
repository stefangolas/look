"""Interval core + fixtures + tensor-product Hermite emitter for the
BG-FID-005-SRF machine-check. Outward-rounded interval arithmetic mirroring
inari's semantics (directed rounding via nextafter); sin/cos true-range
enclosures with interior extrema (mirroring elementary.rs); box helpers
mirroring isotopy.rs; fixtures mirror the module's per-coordinate interval
product style (sphere.rs pattern).
"""
import math

F64_EPS = 2.0 ** -52


def dn(x):
    return math.nextafter(x, -math.inf)


def up(x):
    return math.nextafter(x, math.inf)


class Iv:
    __slots__ = ("lo", "hi")

    def __init__(self, lo, hi):
        assert lo <= hi, (lo, hi)
        self.lo = lo
        self.hi = hi

    @staticmethod
    def c(x):
        return Iv(x, x)

    @staticmethod
    def mk(lo, hi):
        if lo > hi:
            lo, hi = hi, lo
        return Iv(dn(lo), up(hi))

    def mid(self):
        return 0.5 * (self.lo + self.hi)

    def wid(self):
        return self.hi - self.lo

    def contains(self, x):
        return self.lo <= x <= self.hi

    def hull(self, o):
        return Iv(min(self.lo, o.lo), max(self.hi, o.hi))

    def __add__(self, o):
        return Iv(dn(self.lo + o.lo), up(self.hi + o.hi))

    def __neg__(self):
        return Iv(dn(-self.hi), up(-self.lo))

    def __sub__(self, o):
        return Iv(dn(self.lo - o.hi), up(self.hi - o.lo))

    def __mul__(self, o):
        if isinstance(o, (int, float)):
            o = Iv.c(float(o))
        c = [self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi]
        return Iv(dn(min(c)), up(max(c)))

    def __rmul__(self, o):
        return self.__mul__(o)

    def sqr(self):
        return self * self

    def sqrt(self):
        return Iv(dn(math.sqrt(max(self.lo, 0.0))), up(math.sqrt(max(self.hi, 0.0))))

    def __repr__(self):
        return "[%.17g, %.17g]" % (self.lo, self.hi)


PI = math.pi
TWO_PI = 2.0 * math.pi


def iv_sin(x):
    """True range of sin over [lo,hi] incl. interior extrema, 1 ulp out."""
    if x.wid() >= TWO_PI:
        return Iv(-1.0, 1.0)
    lo, hi = x.lo, x.hi
    vals = [math.sin(lo), math.sin(hi)]
    k = math.floor((lo - PI / 2) / PI) - 1
    k1 = math.ceil((hi - PI / 2) / PI) + 1
    while k <= k1:
        c = PI / 2 + k * PI
        if lo < c < hi:
            vals.append(1.0 if k % 2 == 0 else -1.0)
        k += 1
    return Iv(dn(min(vals)), up(max(vals)))


def iv_cos(x):
    return iv_sin(Iv(dn(x.lo + PI / 2), up(x.hi + PI / 2)))


class B3:
    __slots__ = ("x", "y", "z")

    def __init__(self, x, y, z):
        self.x, self.y, self.z = x, y, z

    def wid(self):
        return max(self.x.wid(), self.y.wid(), self.z.wid())

    def hull(self, o):
        return B3(self.x.hull(o.x), self.y.hull(o.y), self.z.hull(o.z))

    def __repr__(self):
        return "B3(%r, %r, %r)" % (self.x, self.y, self.z)


def dot_box(a, b):
    return a.x * b.x + a.y * b.y + a.z * b.z


def cross_box(a, b):
    return B3(a.y * b.z - a.z * b.y,
              a.z * b.x - a.x * b.z,
              a.x * b.y - a.y * b.x)


def box_distance(a, b):
    def gap(A, B):
        return max(B.lo - A.hi, A.lo - B.hi, 0.0)

    dx, dy, dz = gap(a.x, b.x), gap(a.y, b.y), gap(a.z, b.z)
    return math.sqrt(dx * dx + dy * dy + dz * dz)


def sup_distance_box(a, b):
    def far(A, B):
        return max(abs(A.lo - B.hi), abs(A.hi - B.lo))

    dx, dy, dz = far(a.x, b.x), far(a.y, b.y), far(a.z, b.z)
    return math.sqrt(dx * dx + dy * dy + dz * dz)


def norm_sup(b):
    return (b.x.sqr() + b.y.sqr() + b.z.sqr()).sqrt().hi


def norm_inf(b):
    return (b.x.sqr() + b.y.sqr() + b.z.sqr()).sqrt().lo


def abs_lower(i):
    if i.contains(0.0):
        return 0.0
    return min(abs(i.lo), abs(i.hi))


def abs_upper(i):
    return max(abs(i.lo), abs(i.hi))


def angle_pass_form(a, b):
    na, nb = norm_sup(a), norm_sup(b)
    if na == 0.0 or nb == 0.0:
        return 0.0
    return abs_lower(dot_box(a, b)) / (na * nb)


def mig(i):
    if i.contains(0.0):
        return 0.0
    return min(abs(i.lo), abs(i.hi))


def immersion_lower_bound_box(b):
    return (Iv.c(mig(b.x)).sqr() + Iv.c(mig(b.y)).sqr() + Iv.c(mig(b.z)).sqr()).sqrt().lo


HULL_PAD = 64.0 * F64_EPS


def pad_iv(lo, hi):
    pad = HULL_PAD * (1.0 + max(abs(lo), abs(hi)))
    return Iv(dn(lo - pad), up(hi + pad))


def hull_pts(pts):
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]
    return B3(pad_iv(min(xs), max(xs)), pad_iv(min(ys), max(ys)), pad_iv(min(zs), max(zs)))


# ---------------- fixtures ----------------

class SphereBelt:
    """S(u,v) = R (sin u cos v, sin u sin v, cos u); u polar, v azimuth."""

    def __init__(self, r, u0, u1, v0, v1):
        self.r, self.u0, self.u1, self.v0, self.v1 = r, u0, u1, v0, v1

    def subs(self, u, v):
        return (self.r * math.sin(u) * math.cos(v),
                self.r * math.sin(u) * math.sin(v),
                self.r * math.cos(u))

    def der_mn(self, m, n, u, v):
        # d^k/dx^k sin x table and cos table
        def dt(base, k, x):
            # returns d^k/dx^k of base(x), base in {sin, cos}
            if base == "s":
                tbl = [math.sin(x), math.cos(x), -math.sin(x), -math.cos(x)]
            else:
                tbl = [math.cos(x), -math.sin(x), -math.cos(x), math.sin(x)]
            return tbl[k % 4]

        x = self.r * dt("s", m, u) * dt("c", n, v)
        y = self.r * dt("s", m, u) * dt("s", n, v)
        z = self.r * dt("c", m, u) * (1.0 if n == 0 else 0.0)
        return (x, y, z)

    def range(self):
        return (self.u0, self.u1), (self.v0, self.v1)

    def enclose(self, uu, vv):
        r = Iv.c(self.r)
        su, cu = iv_sin(uu), iv_cos(uu)
        sv, cv = iv_sin(vv), iv_cos(vv)
        return B3(r * su * cv, r * su * sv, r * cu)

    def enclose_der(self, m, n, uu, vv):
        r = Iv.c(self.r)
        su, cu = iv_sin(uu), iv_cos(uu)
        sv, cv = iv_sin(vv), iv_cos(vv)
        # u-factor (sin-side, cos-side) per m%4
        uf = {0: (su, cu), 1: (cu, -su), 2: (-su, -cu), 3: (-cu, su)}[m % 4]
        # v-factor (cos-side, sin-side) per n%4
        vf = {0: (cv, sv), 1: (-sv, cv), 2: (-cv, -sv), 3: (sv, -cv)}[n % 4]
        vz = Iv.c(1.0) if n == 0 else Iv.c(0.0)
        return B3(r * uf[0] * vf[0], r * uf[0] * vf[1], r * uf[1] * vz)


class GraphZ:
    """G(u,v) = (u, v, A + B sin(k u) sin(k v)); trivial derivative table."""

    def __init__(self, a, b, k, u0, u1, v0, v1):
        self.a, self.b, self.k = a, b, k
        self.u0, self.u1, self.v0, self.v1 = u0, u1, v0, v1

    def subs(self, u, v):
        return (u, v, self.a + self.b * math.sin(self.k * u) * math.sin(self.k * v))

    def _dsin(self, k, x, order):
        # d^order/dx^order sin(k x) as float
        tbl = [math.sin(k * x), math.cos(k * x), -math.sin(k * x), -math.cos(k * x)]
        return (k ** order) * tbl[order % 4]

    def _dsin_iv(self, k, x, order):
        arg = Iv(dn(k * x.lo), up(k * x.hi))
        fns = [iv_sin, iv_cos, lambda t: -iv_sin(t), lambda t: -iv_cos(t)]
        return fns[order % 4](arg) * Iv.c(k ** order)

    def der_mn(self, m, n, u, v):
        x = 1.0 if (m, n) == (1, 0) else 0.0
        y = 1.0 if (m, n) == (0, 1) else 0.0
        z = 0.0
        if m + n > 0:
            z = self.b * self._dsin(self.k, u, m) * self._dsin(self.k, v, n)
        elif m == 0 and n == 0:
            return self.subs(u, v)
        return (x, y, z)

    def range(self):
        return (self.u0, self.u1), (self.v0, self.v1)

    def enclose(self, uu, vv):
        z = Iv.c(self.a) + Iv.c(self.b) * self._dsin_iv(self.k, uu, 0) * self._dsin_iv(self.k, vv, 0)
        return B3(uu, vv, z)

    def enclose_der(self, m, n, uu, vv):
        if m == 0 and n == 0:
            return self.enclose(uu, vv)
        x = Iv.c(1.0) if (m, n) == (1, 0) else Iv.c(0.0)
        y = Iv.c(1.0) if (m, n) == (0, 1) else Iv.c(0.0)
        z = Iv.c(self.b) * self._dsin_iv(self.k, uu, m) * self._dsin_iv(self.k, vv, n)
        return B3(x, y, z)


class DoubleCover:
    """D(u,v) = (R + a cos(u/2)) (sin v cos u, sin v sin u, cos v),
    u in [0, 4pi] (closed in u; azimuth covered twice), v in [v0, v1]."""

    def __init__(self, r, a, v0, v1):
        self.r, self.a = r, a
        self.u0, self.u1 = 0.0, 4.0 * math.pi
        self.v0, self.v1 = v0, v1

    def _rho(self, u):
        return self.r + self.a * math.cos(0.5 * u)

    def _rho_d(self, u, m):
        if m == 0:
            return self._rho(u)
        # d^m/du^m a cos(u/2) = a (1/2)^m cos(u/2 + m pi/2)
        return self.a * (0.5 ** m) * math.cos(0.5 * u + m * PI / 2)

    def _rho_iv(self, uu, m):
        half = Iv(dn(0.5 * uu.lo), up(0.5 * uu.hi))
        if m == 0:
            return Iv.c(self.a) * iv_cos(half) + Iv.c(self.r)
        arg = half
        fns = [iv_cos, lambda t: -iv_sin(t), lambda t: -iv_cos(t), iv_sin]
        return fns[m % 4](arg) * Iv.c(self.a * (0.5 ** m))

    def subs(self, u, v):
        rho = self._rho(u)
        return (rho * math.sin(v) * math.cos(u),
                rho * math.sin(v) * math.sin(u),
                rho * math.cos(v))

    def der_mn(self, m, n, u, v):
        # Leibniz over u-factors; v-factor shifted
        def cu_d(j):
            return math.cos(u) if j == 0 else math.cos(u + j * PI / 2)

        def su_d(j):
            return math.sin(u) if j == 0 else math.sin(u + j * PI / 2)

        x = 0.0
        y = 0.0
        for j in range(m + 1):
            c = math.comb(m, j)
            x += c * self._rho_d(u, j) * cu_d(m - j)
            y += c * self._rho_d(u, j) * su_d(m - j)
        x *= math.sin(v + n * PI / 2) if n else math.sin(v)
        y *= math.sin(v + n * PI / 2) if n else math.sin(v)
        z = self._rho_d(u, m) * (math.cos(v + n * PI / 2) if n else math.cos(v))
        return (x, y, z)

    def range(self):
        return (self.u0, self.u1), (self.v0, self.v1)

    def enclose(self, uu, vv):
        rho = self._rho_iv(uu, 0)
        return B3(rho * iv_sin(vv) * iv_cos(uu),
                  rho * iv_sin(vv) * iv_sin(uu),
                  rho * iv_cos(vv))

    def enclose_der(self, m, n, uu, vv):
        def cu_j(j):
            return iv_cos(uu) if j == 0 else iv_cos(uu + Iv.c(j * PI / 2))

        def su_j(j):
            return iv_sin(uu) if j == 0 else iv_sin(uu + Iv.c(j * PI / 2))

        x = Iv.c(0.0)
        y = Iv.c(0.0)
        for j in range(m + 1):
            c = Iv.c(float(math.comb(m, j)))
            x = x + self._rho_iv(uu, j) * cu_j(m - j) * c
            y = y + self._rho_iv(uu, j) * su_j(m - j) * c
        sv = iv_sin(vv) if n == 0 else iv_sin(vv + Iv.c(n * PI / 2))
        cv = iv_cos(vv) if n == 0 else iv_cos(vv + Iv.c(n * PI / 2))
        return B3(x * sv, y * sv, self._rho_iv(uu, m) * cv)


# ---------------- tensor-product bicubic Hermite emitter ----------------

def lerp3(p, q, t):
    return (p[0] + (q[0] - p[0]) * t,
            p[1] + (q[1] - p[1]) * t,
            p[2] + (q[2] - p[2]) * t)


def bez_split_col(col, t):
    """de Casteljau split of a cubic (4 pts) at t -> (left, right)."""
    q0 = lerp3(col[0], col[1], t)
    q1 = lerp3(col[1], col[2], t)
    q2 = lerp3(col[2], col[3], t)
    r0 = lerp3(q0, q1, t)
    r1 = lerp3(q1, q2, t)
    s0 = lerp3(r0, r1, t)
    return [col[0], q0, r0, s0], [s0, r1, q2, col[3]]


def build_cell(exact, a, b, c, d):
    """The 16-point control net of the bicubic Hermite patch over
    [a,b]x[c,d] from the exact surface's corner data. Positions via subs;
    tangents and twists as midpoints of degenerate enclosures (the module's
    deterministic convention)."""
    hu, hv = b - a, d - c
    P00 = exact.subs(a, c)
    P30 = exact.subs(b, c)
    P03 = exact.subs(a, d)
    P33 = exact.subs(b, d)

    def der_mid(m, n, u, v):
        bb = exact.enclose_der(m, n, Iv.c(u), Iv.c(v))
        return ((bb.x.mid(), bb.y.mid(), bb.z.mid()))

    U00 = der_mid(1, 0, a, c)
    U30 = der_mid(1, 0, b, c)
    U03 = der_mid(1, 0, a, d)
    U33 = der_mid(1, 0, b, d)
    V00 = der_mid(0, 1, a, c)
    V30 = der_mid(0, 1, b, c)
    V03 = der_mid(0, 1, a, d)
    V33 = der_mid(0, 1, b, d)
    W00 = der_mid(1, 1, a, c)
    W30 = der_mid(1, 1, b, c)
    W03 = der_mid(1, 1, a, d)
    W33 = der_mid(1, 1, b, d)

    def add(*ps):
        return tuple(sum(p[k] for p in ps) for k in range(3))

    def scale(p, s):
        return (p[0] * s, p[1] * s, p[2] * s)

    hu3, hv3 = hu / 3.0, hv / 3.0
    wh = hu * hv / 9.0
    Q = [[None] * 4 for _ in range(4)]
    # corners
    Q[0][0], Q[3][0], Q[0][3], Q[3][3] = P00, P30, P03, P33
    # u-edge tangents
    Q[1][0] = add(P00, scale(U00, hu3))
    Q[2][0] = add(P30, scale(U30, -hu3))
    Q[1][3] = add(P03, scale(U03, hu3))
    Q[2][3] = add(P33, scale(U33, -hu3))
    # v-edge tangents
    Q[0][1] = add(P00, scale(V00, hv3))
    Q[0][2] = add(P03, scale(V03, -hv3))
    Q[3][1] = add(P30, scale(V30, hv3))
    Q[3][2] = add(P33, scale(V33, -hv3))
    # interiors: twist signs + at (a,c),(b,d); - at (b,c),(a,d)
    Q[1][1] = add(P00, scale(U00, hu3), scale(V00, hv3), scale(W00, wh))
    Q[2][1] = add(P30, scale(U30, -hu3), scale(V30, hv3), scale(W30, -wh))
    Q[1][2] = add(P03, scale(U03, hu3), scale(V03, -hv3), scale(W03, -wh))
    Q[2][2] = add(P33, scale(U33, -hu3), scale(V33, -hv3), scale(W33, wh))
    return Q


class SurfCell:
    def __init__(self, a, b, c, d, Q):
        self.a, self.b, self.c, self.d = a, b, c, d
        self.hu, self.hv = b - a, d - c
        self.Q = Q

    def eval(self, u, v):
        s = (u - self.a) / self.hu
        t = (v - self.c) / self.hv

        def bern(i, x):
            return math.comb(3, i) * ((1 - x) ** (3 - i)) * (x ** i)

        px = py = pz = 0.0
        for i in range(4):
            for j in range(4):
                w = bern(i, s) * bern(j, t)
                px += w * self.Q[i][j][0]
                py += w * self.Q[i][j][1]
                pz += w * self.Q[i][j][2]
        return (px, py, pz)

    def eval_du(self, u, v):
        # first u-derivative of the bicubic (chain rule with s)
        s = (u - self.a) / self.hu
        t = (v - self.c) / self.hv

        def bern(i, x):
            return math.comb(3, i) * ((1 - x) ** (3 - i)) * (x ** i)

        def dbern(i, x):
            if i == 0:
                return -3.0 * (1 - x) ** 2
            if i == 1:
                return 3.0 * (1 - x) ** 2 - 6.0 * x * (1 - x)
            if i == 2:
                return 6.0 * x * (1 - x) - 3.0 * x * x
            return 3.0 * x * x

        px = py = pz = 0.0
        for i in range(4):
            for j in range(4):
                w = dbern(i, s) * bern(j, t)
                px += w * self.Q[i][j][0]
                py += w * self.Q[i][j][1]
                pz += w * self.Q[i][j][2]
        return (px / self.hu, py / self.hu, pz / self.hu)

    def der_mn(self, m, n, u, v):
        """The (m, n)-th derivative at a point, via the u-derivative column
        then the 1D v evaluation."""
        if m + n == 0:
            return self.eval(u, v)
        s = (u - self.a) / self.hu
        t = (v - self.c) / self.hv
        cols = _u_der_column(self.Q, m, s, self.hu)
        # evaluate the v-curve (control points cols) and its n-th derivative at t
        if n == 0:
            return tuple(sum(_bern(3, j, t) * cols[j][d] for j in range(4)) for d in range(3))
        pts = _curve_der_points(cols, n, self.hv)
        deg = 3 - n
        return tuple(sum(_bern(deg, j, t) * pts[j][d] for j in range(len(pts))) for d in range(3))

    def restrict_net(self, ulo, uhi, vlo, vhi):
        """16 control points of the sub-patch over [ulo,uhi]x[vlo,vhi]."""
        def axis(lo_f, hi_f):
            if lo_f >= 1.0:
                return "end"
            if hi_f <= 0.0:
                return "start"
            if lo_f <= 0.0 and hi_f >= 1.0:
                return "full"
            return (lo_f, hi_f)

        ru = axis((ulo - self.a) / self.hu, (uhi - self.a) / self.hu)
        rv = axis((vlo - self.c) / self.hv, (vhi - self.c) / self.hv)

        def restrict_cols(net, mode):
            if mode == "full":
                return net
            out = [[None] * 4 for _ in range(4)]
            for j in range(4):
                col = [net[i][j] for i in range(4)]
                if mode == "end":
                    r = [col[3]] * 4
                elif mode == "start":
                    r = [col[0]] * 4
                else:
                    _, r1 = bez_split_col(col, mode[0])
                    if mode[1] >= 1.0:
                        r = r1
                    elif mode[0] < 1.0:
                        tt = (mode[1] - mode[0]) / (1.0 - mode[0])
                        sub, _ = bez_split_col(r1, tt)
                        r = sub
                    else:
                        r = r1
                for i in range(4):
                    out[i][j] = r[i]
            return out

        def restrict_rows(net, mode):
            if mode == "full":
                return net
            out = [[None] * 4 for _ in range(4)]
            for i in range(4):
                row = [net[i][j] for j in range(4)]
                if mode == "end":
                    r = [row[3]] * 4
                elif mode == "start":
                    r = [row[0]] * 4
                else:
                    _, r1 = bez_split_col(row, mode[0])
                    if mode[1] >= 1.0:
                        r = r1
                    elif mode[0] < 1.0:
                        tt = (mode[1] - mode[0]) / (1.0 - mode[0])
                        sub, _ = bez_split_col(r1, tt)
                        r = sub
                    else:
                        r = r1
                for j in range(4):
                    out[i][j] = r[j]
            return out

        return restrict_rows(restrict_cols(self.Q, ru), rv)

    def enclose(self, ulo, uhi, vlo, vhi):
        net = self.restrict_net(ulo, uhi, vlo, vhi)
        pts = [net[i][j] for i in range(4) for j in range(4)]
        return hull_pts(pts)

    def enclose_du(self, ulo, uhi, vlo, vhi):
        net = self.restrict_net(ulo, uhi, vlo, vhi)
        hu = uhi - ulo
        vs = []
        for i in range(3):
            for j in range(4):
                p, q = net[i + 1][j], net[i][j]
                vs.append(((p[0] - q[0]) * 3.0 / hu,
                           (p[1] - q[1]) * 3.0 / hu,
                           (p[2] - q[2]) * 3.0 / hu))
        return hull_pts(vs)

    def enclose_dv(self, ulo, uhi, vlo, vhi):
        net = self.restrict_net(ulo, uhi, vlo, vhi)
        hv = vhi - vlo
        vs = []
        for i in range(4):
            for j in range(3):
                p, q = net[i][j + 1], net[i][j]
                vs.append(((p[0] - q[0]) * 3.0 / hv,
                           (p[1] - q[1]) * 3.0 / hv,
                           (p[2] - q[2]) * 3.0 / hv))
        return hull_pts(vs)


class HermiteSurface:
    """Emitted tensor-product bicubic Hermite over a uniform grid."""

    def __init__(self, exact, du, dv, u0, u1, v0, v1):
        self.u0, self.u1, self.v0, self.v1 = u0, u1, v0, v1
        self.nu, self.nv = 2 ** du, 2 ** dv
        self.us = [u0 + (u1 - u0) * k / self.nu for k in range(self.nu + 1)]
        self.vs = [v0 + (v1 - v0) * k / self.nv for k in range(self.nv + 1)]
        self.cells = []
        for iu in range(self.nu):
            row = []
            for iv in range(self.nv):
                Q = build_cell(exact, self.us[iu], self.us[iu + 1],
                               self.vs[iv], self.vs[iv + 1])
                row.append(SurfCell(self.us[iu], self.us[iu + 1],
                                    self.vs[iv], self.vs[iv + 1], Q))
            self.cells.append(row)

    def cell(self, iu, iv):
        return self.cells[iu][iv]

    def subs(self, u, v):
        iu = min(self.nu - 1, max(0, int((u - self.u0) / (self.u1 - self.u0) * self.nu)))
        iv = min(self.nv - 1, max(0, int((v - self.v0) / (self.v1 - self.v0) * self.nv)))
        return self.cells[iu][iv].eval(u, v)

    def range(self):
        return (self.u0, self.u1), (self.v0, self.v1)

    def der_mn(self, m, n, u, v):
        iu = min(self.nu - 1, max(0, int((u - self.u0) / (self.u1 - self.u0) * self.nu)))
        iv = min(self.nv - 1, max(0, int((v - self.v0) / (self.v1 - self.v0) * self.nv)))
        return self.cells[iu][iv].der_mn(m, n, u, v)

    def _cells_overlapping(self, uu, vv):
        """The curve module's cellOverlaps rule, per axis: a cell contributes
        when the interior overlaps, or when the query is a degenerate point on
        the cell boundary lying inside the cell."""
        out = []
        udeg = uu.lo == uu.hi
        vdeg = vv.lo == vv.hi
        for iu in range(self.nu):
            ulo = max(uu.lo, self.us[iu])
            uhi = min(uu.hi, self.us[iu + 1])
            if uhi < ulo:
                continue
            if uhi == ulo:
                # degenerate u-intersection: only when the query is that point
                if not (udeg and self.us[iu] <= uu.lo <= self.us[iu + 1]):
                    continue
            for iv in range(self.nv):
                vlo = max(vv.lo, self.vs[iv])
                vhi = min(vv.hi, self.vs[iv + 1])
                if vhi < vlo:
                    continue
                if vhi == vlo:
                    if not (vdeg and self.vs[iv] <= vv.lo <= self.vs[iv + 1]):
                        continue
                out.append((iu, iv, ulo, uhi, vlo, vhi))
        return out

    def enclose(self, uu, vv):
        acc = None
        for (iu, iv, lo_u, hi_u, lo_v, hi_v) in self._cells_overlapping(uu, vv):
            b = self.cells[iu][iv].enclose(lo_u, hi_u, lo_v, hi_v)
            acc = b if acc is None else acc.hull(b)
        if acc is None:
            return hull_pts([(0.0, 0.0, 0.0)])
        return acc

    def enclose_der(self, m, n, uu, vv):
        """Enclosure of the (m, n)-th derivative. Per-cell intersections whose
        width in an axis is below the width floor route through the direct
        evaluation path: the restricted-net derivative scaling divides by the
        intersection width, which explodes on ulp-wide slivers (a query box
        edge landing within ulps of a grid knot); the house hull pad absorbs
        the O(sliver) variation of the point evaluation."""
        if m + n == 0:
            return self.enclose(uu, vv)
        pts = []
        for (iu, iv, lo_u, hi_u, lo_v, hi_v) in self._cells_overlapping(uu, vv):
            cell = self.cells[iu][iv]
            u_sliver = (hi_u - lo_u) <= 8.0 * 2.0 ** -52 * max(abs(lo_u), abs(hi_u), 1.0)
            v_sliver = (hi_v - lo_v) <= 8.0 * 2.0 ** -52 * max(abs(lo_v), abs(hi_v), 1.0)
            if u_sliver and v_sliver:
                pts.append(cell.der_mn(m, n, 0.5 * (lo_u + hi_u), 0.5 * (lo_v + hi_v)))
            elif u_sliver:
                s = (0.5 * (lo_u + hi_u) - cell.a) / cell.hu
                cols = _u_der_column(cell.Q, m, s, cell.hu)
                sub = _restrict_curve(cols, (lo_v - cell.c) / cell.hv, (hi_v - cell.c) / cell.hv)
                pts.extend(_curve_der_points(sub, n, hi_v - lo_v))
            elif v_sliver:
                t = (0.5 * (lo_v + hi_v) - cell.c) / cell.hv
                rows = _u_der_column_t(cell.Q, n, t, cell.hv)
                sub = _restrict_curve(rows, (lo_u - cell.a) / cell.hu, (hi_u - cell.a) / cell.hu)
                pts.extend(_curve_der_points(sub, m, hi_u - lo_u))
            else:
                net = cell.restrict_net(lo_u, hi_u, lo_v, hi_v)
                pts.extend(_net_der_points(net, m, n, hi_u - lo_u, hi_v - lo_v))
        if not pts:
            return hull_pts([(0.0, 0.0, 0.0)])
        return hull_pts(pts)


def _net_der_points(net, m, n, hu, hv):
    """Derivative control points of a restricted 4x4 net: m forward differences
    along u (index i), n along v (index j), with Bernstein factorial scaling."""
    def fdiff_u(netp, times):
        cur = netp
        f = 1.0
        for k in range(times):
            f *= (3 - k)
            cur = [[tuple(cur[i + 1][j][d] - cur[i][j][d] for d in range(3))
                    for j in range(4)] for i in range(3 - k)]
        return cur, f

    def fdiff_v(netp, times):
        cur = netp
        f = 1.0
        for k in range(times):
            f *= (3 - k)
            cur = [[tuple(cur[i][j + 1][d] - cur[i][j][d] for d in range(3))
                    for j in range(3 - k)] for i in range(len(cur))]
        return cur, f

    netp, fu = fdiff_u(net, m)
    netp, fv = fdiff_v(netp, n)
    su = (fu / (hu ** m)) if m > 0 else 1.0
    sv = (fv / (hv ** n)) if n > 0 else 1.0
    return [tuple(p[d] * su * sv for d in range(3))
            for row in netp for p in row]


def _bern(deg, i, x):
    return math.comb(deg, i) * ((1 - x) ** (deg - i)) * (x ** i)


def _u_der_column(Q, m, s, hu):
    """The u-m-derivative control column at parameter s: one point per j,
    i.e. the control points (in v) of d^m/du^m at the u-line s."""
    if m == 0:
        return [tuple(sum(_bern(3, i, s) * Q[i][j][d] for i in range(4)) for d in range(3))
                for j in range(4)]
    f = 1.0
    for k in range(m):
        f *= (3 - k)
    diffs = [[tuple(sum((-1) ** (m - r) * math.comb(m, r) * Q[i + r][j][d] for r in range(m + 1)) for d in range(3))
              for j in range(4)] for i in range(4 - m)]
    deg = 3 - m
    return [tuple(f / (hu ** m) * sum(_bern(deg, i, s) * diffs[i][j][d] for i in range(4 - m)) for d in range(3))
            for j in range(4)]


def _u_der_column_t(Q, n, t, hv):
    """The v-n-derivative control row at parameter t: one point per i."""
    if n == 0:
        return [tuple(sum(_bern(3, j, t) * Q[i][j][d] for j in range(4)) for d in range(3))
                for i in range(4)]
    f = 1.0
    for k in range(n):
        f *= (3 - k)
    diffs = [[tuple(sum((-1) ** (n - r) * math.comb(n, r) * Q[i][j + r][d] for r in range(n + 1)) for d in range(3))
              for j in range(4 - n)] for i in range(4)]
    deg = 3 - n
    return [tuple(f / (hv ** n) * sum(_bern(deg, j, t) * diffs[i][j][d] for j in range(4 - n)) for d in range(3))
            for i in range(4)]


def _restrict_curve(col, t1, t2):
    """Restrict a 1D cubic control list to [t1, t2] (de Casteljau, the curve
    module's restrict logic: split at t1 keep right, split at t2 keep left)."""
    if t1 >= 1.0:
        return [col[3]] * 4
    if t2 <= 0.0:
        return [col[0]] * 4
    if t1 <= 0.0 and t2 >= 1.0:
        return list(col)
    _, right = bez_split_col(col, t1)
    if t2 >= 1.0:
        return right
    if t1 < 1.0:
        tt = (t2 - t1) / (1.0 - t1)
        sub, _ = bez_split_col(right, tt)
        return sub
    return right


def _curve_der_points(col, n, hv):
    """The n-th derivative control points of a 1D cubic over width hv."""
    if n == 0:
        return list(col)
    f = 1.0
    for k in range(n):
        f *= (3 - k)
    cur = col
    for _ in range(n):
        cur = [tuple(cur[i + 1][d] - cur[i][d] for d in range(3)) for i in range(len(cur) - 1)]
    return [tuple(p[d] * f / (hv ** n) for d in range(3)) for p in cur]

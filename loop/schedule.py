"""Which packets can run right now, and how many can run at once.

Two constraints, and the second is the one the wave table in
docs/KERNEL_AUTOBUILD_LOOP.md gets wrong: logical dependency (`needs`) is not
enough. Two packets can be logically independent and still collide on a file,
and a collision is only discovered at merge, after both workers have been paid
for. So the scheduler dispatches a packet only when its write set is disjoint
from every packet already in flight.

Usage:  python loop/schedule.py [--assume-done ID,ID]
"""
import json, sys, fnmatch, collections

ROWS = [json.loads(l) for l in open('loop/PACKETS.jsonl', encoding='utf-8') if l.strip()]
BY = {r['id']: r for r in ROWS}

def overlaps(a, b):
    """Do two write sets intersect? Globs are compared both ways so
    `truck-geometry/src/decorators/**` catches a specific file inside it."""
    for x in a:
        for y in b:
            if fnmatch.fnmatch(x, y) or fnmatch.fnmatch(y, x):
                return True
    return False

def frontier(done, running):
    # A packet marked DONE in PACKETS.jsonl is done, whether or not the caller
    # remembered to name it in --assume-done. Without this the ledger and the
    # scheduler disagree the moment anything lands: after BG-EVD-r3 merged, the
    # frontier read zero eligible packets because every W2 item still waited on
    # a dependency the file already recorded as satisfied. --assume-done stays
    # as the way to ask "what would open up if X landed", which is a question
    # about the future, not a way to state the present.
    done = set(done) | {r['id'] for r in ROWS if r['status'] == 'DONE'}
    out = []
    for r in ROWS:
        # BLOCKED is not "not yet reached": it is "reached, and found not to be
        # dispatchable as written". BG-ENC-004-OFFSET is the first one -- its
        # dependencies are all DONE, so a needs-only frontier lists it as
        # eligible forever, and the next session dispatches a packet that
        # cannot be written. A status the scheduler does not know is a status
        # that does not exist, so it has to be here and not only in `note`.
        if r['id'] in done or r['status'] in ('RUNNING', 'DONE', 'BLOCKED'):
            continue
        if any(n not in done for n in r['needs']):
            continue
        out.append(r)
    return out

def max_parallel(cands, running=()):
    """Greedy maximal set of mutually write-disjoint packets.

    Seeded with whatever is already in flight: a candidate that collides with a
    running worker is no more dispatchable than one that collides with another
    candidate, and forgetting that is how two workers end up editing the same
    file and discovering it at merge, after both have been paid for."""
    chosen = list(running)
    picked = []
    for r in cands:
        if not any(overlaps(r['writes'], c['writes']) for c in chosen):
            chosen.append(r)
            picked.append(r)
    return picked

if __name__ == '__main__':
    done = set()
    override = None
    args = sys.argv[1:]
    while args:
        flag, args = args[0], args[1:]
        if flag == '--assume-done' and args:
            done, args = set(args[0].split(',')), args[1:]
        elif flag == '--running' and args:
            override, args = set(args[0].split(',')), args[1:]
    if override is None:
        running = [r for r in ROWS if r['status'] == 'RUNNING']
    else:
        running = [r for r in ROWS if r['id'] in override]
    print(f"running: {[r['id'] for r in running]}")
    cands = frontier(done, running)
    par = max_parallel(cands, running)
    print(f"eligible now: {len(cands)}  dispatchable in parallel: {len(par)}")
    for r in par:
        print(f"  {r['id']:<28} {r['class']:<16} {r['wave']}")
    blocked = [r for r in cands if r not in par]
    if blocked:
        print("held back by write-set collision:")
        for r in blocked:
            hit = next(c['id'] for c in list(running) + par
                       if overlaps(r['writes'], c['writes']))
            print(f"  {r['id']:<28} collides with {hit}")
    print()
    w = collections.Counter(r['wave'] for r in ROWS)
    print("packets per wave:", dict(sorted(w.items())))
    print("by class:", dict(collections.Counter(r['class'] for r in ROWS)))

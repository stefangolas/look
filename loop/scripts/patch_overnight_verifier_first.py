"""Session-51, owner directive final form:
1. Verifier-first gate: no dispatch until the CC battery is green
   (rows_done). BIE rows sit in the shared registry and stay held.
2. Merge-on-save: try_land re-reads the registry before writing so rows
   appended concurrently (the side session's BIE registrations) are never
   clobbered by the driver's full-file rewrite.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()

# 1. crash fix
old_main = "def main():\n    log(f\"overnight driver start (pid {os.getpid()})\")"
assert old_main in src
src = src.replace(old_main,
    "def main():\n    global BATTERY_COOLDOWN_UNTIL\n"
    "    log(f\"overnight driver start (pid {os.getpid()})\")", 1)

# 2. verifier-first gate replaces the dispatch+old-battery tail
i = src.index("        # Rolling dispatch (owner direction, session 51)")
new_tail = '''        # Verifier-first gate (owner directive, session 51): no dispatch
        # until the CC program's battery is green. BIE rows in the shared
        # registry stay held by this gate.
        still_running = any(
            len(l.split()) >= 3 and l.split()[2] == "RUNNING"
            for l in status_out.splitlines()
            if l.startswith("slot "))
        rows_done_now = rows_done(rows)
        if not still_running and all_landed(rows) and not rows_done_now:
            if time.time() < BATTERY_COOLDOWN_UNTIL:
                log("battery cooldown active - no dispatch until green")
            else:
                battery(rows, order, reg_path)
                if rows_done(rows):
                    log("BATTERY GREEN - program complete; dispatch opens")
                else:
                    BATTERY_COOLDOWN_UNTIL = time.time() + 2 * 3600
                    log("battery not green - 2h cooldown, no dispatch until green")
        elif rows_done_now:
            disp = sh([sys.executable, str(ROOT / 'loop' / 'dispatch_ready.py'),
                       '--max-workers', '4'], timeout=1800)
            for ln in (disp.stdout or '').splitlines():
                if ln.strip():
                    log('dispatch: ' + ln.strip())
        else:
            log("landing/running phase - no dispatch")
        time.sleep(POLL_SECONDS)
    log("overnight driver exit")
'''
src = src[:i] + new_tail

# 3. merge-on-save inside try_land: re-read before writing
old_save = '''def save_registry(rows, order, p):
    with open(p, "w", encoding="utf-8", newline="\\n") as f:'''
assert old_save in src
src = src.replace(old_save, '''def save_registry(rows, order, p):
    # merge-on-save (session 51): rows appended concurrently (the side
    # session's registrations) must survive the driver's rewrite.
    for line in p.read_text(encoding="utf-8-sig").splitlines():
        if line.strip():
            r = json.loads(line)
            if r["id"] not in rows:
                rows[r["id"]] = r
                order.append(r["id"])
    with open(p, "w", encoding="utf-8", newline="\\n") as f:''', 1)

open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("verifier-first + merge-on-save applied + parses OK")

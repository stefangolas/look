"""Session-51 recovery patch for the overnight driver (run once, keep for the record).

1. all_landed: the PowerShell -replace patch silently no-op'd (no-match is not
   an error), so the driver ran the ORIGINAL KV2-only check - vacuously true
   now that every KV2 row is landed - and fired the battery mid-program.
2. Non-exiting recovery: battery-not-green enters a 2h cooldown and the loop
   keeps cycling (landing + dispatching) instead of exiting.
"""
import ast

src = open("loop/overnight.py", encoding="utf-8").read()

# --- 1. all_landed ---
i = src.index("def all_landed")
j = src.index("def battery")
new_al = '''def all_landed(rows):
    prog = [r for r in rows.values()
            if r["id"].startswith(("BG-KV2-", "CC-"))]
    if not prog:
        return False  # vacuous truth fired the premature battery (session 51)
    return all(LANDED_RE.search((r.get("note") or "").lower())
               for r in prog)


'''
src = src[:i] + new_al + src[j:]

# --- 2. rows_done helper ---
src = src.replace(
    "def battery(rows, order, reg_path):",
    '''def rows_done(rows):
    return all(r.get("status") == "DONE"
               for r in rows.values()
               if r["id"].startswith(("BG-KV2-", "CC-")))


def battery(rows, order, reg_path):''',
    1,
)

# --- 3. cooldown + keep-cycling on battery-not-green ---
src = src.replace("POLL_SECONDS = 300",
                  "POLL_SECONDS = 300\nBATTERY_COOLDOWN_UNTIL = 0.0", 1)
src = src.replace(
    '''            if not still_running:
                battery(rows, order, reg_path)
                battery_done = True
                break''',
    '''            if not still_running:
                if time.time() < BATTERY_COOLDOWN_UNTIL:
                    log("battery cooldown active - cycling")
                elif rows_done(rows) and all_landed(rows):
                    battery(rows, order, reg_path)
                    if rows_done(rows):
                        battery_done = True
                        break
                    BATTERY_COOLDOWN_UNTIL = time.time() + 2 * 3600
                    log("battery not green - 2h cooldown, the loop keeps cycling")
                else:
                    log("program rows not all DONE yet - keep cycling")''',
    1,
)

open("loop/overnight.py", "w", encoding="utf-8", newline="\n").write(src)
ast.parse(src)
print("patched + parses OK")

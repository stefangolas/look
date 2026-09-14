"""LOOP-MACH-1 acceptance suite (items 1-7, plus the shadow equivalence).

Plain unittest, runnable as `python -m pytest loop/tests_loop_mach1.py -q`.
These tests are the gate for a harness packet: there are no cargo gates and no
anchors, so they exercise the machinery directly -- a temp SQLite state file
for the claim/adjudication/outcome items and a throwaway git repo for the
write-set and transactional-landing items.

    claim_is_atomic                 two concurrent claims, exactly one wins
    adjudicating_blocks_dispatch    FINISHED/ADJUDICATING block, LANDED releases
    attempt_evidence_immutable      attempt 0002 never rewrites 0001's file
    writeset_violation_rejected     undeclared path aborts before any ref moves
    transactional_landing_crash_safe  a gate failure leaves the ref unmoved
    outcome_separation              COMPLETE+SPEC_GAP+findings vs COMPLETE+LANDED
    ci_gate_exec_bit                a 100644 scripts/*.sh fails locally
    shadow_equivalence              JSONL and SQLite dispatch decisions agree
"""

import json
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import check_writeset  # noqa: E402
import ci_gate  # noqa: E402
import dispatch_ready  # noqa: E402


def git(repo, *args, check=True):
    res = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True,
        encoding="utf-8", errors="replace")
    if check and res.returncode != 0:
        raise AssertionError(
            f"git {' '.join(args)} failed in {repo}:\n{res.stderr}")
    return res


def make_repo(tmp):
    repo = Path(tmp) / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "loop@localhost")
    git(repo, "config", "user.name", "loop")
    git(repo, "config", "commit.gpgsign", "false")
    (repo / "base.txt").write_text("base\n", encoding="utf-8")
    git(repo, "add", "base.txt")
    git(repo, "commit", "-qm", "base")
    git(repo, "branch", "integration/kernel-bg")
    return repo


def write_packet(repo, packet_id, write_allow):
    d = repo / "loop" / "packets"
    d.mkdir(parents=True, exist_ok=True)
    path = d / f"{packet_id}.md"
    body = ["# PACKET", "```yaml", f"id: {packet_id}", "write_allow:"]
    body += [f"  - {entry}" for entry in write_allow]
    body += ["```", ""]
    path.write_text("\n".join(body), encoding="utf-8")
    return path


class StateTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="loop-mach1-state-")
        self.db = str(Path(self._tmp.name) / "state.sqlite")
        self.conn = dispatch_ready.connect_state(self.db)
        dispatch_ready.init_state(self.conn)

    def tearDown(self):
        self.conn.close()
        self._tmp.cleanup()

    # 1 -------------------------------------------------------------------
    def test_claim_is_atomic(self):
        aid = dispatch_ready.mint_attempt(self.conn, "PKT", "base000", slot=1)
        self.conn.close()
        outcomes = []
        barrier = threading.Barrier(2)

        def claim():
            conn = dispatch_ready.connect_state(self.db)
            try:
                barrier.wait(timeout=10)
                outcomes.append(
                    dispatch_ready.claim_attempt(conn, aid, 1, "claimer"))
            finally:
                conn.close()

        threads = [threading.Thread(target=claim) for _ in range(2)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=30)
        self.assertEqual(outcomes.count(True), 1)
        self.assertEqual(outcomes.count(False), 1)

        self.conn = dispatch_ready.connect_state(self.db)
        row = dispatch_ready.latest_attempt(self.conn, "PKT")
        self.assertEqual(row["state"], "CLAIMED")
        self.assertIn(row["lease_owner"], ("claimer",))

    # 2 -------------------------------------------------------------------
    def test_adjudicating_blocks_dispatch(self):
        aid = dispatch_ready.mint_attempt(self.conn, "PKT", "base000", slot=1)
        self.assertTrue(
            dispatch_ready.claim_attempt(self.conn, aid, 1, "worker"))
        self.assertTrue(dispatch_ready.finish_attempt(
            self.conn, aid, execution_status="COMPLETE", outcome="SPEC_GAP"))
        self.assertEqual(
            dispatch_ready.latest_attempt(self.conn, "PKT")["state"],
            "FINISHED")

        # A finished attempt is awaiting adjudication: a competing dispatch
        # must not mint a second attempt.
        self.assertIsNone(dispatch_ready.claim_for_dispatch(
            self.conn, "PKT", "base000", 2, "other"))

        # The landing side claims adjudication atomically; ADJUDICATING still
        # blocks the scheduler.
        self.assertTrue(dispatch_ready.claim_adjudication(self.conn, aid))
        self.assertEqual(
            dispatch_ready.latest_attempt(self.conn, "PKT")["state"],
            "ADJUDICATING")
        self.assertIsNone(dispatch_ready.claim_for_dispatch(
            self.conn, "PKT", "base000", 2, "other"))

        # Landing releases the active lock; the packet is terminal, not stuck.
        self.assertTrue(dispatch_ready.land_attempt(self.conn, aid))
        self.assertEqual(
            dispatch_ready.latest_attempt(self.conn, "PKT")["state"],
            "LANDED")
        self.assertIsNone(dispatch_ready.active_attempt(self.conn, "PKT"))
        self.assertFalse(dispatch_ready.claim_adjudication(self.conn, aid))

    # 3 -------------------------------------------------------------------
    def test_attempt_evidence_immutable(self):
        root = Path(self._tmp.name)
        first = dispatch_ready.file_result(
            root, "PKT", 1,
            dispatch_ready.build_result("PKT", "PKT/0001", "COMPLETE", "LANDED"))
        second = dispatch_ready.file_result(
            root, "PKT", 2,
            dispatch_ready.build_result("PKT", "PKT/0002", "COMPLETE",
                                        "SPEC_GAP", findings=["nope"]))

        self.assertEqual(
            json.loads(first.read_text(encoding="utf-8"))["attempt_id"],
            "PKT/0001")
        # Reusing attempt 1's slot is refused, so attempt 2 cannot clobber it.
        with self.assertRaises(FileExistsError):
            dispatch_ready.file_result(
                root, "PKT", 1,
                dispatch_ready.build_result("PKT", "PKT/0001", "COMPLETE",
                                            "SUPERSEDED"))
        self.assertEqual(
            json.loads(first.read_text(encoding="utf-8"))["outcome"], "LANDED")
        self.assertTrue(second.is_file())
        self.assertEqual(
            json.loads(second.read_text(encoding="utf-8"))["attempt_id"],
            "PKT/0002")

    # 6 -------------------------------------------------------------------
    def test_outcome_separation(self):
        gap = dispatch_ready.build_result(
            "FHC-G10", "FHC-G10/0002", "COMPLETE", "SPEC_GAP",
            findings=["covariance law is not exact over the landed cell"],
            blocked_by_capability=["exact_unplaced_patch_recovery"])
        self.assertEqual(gap["execution_status"], "COMPLETE")
        self.assertEqual(gap["outcome"], "SPEC_GAP")
        self.assertEqual(len(gap["findings"]), 1)
        self.assertEqual(gap["blocked_by_capability"],
                         ["exact_unplaced_patch_recovery"])

        landed = dispatch_ready.build_result(
            "PKT", "PKT/0001", "COMPLETE", "LANDED")
        self.assertEqual(landed["execution_status"], "COMPLETE")
        self.assertEqual(landed["outcome"], "LANDED")

        # The two vocabularies are disjoint: "DONE" is neither a state.
        with self.assertRaises(ValueError):
            dispatch_ready.build_result("PKT", "PKT/1", "DONE", "LANDED")
        with self.assertRaises(ValueError):
            dispatch_ready.build_result("PKT", "PKT/1", "COMPLETE", "DONE")

    # 8 (the shadow-mode equivalence the Done-when requires) ---------------
    def test_shadow_equivalence(self):
        rows = [
            {"id": "A", "status": "DONE", "needs": [], "writes": ["x"]},
            {"id": "B", "status": "READY", "needs": ["A"], "writes": ["y"]},
            {"id": "C", "status": "READY", "needs": [], "writes": ["x"]},
            {"id": "D", "status": "BLOCKED", "needs": [], "writes": ["z"]},
            {"id": "E", "status": "READY", "needs": [], "writes": ["w"],
             "note": "landed abc1234"},
        ]
        report = dispatch_ready.shadow_compare(
            rows, assigned=set(), dead=set(), running_writes={"x"},
            db_path=str(Path(self._tmp.name) / "shadow.sqlite"))
        self.assertTrue(report["equal"],
                        f"JSONL {report['jsonl']} != SQLite {report['sqlite']}")
        self.assertEqual(report["jsonl"], ["B"])


class LandingTestCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="loop-mach1-git-")
        self.repo = make_repo(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def _branch_with(self, packet_id, write_allow, changed):
        packet = write_packet(self.repo, packet_id, write_allow)
        git(self.repo, "checkout", "-q", "integration/kernel-bg")
        git(self.repo, "add", packet.relative_to(self.repo).as_posix())
        git(self.repo, "commit", "-qm", f"packet {packet_id}")
        git(self.repo, "checkout", "-q", "-b", f"packet/{packet_id}",
            "integration/kernel-bg")
        for name, content in changed.items():
            (self.repo / name).write_text(content, encoding="utf-8")
            git(self.repo, "add", name)
        git(self.repo, "commit", "-qm", "attempt")
        return packet

    def _integration_sha(self):
        return git(self.repo, "rev-parse",
                   "integration/kernel-bg").stdout.strip()

    # 4 -------------------------------------------------------------------
    def test_writeset_violation_rejected(self):
        packet = self._branch_with(
            "PKT", ["allowed.txt"],
            {"allowed.txt": "ok\n", "undeclared.txt": "sneaky\n"})
        before = self._integration_sha()
        result = check_writeset.transactional_land(
            self.repo, packet, "packet/PKT", "integration/kernel-bg",
            gate=lambda sha, wt: True)
        self.assertEqual(result["status"], "WRITESET_VIOLATION")
        self.assertIn("undeclared.txt", result["violations"])
        self.assertEqual(before, self._integration_sha())
        # No temp worktree survived the aborted landing.
        worktrees = git(self.repo, "worktree", "list", "--porcelain").stdout
        self.assertEqual(worktrees.count("worktree "), 1)

    # 5 -------------------------------------------------------------------
    def test_transactional_landing_crash_safe(self):
        packet = self._branch_with("PKT", ["allowed.txt"],
                                   {"allowed.txt": "ok\n"})
        before = self._integration_sha()

        failed = check_writeset.transactional_land(
            self.repo, packet, "packet/PKT", "integration/kernel-bg",
            gate=lambda sha, wt: False)
        self.assertEqual(failed["status"], "GATE_FAILED")
        self.assertEqual(before, self._integration_sha())
        self.assertEqual(
            git(self.repo, "status", "--porcelain").stdout.strip(), "")
        worktrees = git(self.repo, "worktree", "list", "--porcelain").stdout
        self.assertEqual(worktrees.count("worktree "), 1)

        landed = check_writeset.transactional_land(
            self.repo, packet, "packet/PKT", "integration/kernel-bg",
            gate=lambda sha, wt: True)
        self.assertEqual(landed["status"], "LANDED")
        self.assertNotEqual(before, self._integration_sha())
        self.assertTrue(git(self.repo, "merge-base", "--is-ancestor",
                            "packet/PKT", "integration/kernel-bg",
                            check=False).returncode == 0)

    # 2 (the ledger/evidence half of immutable attempt identities) ----------
    def test_ledger_carries_attempt_id(self):
        packet = self._branch_with("PKT", ["allowed.txt"],
                                   {"allowed.txt": "ok\n"})
        landed = check_writeset.transactional_land(
            self.repo, packet, "packet/PKT", "integration/kernel-bg",
            gate=lambda sha, wt: True)
        self.assertEqual(landed["status"], "LANDED")

        result = dispatch_ready.build_result("PKT", "PKT/0002", "COMPLETE",
                                             "LANDED")
        dest = check_writeset.record_metadata(
            self.repo, "PKT", "PKT/0002", result,
            merged_sha=landed["merged_sha"])
        self.assertTrue(dest.is_file())
        row = json.loads(
            (self.repo / "loop" / "LEDGER.jsonl")
            .read_text(encoding="utf-8").splitlines()[-1])
        self.assertEqual(row["attempt_id"], "PKT/0002")
        self.assertEqual(row["outcome"], "LANDED")
        with self.assertRaises(FileExistsError):
            check_writeset.record_metadata(self.repo, "PKT", "PKT/0002", result)

        check_writeset.annotate_registry(
            self.repo, "PKT", "PKT/0002", merged_sha=landed["merged_sha"])
        reg = json.loads(
            (self.repo / "loop" / "PACKETS.jsonl")
            .read_text(encoding="utf-8").splitlines()[-1])
        self.assertEqual(reg["attempt_id"], "PKT/0002")
        self.assertIn("landed", reg["note"])


class CIGateTestCase(unittest.TestCase):
    # 7 -------------------------------------------------------------------
    def test_ci_gate_exec_bit(self):
        with tempfile.TemporaryDirectory(prefix="loop-mach1-ci-") as tmp:
            repo = make_repo(tmp)
            (repo / "scripts").mkdir()
            (repo / "scripts" / "bad.sh").write_text(
                "#!/bin/sh\necho hi\n", encoding="utf-8")
            git(repo, "add", "scripts/bad.sh")
            git(repo, "commit", "-qm", "add a non-executable script")

            violations = ci_gate.exec_bit_violations(repo)
            self.assertIn(("scripts/bad.sh", "100644"), violations)

            git(repo, "update-index", "--chmod=+x", "scripts/bad.sh")
            git(repo, "commit", "-qm", "make the script executable")
            self.assertEqual(ci_gate.exec_bit_violations(repo), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)

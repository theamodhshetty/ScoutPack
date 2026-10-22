import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location("smoke_codex", Path(__file__).parents[1] / "smoke-codex.py")
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


class TranscriptTests(unittest.TestCase):
    def setUp(self):
        self.expected = {"head": "a" * 40, "marker": "COMMITTED_fixture", "dirty_marker": "DIRTY_fixture"}
        context = {"packet": "Content commit: `" + self.expected["head"] +
                   "`\n```file:after.ts:1-1\nCOMMITTED_fixture\n```",
                   "budget": 2500, "estimated_tokens": 100}
        changes = {"recent_changes": {"head_commit": self.expected["head"], "files_changed": 1,
                   "changes": [{"path": "after.ts", "previous_path": "before.ts", "status": "renamed",
                                "additions": 0, "deletions": 0}]}}
        self.events = [
            {"type": "item.completed", "item": {"type": "mcp_tool_call", "server": "scoutpack",
             "tool": "context", "status": "completed", "arguments": {"branch": True, "task": smoke.TASK, "budget": 2500},
             "error": None, "result": {"structured_content": context}}},
            {"type": "item.completed", "item": {"type": "mcp_tool_call", "server": "scoutpack",
             "tool": "recent_changes", "status": "completed", "arguments": {"branch": True},
             "error": None, "result": {"structured_content": changes}}},
            {"type": "item.completed", "item": {"type": "agent_message",
             "text": f"COMMITTED_fixture before.ts after.ts {self.expected['head']}"}},
            {"type": "turn.completed", "usage": {"input_tokens": 500, "output_tokens": 30}},
        ]

    def validate(self, events=None):
        return smoke.validate_events(self.events if events is None else events, self.expected)

    def test_complete_tool_evidence_passes(self):
        self.assertEqual(self.validate()["packet_estimated_tokens"], 100)

    def test_agent_claim_and_successful_turn_without_tools_fail(self):
        with self.assertRaisesRegex(ValueError, "Missing completed turn or successful"):
            self.validate(self.events[2:])

    def test_approval_failure_does_not_pass_with_a_completed_turn(self):
        self.events[0]["item"].update(status="failed", result=None, error={"message": "approval policy is never"})
        with self.assertRaisesRegex(ValueError, "did not succeed"):
            self.validate()

    def test_server_error_result_is_rejected(self):
        for key in ("is_error", "isError"):
            events = copy.deepcopy(self.events)
            events[0]["item"]["result"][key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate(events)

    def test_wrong_scope_server_or_budget_is_rejected(self):
        for key, value in [("arguments", {"branch": False}), ("server", "another_server"),
                           ("arguments", {"branch": True, "task": smoke.TASK, "budget": 999})]:
            events = copy.deepcopy(self.events)
            events[0]["item"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                self.validate(events)

    def test_text_only_results_do_not_substitute_for_structured_evidence(self):
        self.events[0]["item"]["result"] = {"content": [{"type": "text", "text": "looks good"}]}
        with self.assertRaisesRegex(ValueError, "Missing structured"):
            self.validate()

    def test_dirty_or_stale_context_is_rejected(self):
        for packet in ["DIRTY_fixture", "COMMITTED_fixture from an old commit"]:
            self.events[0]["item"]["result"]["structured_content"]["packet"] = packet
            with self.subTest(packet=packet), self.assertRaises(ValueError):
                self.validate()

    def test_incorrect_rename_is_rejected(self):
        self.events[1]["item"]["result"]["structured_content"]["recent_changes"]["changes"][0]["previous_path"] = "wrong.ts"
        with self.assertRaisesRegex(ValueError, "rename relationship"):
            self.validate()

    def test_incomplete_turn_or_incorrect_final_answer_is_rejected(self):
        with self.assertRaises(ValueError):
            self.validate(self.events[:-1])
        self.events[2]["item"]["text"] = "The tool worked."
        with self.assertRaisesRegex(ValueError, "final answer"):
            self.validate()

    def test_shell_fallback_or_client_error_is_rejected(self):
        for event in [{"type": "item.started", "item": {"type": "command_execution"}}, {"type": "error"}]:
            with self.subTest(event=event), self.assertRaises(ValueError):
                self.validate([event] + self.events)

    def test_changed_client_schema_or_repeated_calls_fail_closed(self):
        for events in [[None], [{"item": None}], [self.events[0]] + self.events]:
            with self.subTest(events=events), self.assertRaises(ValueError):
                self.validate(events)

    def test_invalid_estimated_budgets_are_rejected(self):
        for estimate in [True, 0, -1, 2501, "100"]:
            events = copy.deepcopy(self.events)
            events[0]["item"]["result"]["structured_content"]["estimated_tokens"] = estimate
            with self.subTest(estimate=estimate), self.assertRaises(ValueError):
                self.validate(events)

    def test_fixture_has_different_committed_and_dirty_content(self):
        with tempfile.TemporaryDirectory() as directory:
            root, expected = smoke.make_fixture(Path(directory))
            text = (root / "after.ts").read_text()
            self.assertIn(expected["dirty_marker"], text)
            self.assertNotIn(expected["marker"], text)
            self.assertFalse((root / ".scoutpack").exists())


if __name__ == "__main__":
    unittest.main()

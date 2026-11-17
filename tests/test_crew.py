import re
from agent03.crew import run_crew, AGENTS, parse_json, cap


def env(plan_reply):
    def ai(_m, inp):
        role = inp["messages"][0]["content"].split(".")[0]
        if "PLANNER" in role:
            return {"response": plan_reply}
        if "RESEARCHER" in role:
            return {"response": "- note on " + inp["messages"][1]["content"]}
        return {"response": f"FINAL: {len(re.findall('## ', inp['messages'][1]['content']))} sections"}
    return ai


def test_planner_researchers_writer():
    r = run_crew(env('["Q1?", "Q2?"]'), "Compare A and B")
    assert r["plan"] == ["Q1?", "Q2?"] and r["answer"] == "FINAL: 2 sections"
    assert [h["from"] for h in r["handoffs"]] == ["user", "planner", "researcher#1", "researcher#2", "writer"]


def test_bad_plan_falls_back():
    assert run_crew(env("not json"), "Explain X")["plan"] == ["Explain X"]


def mk(plan, calls=None):
    def ai(_m, inp):
        sys = inp["messages"][0]["content"]
        if calls is not None:
            calls.append({"sys": sys, "max": inp["max_tokens"]})
        if sys == AGENTS["planner"]:
            return {"response": plan}
        if sys == AGENTS["researcher"]:
            return {"response": "- " + inp["messages"][1]["content"]}
        return {"response": "written"}
    return ai


def test_plan_capped_non_strings_dropped():
    assert run_crew(mk('["a","b",3,"c","d"]'), "t")["plan"] == ["a", "b", "c"]


def test_plan_wrapped_in_prose():
    assert run_crew(mk('Here you go: ["x?","y?"] thanks'), "t")["plan"] == ["x?", "y?"]


def test_empty_array_falls_back():
    assert run_crew(mk("[]"), "Do it")["plan"] == ["Do it"]


def test_one_researcher_per_question_and_budgets():
    calls = []
    run_crew(mk('["a","b"]', calls), "t")
    assert len([c for c in calls if c["sys"] == AGENTS["researcher"]]) == 2
    assert [c["max"] for c in calls] == [200, 300, 300, 400]


def test_writer_receives_task_and_notes():
    seen = []
    base = mk('["a","b"]')
    def ai(m, inp):
        if inp["messages"][0]["content"] == AGENTS["writer"]:
            seen.append(inp["messages"][1]["content"])
        return base(m, inp)
    run_crew(ai, "TASK")
    assert "Task: TASK" in seen[0] and "## a\n- a" in seen[0] and "## b\n- b" in seen[0]


def test_handoff_timings_non_decreasing():
    t = iter(range(0, 1000, 5))
    r = run_crew(mk('["a"]'), "t", now=lambda: next(t))
    ms = [h["ms"] for h in r["handoffs"]]
    assert ms == sorted(ms) and r["total_ms"] >= ms[-1]


def test_parse_json_and_cap():
    assert parse_json("nope", "fb") == "fb" and parse_json('x {"a":1} y') == {"a": 1}
    assert cap("hi") and not cap("   ") and not cap("x" * 501) and not cap(5)

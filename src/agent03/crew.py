"""Three specialised agents (planner, researcher, writer); an orchestrator passes work between them."""
import json, re, time

MODEL = "@cf/meta/llama-3.3-70b-instruct-fp8-fast"
AGENTS = {
    "planner": "You are the PLANNER. Break the user's task into 2-3 focused research questions. Reply ONLY with a JSON array of strings.",
    "researcher": "You are the RESEARCHER. Answer the research question with concise, factual bullet points (max 5). Note any uncertainty.",
    "writer": "You are the WRITER. Using only the research notes, write a clear, well-structured final answer to the original task in under 200 words.",
}


def text(r) -> str:
    v = r.get("response") if isinstance(r, dict) else None
    if isinstance(v, str):
        return v
    return json.dumps(v if v is not None else r, separators=(",", ":"), ensure_ascii=False)


def parse_json(s, fallback=None):
    m = re.search(r"[\[{][\s\S]*[\]}]", str(s))
    try:
        return json.loads(m.group(0)) if m else fallback
    except ValueError:
        return fallback


def cap(s, n: int = 500) -> bool:
    return isinstance(s, str) and bool(s.strip()) and len(s) <= n


def run_crew(ai, task: str, now=lambda: time.time() * 1000) -> dict:
    """ai(model, input) -> dict (Workers AI shape). Injected so tests run offline."""
    t0 = now()
    handoffs = []
    log = lambda frm, to, content: handoffs.append({"from": frm, "to": to, "content": content, "ms": round(now() - t0)})
    ask = lambda system, user, mx: text(ai(MODEL, {"messages": [{"role": "system", "content": system}, {"role": "user", "content": user}], "max_tokens": mx}))
    log("user", "planner", task)
    plan = parse_json(ask(AGENTS["planner"], task, 200), None)
    if not isinstance(plan, list) or not plan:
        plan = [task]
    plan = [q for q in plan if isinstance(q, str)][:3]
    log("planner", "researcher", plan)
    notes = [ask(AGENTS["researcher"], q, 300) for q in plan]
    for i, n in enumerate(notes):
        log(f"researcher#{i + 1}", "writer", n)
    body = "\n\n".join(f"## {q}\n{notes[i]}" for i, q in enumerate(plan))
    draft = ask(AGENTS["writer"], f"Task: {task}\n\nResearch notes:\n{body}", 400)
    log("writer", "user", draft)
    return {"answer": draft, "plan": plan, "handoffs": handoffs, "total_ms": round(now() - t0)}

//! Three specialised agents (planner, researcher, writer); an orchestrator passes work between them.
use regex::Regex;
use serde_json::{json, Value};

pub const MODEL: &str = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";
pub const PLANNER: &str = "You are the PLANNER. Break the user's task into 2-3 focused research questions. Reply ONLY with a JSON array of strings.";
pub const RESEARCHER: &str = "You are the RESEARCHER. Answer the research question with concise, factual bullet points (max 5). Note any uncertainty.";
pub const WRITER: &str = "You are the WRITER. Using only the research notes, write a clear, well-structured final answer to the original task in under 200 words.";

/// Injected Workers AI: run(model, input) -> response JSON.
#[allow(async_fn_in_trait)]
pub trait Ai {
    async fn run(&self, model: &str, input: Value) -> Result<Value, String>;
}

pub fn text(r: &Value) -> String {
    match r.get("response") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => r.to_string(),
        Some(v) => v.to_string(),
    }
}

pub fn parse_json(s: &str) -> Option<Value> {
    let re = Regex::new(r"[\[{][\s\S]*[\]}]").unwrap();
    re.find(s).and_then(|m| serde_json::from_str(m.as_str()).ok())
}

pub fn cap(s: Option<&str>, n: usize) -> bool {
    matches!(s, Some(s) if !s.trim().is_empty() && s.chars().count() <= n)
}

async fn ask<A: Ai>(ai: &A, system: &str, user: &str, max: u32) -> Result<String, String> {
    let r = ai.run(MODEL, json!({"messages":[{"role":"system","content":system},{"role":"user","content":user}],"max_tokens":max})).await?;
    Ok(text(&r))
}

/// `now` returns milliseconds (Date.now in the Worker). Researchers run in parallel.
pub async fn run_crew<A: Ai>(ai: &A, task: &str, now: &dyn Fn() -> f64) -> Result<Value, String> {
    let t0 = now();
    let mut handoffs: Vec<Value> = vec![];
    let mut log = |from: String, to: &str, content: Value, handoffs: &mut Vec<Value>| {
        handoffs.push(json!({"from": from, "to": to, "content": content, "ms": (now() - t0).round() as i64}));
    };
    log("user".into(), "planner", json!(task), &mut handoffs);
    let mut plan = match parse_json(&ask(ai, PLANNER, task, 200).await?) {
        Some(Value::Array(a)) if !a.is_empty() => a,
        _ => vec![json!(task)],
    };
    plan = plan.into_iter().filter(|q| q.is_string()).take(3).collect();
    log("planner".into(), "researcher", Value::Array(plan.clone()), &mut handoffs);
    let qs: Vec<String> = plan.iter().map(|q| q.as_str().unwrap().to_string()).collect();
    let notes: Vec<String> = futures::future::join_all(qs.iter().map(|q| ask(ai, RESEARCHER, q, 300))).await.into_iter().collect::<Result<_, _>>()?;
    for (i, n) in notes.iter().enumerate() {
        log(format!("researcher#{}", i + 1), "writer", json!(n), &mut handoffs);
    }
    let body: Vec<String> = qs.iter().zip(&notes).map(|(q, n)| format!("## {q}\n{n}")).collect();
    let draft = ask(ai, WRITER, &format!("Task: {task}\n\nResearch notes:\n{}", body.join("\n\n")), 400).await?;
    log("writer".into(), "user", json!(draft), &mut handoffs);
    Ok(json!({"answer": draft, "plan": plan, "handoffs": handoffs, "total_ms": (now() - t0).round() as i64}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use std::cell::RefCell;

    struct Mock {
        plan: String,
        calls: RefCell<Vec<(String, u64)>>,
        writer_in: RefCell<Vec<String>>,
    }
    impl Ai for Mock {
        async fn run(&self, _m: &str, inp: Value) -> Result<Value, String> {
            let sys = inp["messages"][0]["content"].as_str().unwrap().to_string();
            let user = inp["messages"][1]["content"].as_str().unwrap().to_string();
            self.calls.borrow_mut().push((sys.clone(), inp["max_tokens"].as_u64().unwrap()));
            Ok(if sys == PLANNER {
                json!({"response": self.plan})
            } else if sys == RESEARCHER {
                json!({"response": format!("- {user}")})
            } else {
                self.writer_in.borrow_mut().push(user.clone());
                json!({"response": format!("FINAL: {} sections", user.matches("## ").count())})
            })
        }
    }
    fn mk(plan: &str) -> Mock {
        Mock { plan: plan.into(), calls: RefCell::new(vec![]), writer_in: RefCell::new(vec![]) }
    }
    fn clock() -> impl Fn() -> f64 {
        let c = std::cell::Cell::new(0.0);
        move || { c.set(c.get() + 5.0); c.get() }
    }

    #[test]
    fn planner_researchers_writer() {
        let r = block_on(run_crew(&mk("[\"Q1?\", \"Q2?\"]"), "Compare A and B", &clock())).unwrap();
        assert_eq!(r["plan"], json!(["Q1?", "Q2?"]));
        assert_eq!(r["answer"], "FINAL: 2 sections");
        let from: Vec<&str> = r["handoffs"].as_array().unwrap().iter().map(|h| h["from"].as_str().unwrap()).collect();
        assert_eq!(from, ["user", "planner", "researcher#1", "researcher#2", "writer"]);
    }
    #[test]
    fn plan_fallbacks_and_caps() {
        assert_eq!(block_on(run_crew(&mk("not json"), "Explain X", &clock())).unwrap()["plan"], json!(["Explain X"]));
        assert_eq!(block_on(run_crew(&mk("[]"), "Do it", &clock())).unwrap()["plan"], json!(["Do it"]));
        assert_eq!(block_on(run_crew(&mk("[\"a\",\"b\",3,\"c\",\"d\"]"), "t", &clock())).unwrap()["plan"], json!(["a", "b", "c"]));
        assert_eq!(block_on(run_crew(&mk("Here you go: [\"x?\",\"y?\"] thanks"), "t", &clock())).unwrap()["plan"], json!(["x?", "y?"]));
    }
    #[test]
    fn budgets_and_writer_input() {
        let m = mk("[\"a\",\"b\"]");
        block_on(run_crew(&m, "TASK", &clock())).unwrap();
        let c = m.calls.borrow();
        assert_eq!(c.iter().filter(|x| x.0 == RESEARCHER).count(), 2);
        assert_eq!(c.iter().map(|x| x.1).collect::<Vec<_>>(), [200, 300, 300, 400]);
        let w = &m.writer_in.borrow()[0];
        assert!(w.contains("Task: TASK") && w.contains("## a\n- a") && w.contains("## b\n- b"));
    }
    #[test]
    fn timings_non_decreasing() {
        let r = block_on(run_crew(&mk("[\"a\"]"), "t", &clock())).unwrap();
        let ms: Vec<i64> = r["handoffs"].as_array().unwrap().iter().map(|h| h["ms"].as_i64().unwrap()).collect();
        assert!(ms.windows(2).all(|w| w[0] <= w[1]));
        assert!(r["total_ms"].as_i64().unwrap() >= *ms.last().unwrap());
    }
    #[test]
    fn helpers() {
        assert_eq!(parse_json("nope"), None);
        assert_eq!(parse_json("x {\"a\":1} y"), Some(json!({"a":1})));
        assert!(cap(Some("hi"), 500) && !cap(Some("   "), 500) && !cap(Some(&"x".repeat(501)), 500) && !cap(None, 500));
    }
}

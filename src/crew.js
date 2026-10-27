import { ask, parseJSON } from "./core.js";

// Three specialised agents with their own instructions; an orchestrator passes work between them.
export const AGENTS = {
  planner: "You are the PLANNER. Break the user's task into 2-3 focused research questions. Reply ONLY with a JSON array of strings.",
  researcher: "You are the RESEARCHER. Answer the research question with concise, factual bullet points (max 5). Note any uncertainty.",
  writer: "You are the WRITER. Using only the research notes, write a clear, well-structured final answer to the original task in under 200 words.",
};

export async function runCrew(env, task) {
  const t0 = Date.now(), handoffs = [];
  const log = (from, to, content) => handoffs.push({ from, to, content, ms: Date.now() - t0 });
  log("user", "planner", task);
  let plan = parseJSON(await ask(env, AGENTS.planner, task, { max_tokens: 200 }), null);
  if (!Array.isArray(plan) || !plan.length) plan = [task];
  plan = plan.filter((q) => typeof q === "string").slice(0, 3);
  log("planner", "researcher", plan);
  // Researchers run in parallel, one per sub-question.
  const notes = await Promise.all(plan.map((q) => ask(env, AGENTS.researcher, q, { max_tokens: 300 })));
  notes.forEach((n, i) => log(`researcher#${i + 1}`, "writer", n));
  const draft = await ask(env, AGENTS.writer, `Task: ${task}\n\nResearch notes:\n${plan.map((q, i) => `## ${q}\n${notes[i]}`).join("\n\n")}`, { max_tokens: 400 });
  log("writer", "user", draft);
  return { answer: draft, plan, handoffs, total_ms: Date.now() - t0 };
}

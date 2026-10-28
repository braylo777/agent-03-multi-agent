import { test } from "node:test";
import assert from "node:assert/strict";
import { runCrew, AGENTS } from "../src/crew.js";
import { parseJSON, cap } from "../src/core.js";

const mk = (plan, calls = []) => ({ AI: { run: async (_m, inp) => {
  const sys = inp.messages[0].content; calls.push({ sys, max: inp.max_tokens });
  if (sys === AGENTS.planner) return { response: plan };
  if (sys === AGENTS.researcher) return { response: "- " + inp.messages[1].content };
  return { response: "written" };
} } });
test("plan capped at 3 questions, non-strings dropped", async () => {
  const r = await runCrew(mk('["a","b",3,"c","d"]'), "t"); assert.deepEqual(r.plan, ["a", "b", "c"]);
});
test("plan JSON wrapped in prose is still parsed", async () => {
  const r = await runCrew(mk('Here you go: ["x?","y?"] thanks'), "t"); assert.deepEqual(r.plan, ["x?", "y?"]);
});
test("empty array plan falls back to the task", async () => {
  const r = await runCrew(mk("[]"), "Do it"); assert.deepEqual(r.plan, ["Do it"]);
});
test("one researcher call per sub-question, token budgets respected", async () => {
  const calls = []; await runCrew(mk('["a","b"]', calls), "t");
  assert.equal(calls.filter((c) => c.sys === AGENTS.researcher).length, 2);
  assert.deepEqual(calls.map((c) => c.max), [200, 300, 300, 400]);
});
test("writer receives task and every note", async () => {
  let w; const env = mk('["a","b"]'); const run = env.AI.run;
  env.AI.run = async (m, inp) => { if (inp.messages[0].content === AGENTS.writer) w = inp.messages[1].content; return run(m, inp); };
  await runCrew(env, "TASK"); assert.match(w, /Task: TASK/); assert.match(w, /## a\n- a/); assert.match(w, /## b\n- b/);
});
test("handoffs carry non-decreasing timings and total_ms", async () => {
  const r = await runCrew(mk('["a"]'), "t");
  r.handoffs.reduce((p, h) => (assert.ok(h.ms >= p), h.ms), 0); assert.ok(r.total_ms >= r.handoffs.at(-1).ms);
});
test("parseJSON and cap helpers", () => {
  assert.equal(parseJSON("nope", "fb"), "fb"); assert.deepEqual(parseJSON('x {"a":1} y'), { a: 1 });
  assert.ok(cap("hi")); assert.ok(!cap("   ")); assert.ok(!cap("x".repeat(501))); assert.ok(!cap(5));
});

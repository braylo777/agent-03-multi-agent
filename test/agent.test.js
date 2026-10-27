import { test } from "node:test";
import assert from "node:assert/strict";
import { runCrew } from "../src/crew.js";

const env = (planReply) => ({ AI: { run: async (_m, inp) => {
  const role = inp.messages[0].content.split(".")[0];
  if (role.includes("PLANNER")) return { response: planReply };
  if (role.includes("RESEARCHER")) return { response: "- note on " + inp.messages[1].content };
  return { response: "FINAL: " + (inp.messages[1].content.match(/## /g) || []).length + " sections" };
} } });
test("planner -> parallel researchers -> writer", async () => {
  const r = await runCrew(env('["Q1?", "Q2?"]'), "Compare A and B");
  assert.deepEqual(r.plan, ["Q1?", "Q2?"]); assert.equal(r.answer, "FINAL: 2 sections");
  assert.deepEqual(r.handoffs.map((h) => h.from), ["user", "planner", "researcher#1", "researcher#2", "writer"]);
});
test("bad plan falls back to single research step", async () => {
  const r = await runCrew(env("not json"), "Explain X"); assert.deepEqual(r.plan, ["Explain X"]);
});

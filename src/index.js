import { json, limited, body, cap } from "./core.js";
import { runCrew } from "./crew.js";
import page from "./page.html";

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/api/run") {
      if (await limited(req, env)) return json({ error: "Rate limit: try again in a minute." }, 429);
      const { task } = await body(req);
      if (!cap(task)) return json({ error: "task required (<=500 chars)" }, 400);
      try { return json(await runCrew(env, task)); } catch (e) { return json({ error: "Agent error: " + e.message }, 502); }
    }
    return new Response(page, { headers: { "content-type": "text/html;charset=utf-8" } });
  },
};

# agent-03-multi-agent

A **multi-agent system** where specialised agents (planner, researchers and writer) collaborate on one task, with every handoff visible. It runs serverless on Cloudflare Workers with the free Workers AI tier. Project 3 of a 10-part agentic AI portfolio.

**Live demo:** https://agent-03-multi-agent.braylo777.workers.dev

## What it is
Give it a task. The planner splits it into 2-3 research questions, a researcher agent answers each one in parallel, and the writer synthesises the final answer. The response includes the plan, each agent's output, and per-handoff timing.

## How it works
```
task ─▶ PLANNER ─▶ [q1, q2, q3]
                    ├─▶ RESEARCHER#1 ─┐
                    ├─▶ RESEARCHER#2 ─┼─▶ WRITER ─▶ final answer
                    └─▶ RESEARCHER#3 ─┘
```
- `src/crew.js`: agent roles (system prompts) and the orchestrator. Researchers run concurrently with `Promise.all`.
- Robust handoffs: if the planner returns invalid JSON, the orchestrator falls back to a single research step.
- Guardrails: 5 requests/min per IP, 500-char cap, at most 3 sub-questions.

## Run it
```bash
npm install
npm test            # offline unit tests (Node 20+), LLM is mocked
npx wrangler dev    # local; Workers AI runs via your Cloudflare login
npx wrangler deploy # free tier, no paid API keys
```

## Stack
Cloudflare Workers · Workers AI (`@cf/meta/llama-3.3-70b-instruct-fp8-fast`) · vanilla JS · node:test

## License
MIT

# Project Console

In Studio this is called the **Project Console** — toggled from the shell,
titled "Console" — and it drives
`POST /api/projects/{owner}/{project}/pipelines/dsl`, the same DSL endpoint
`register`/`activate`/`execute`/`describe` commands go through from a script
or agent.

Use it as the fast path for:

- inspecting runtime output
- running DSL commands
- checking agent work
- quick operational debugging

It should be treated as a primary operational surface, not an afterthought.

Use it when you need the shortest route from question to execution.

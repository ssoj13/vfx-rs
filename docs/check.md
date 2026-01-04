# Bug Hunt: Это приложение - это новый Rust Crate который имплементит функциональность OpenImageIO и OpenColorIO в _ref (там референсный C++ код).
Я делаю новый Rust only crate который соединил бы в себе эту функциональность и поддержал бы стандарт на платформе Rust.
Исследуй проект, посмотри что сделано корректно и что возможно неверно, что ещё нужно доделать. Проверь всё на корректность и логику, на правильность.


## Prereqs: 
  - Answer in Russian in chat, write English code and .md files.
  - MANDATORY: Use filesystem MCP to work with files, memory MCP to remember, log things and create relations and github MCP or "gh" tool if needed. 
  - Use sub-agents and work in parallel.

## Workflow:
  - Check the app, try to spot some illogical places, errors, mistakes, unused and dead code and such things.
  - Check interface compatibility, all FIXME, TODO, all unfinished code - try to understand what to do with it, offer suggestions.
  - Find unused code and try to figure out why it was created. I think you haven't finished the big refactoring and lost pieces by the way.
  - Check possibilities for code deduplication and single source of ground truth about entities, items and logic in app.
  - Unify the architecture, dataflows, codepaths, deduplicate everything, simplify but keeping the logic and functionality! Do not remove features!
  - Avoid of creation of special function with long stupid names in favor of arguments: just add the optional argument to existing function if needed.
  - Do not guess, you have to be sure and produce production-grade decisions and problem solutions. Consult context7 MCP use fetch MCP to search internet.
  - Create a comprehensive dataflow for human and for yourself to help you understand the logic.
  - Do not try to simplify things or take shortcuts or remove functionality, we need just the best practices: fast, compact and elegant, powerful code.
  - If you feel task is complex - ask questions, then just split it into sub-tasks, create a plan and follow it updating that plan on each step (setting checkboxes on what's done).
  - Don't be lazy and do not assume things, do not guess code. You need to be SURE, since you're writing a production code. Do not simplify things unless it will significantly improve the code logic.
  - Discard any compatibility issues, we don't need it.
  - Create comprehensive report so you could "survive" after context compactification, re-read it and continue without losing details. Offer pro-grade solutions.


## Outputs:
  - At the end create a professional comprehensive report and update plan and write it to planN.md where N is the next available number, and wait for approval! 
  - Also create a detailed AGENTS.md with dataflow and codepath diagrams.
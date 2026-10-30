#!/usr/bin/env python3
"""Generate docs/generated/AGENT-TOOL-MATRIX.md from manifests + DSH reference."""
import pathlib, re
root = pathlib.Path(__file__).resolve().parent.parent
rows = [
 ("bash","tool.bash","zeokeel/tool-bash","implemented","approval + background"),
 ("bash-persistent","tool.bash_persistent","zeokeel/tool-bash-persistent","implemented","same session cwd/env"),
 ("pwsh","tool.pwsh","zeokeel/tool-pwsh","implemented","platform gated"),
 ("pwsh-persistent","tool.pwsh_persistent","zeokeel/tool-pwsh-persistent","unsupported","explicit platform error"),
 ("fs read/write/edit/read_image","tool.fs/tool.fs-write/tool.fs-edit/tool.read_image","zeokeel/tool-fs*","implemented","read_image data URL"),
 ("fs-search glob/grep","tool.glob/tool.grep","zeokeel/tool-glob/tool-grep","implemented",""),
 ("jobs output/list/kill","tool.jobs.*","zeokeel/tool-job-*","implemented","in-process registry"),
 ("terminal","tool.terminal","zeokeel/tool-terminal","partial","single session per conversation"),
 ("todo","tool.todo","zeokeel/tool-todo","implemented",""),
 ("ask-user","tool.ask_user","zeokeel/tool-ask-user","implemented","UI question card + IPC"),
 ("present","tool.present","zeokeel/tool-present","implemented",""),
 ("web fetch","tool.web.fetch","zeokeel/tool-web-fetch","implemented","SSRF guarded"),
 ("structured extraction","tool.extract","zeokeel/tool-extract","implemented","rig extractor runner"),
 ("web search","tool.web_search","zeokeel/tool-web-search","implemented","DuckDuckGo/HTTPS provider env"),
 ("skill","tool.skill","zeokeel/tool-skill","implemented","read-only"),
 ("goal","tool.goal","zeokeel/tool-goal","partial","no round driver"),
 ("schedule","tool.schedule","zeokeel/tool-schedule","implemented","desktop scheduler executes due intervals"),
 ("session-query","tool.session_query","zeokeel/tool-session-query","implemented","search/read"),
 ("subagent","tool.subagent","zeokeel/tool-subagent","partial","synchronous; no async control"),
 ("subagent_fork","tool.subagent_fork","zeokeel/tool-subagent-fork","implemented","forks current conversation context"),
 ("subagent-control","tool.subagent_control","zeokeel/tool-subagent-control","partial","real child registry list; send/interrupt unavailable"),
 ("workflow","tool.workflow","zeokeel/tool-workflow","implemented","linear + DAG topological order"),
 ("ralph","tool.ralph","zeokeel/tool-ralph","partial","bounded loop; no fresh context isolation"),
 ("working-directory","tool.working_directory","zeokeel/tool-working-directory","implemented",""),
 ("workspace-dependencies","tool.workspace_dependencies","zeokeel/tool-workspace-dependencies","implemented",""),
 ("read document (PDF/EPUB)","tool.read_document","zeokeel/tool-read-document","implemented","PDF/EPUB positive + negative tests"),
 ("worktree","tool.worktree","zeokeel/tool-worktree","implemented","argv, safe path"),
 ("vector store","vector.store","zeokeel/cap-vector","implemented","rig embeddings + in-process cosine; configure ZEOKEEL_EMBEDDING_*"),
 ("image generation","media.image","zeokeel/cap-image","implemented","rig image model; configure ZEOKEEL_IMAGE_*; enterprise gateway unsupported"),
 ("audio generation","media.audio","zeokeel/cap-audio","implemented","rig audio model; configure ZEOKEEL_AUDIO_*; enterprise gateway unsupported"),
 ("transcription","media.transcription","zeokeel/cap-transcription","implemented","rig transcription model; configure ZEOKEEL_TRANSCRIPTION_*"),
 ("rerank","media.rerank","zeokeel/cap-rerank","implemented","rig rerank (voyageai); configure ZEOKEEL_RERANK_*"),
 ("rig hooks","agent.hooks","zeokeel/cap-hooks","implemented","HookEvent lifecycle; configure ZEOKEEL_AGENT_HOOKS_ENABLED"),
 ("call-timeout","-","agent-rig","implemented (exemption)","ZEOKEEL_TOOL_TIMEOUT_MS; agent-level policy, no separate package per C3 exemption"),
 ("tool-cordis","tool.cordis","zeokeel/tool-cordis","disabled","C2 forbids runtime plugin loading"),
 ("agent-team","tool.agent_team","zeokeel/tool-agent-team","experimental","explicit unsupported error"),
]
out = ["# Agent tool matrix (@generated)", "", "DSH reference: `research/dsh-agent-inventory.md` §2. Status is honest: implemented / partial / unsupported / unimplemented / disabled / experimental.", "", "| DSH module | ZeoKeel role | Package | Status | Notes |", "| --- | --- | --- | --- | --- |"]
for row in rows: out.append("| " + " | ".join(row) + " |")
matrix = "\n".join(out) + "\n"
path = root / "docs/generated/AGENT-TOOL-MATRIX.md"
if "--check" in __import__("sys").argv:
    current = path.read_text() if path.is_file() else ""
    if current != matrix:
        print("agent tool matrix out of date"); raise SystemExit(1)
    print("agent tool matrix up to date")
else:
    path.write_text(matrix); print(f"wrote {path}")

/** Module: conversation — thread, tool rows, plan, approval, six states. */
import type { ApprovalView, MessageView, PlanStep, ToolView } from "../types";
import { Icon } from "./kit";

function ToolRow({ tool, expanded, onToggle }: { tool: ToolView; expanded: boolean; onToggle: () => void }) {
  return (
    <div className="rounded-sm border border-border/60">
      <button type="button" onClick={onToggle} aria-expanded={expanded} className="flex w-full items-center gap-2 px-2 py-[5px] text-start font-mono text-[11px] hover:bg-foreground/[0.03]">
        <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${tool.status === "running" ? "bg-live" : tool.status === "error" ? "bg-destructive" : "bg-border"}`} />
        <span className="w-[72px] shrink-0 text-foreground">{tool.name}</span>
        <span className="min-w-0 flex-1 truncate text-muted-foreground">{tool.detail}</span>
        <span className="shrink-0 text-muted-foreground">{tool.time}</span>
        <span className={`text-muted-foreground transition-transform ${expanded ? "rotate-180" : ""}`}><Icon name="chevron" size={10} /></span>
      </button>
      {expanded ? (
        <div className="border-t border-border/60 px-2 py-1.5 font-mono text-[11px]">
          <p className="text-muted-foreground">args</p><pre className="whitespace-pre-wrap text-foreground">{tool.args}</pre>
          <p className="mt-1 text-muted-foreground">result</p><pre className="max-h-24 overflow-auto whitespace-pre-wrap text-foreground">{tool.result || "…"}</pre>
        </div>
      ) : null}
    </div>
  );
}

function PlanCard({ steps }: { steps: PlanStep[] }) {
  return (
    <div className="rounded-document border border-border bg-background px-2.5 py-2">
      <div className="flex items-center gap-2 text-[11px]"><span className="font-medium text-foreground">计划</span><span className="text-muted-foreground">{steps.filter((s) => s.status === "done").length}/{steps.length}</span><span className="ms-auto flex items-center gap-1 text-[10px] text-muted-foreground"><Icon name="spark" size={11} />自动继续</span></div>
      <div className="mt-1.5 grid gap-1 text-[11px]">
        {steps.map((step) => (
          <div key={step.id} className="flex items-center gap-1.5">
            {step.status === "done" ? <Icon name="check" size={11} /> : step.status === "running" ? <span className="h-1.5 w-1.5 rounded-capsule bg-live" /> : <span className="h-2.5 w-2.5 rounded-capsule border border-border" />}
            <span className={step.status === "pending" ? "text-muted-foreground" : "text-foreground"}>{step.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ApprovalCard({ approval, onDecision }: { approval: ApprovalView; onDecision: (d: "approved" | "rejected") => void }) {
  return (
    <div role="group" aria-label="需要审批" className="rounded-document bg-muted px-2.5 py-2">
      <div className="flex items-center gap-2"><strong className="text-[12px] font-medium">需要审批</strong><span className="ms-auto text-[11px] text-muted-foreground">{approval.decision === "pending" ? "等待决定" : approval.decision === "approved" ? "已允许" : "已拒绝"}</span></div>
      <p className="mt-1 font-mono text-[11px]">{approval.tool}</p>
      <pre className="mt-0.5 whitespace-pre-wrap font-mono text-[10px] text-muted-foreground">{approval.args}</pre>
      <p className="mt-1 text-[11px] text-muted-foreground">{approval.reason}</p>
      {approval.decision === "pending" ? (
        <div className="mt-2 flex gap-1.5">
          <button type="button" onClick={() => onDecision("approved")} className="rounded-control bg-foreground px-2.5 py-1 text-[11px] font-medium text-background">允许一次</button>
          <button type="button" onClick={() => onDecision("rejected")} className="rounded-control border border-border px-2.5 py-1 text-[11px]">拒绝</button>
        </div>
      ) : null}
    </div>
  );
}

export interface ConversationProps {
  messages: MessageView[];
  expandedTools: Record<string, boolean>;
  onToggleTool: (id: string) => void;
  onDecision: (id: string, d: "approved" | "rejected") => void;
  onSuggestion: (text: string) => void;
}

export function Conversation(props: ConversationProps) {
  if (props.messages.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center px-6 text-center">
        <img src="brand/logo.svg" alt="" className="h-12 w-12 object-contain" />
        <h2 className="mt-5 text-[24px] font-medium tracking-[-0.02em]">今天想完成什么？</h2>
        <p className="mt-2 text-[12px] text-muted-foreground">所有对话都在当前工作区 <span className="font-mono">~</span> 内进行。</p>
        <div className="mt-5 grid w-full max-w-[620px] gap-2 sm:grid-cols-3">
          {["读取文件并总结", "起草一段文字", "解释一个概念"].map((title, i) => (
            <button key={title} type="button" onClick={() => props.onSuggestion(title)} className="rounded-document border border-border bg-background px-3 py-2 text-start text-[12px] hover:bg-muted">
              <span className="block font-medium text-foreground">{title}</span>
              <span className="mt-0.5 block text-[11px] text-muted-foreground">{["读取 demo-data/hello.txt 并总结", "帮我起草一段项目进展说明", "用简单的话解释 Rust 所有权"][i]}</span>
            </button>
          ))}
        </div>
      </div>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto flex w-full max-w-[760px] flex-col gap-4 px-5 py-4 text-[13px] leading-[1.55]">
        {props.messages.map((message) => (
          <div key={message.id} className="flex flex-col gap-1.5">
            {message.role === "user" ? (
              <div className="flex justify-end"><div className="max-w-[78%] rounded-thread bg-muted px-3 py-1.5">{message.text}</div></div>
            ) : (
              <>
                <div className="flex items-baseline gap-2">
                  <span className="text-[11px] font-semibold">ZeoKeel</span>
                  {message.status === "streaming" ? <span className="text-[11px] font-medium text-foreground">生成中…</span> : null}
                  {message.status === "cancelled" ? <span className="text-[10px] text-muted-foreground">已取消</span> : null}
                  {message.status === "error" ? <span className="text-[10px] text-destructive">错误</span> : null}
                </div>
                {message.tools.length > 0 ? (
                  <div className="grid gap-1">
                    {message.tools.map((tool) => (
                      <ToolRow key={tool.id} tool={tool} expanded={props.expandedTools[tool.id] === true} onToggle={() => props.onToggleTool(tool.id)} />
                    ))}
                  </div>
                ) : null}
                {message.plan !== undefined ? <PlanCard steps={message.plan} /> : null}
                <p className="whitespace-pre-wrap">{message.text}{message.status === "streaming" ? <span className="ms-0.5 inline-block h-3.5 w-[2px] translate-y-0.5 animate-pulse bg-live" aria-hidden="true" /> : null}</p>
                {message.approval !== undefined ? <ApprovalCard approval={message.approval} onDecision={(d) => props.onDecision(message.approval?.id ?? "", d)} /> : null}
              </>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

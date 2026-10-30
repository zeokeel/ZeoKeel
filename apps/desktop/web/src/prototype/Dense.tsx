/**
 * High-density design prototype (v2). Static mock data; uses design tokens,
 * official logo, and ui-kit primitives. Screenshots only — no backend.
 */
import { useEffect } from "react";

import { BRAND } from "../generated/brand";

type IconName = "search" | "plus" | "sliders" | "panel" | "command" | "attach" | "send" | "stop" | "folder" | "file" | "diff" | "terminal" | "chevron" | "more" | "spark" | "branch" | "check" | "close" | "arrow";

function Icon({ name, size = 14 }: { name: IconName; size?: number }) {
  const p: Record<IconName, string> = {
    search: "M7 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm4-1 3 3",
    plus: "M8 3v10M3 8h10",
    sliders: "M3 5h10M3 11h10M6 3v4M10 9v4",
    panel: "M3 3h10v10H3zM9 3v10",
    command: "M6 3a2 2 0 1 0 0 4h4a2 2 0 1 0 0-4 2 2 0 0 0-2 2v6a2 2 0 1 0 2-2H6a2 2 0 1 0 0 4 2 2 0 0 0 2-2V5",
    attach: "M10.5 4.5 5 10a3 3 0 0 0 4 4l5-5a4.5 4.5 0 0 0-6.5-6.5L2.5 8",
    send: "M3 13 14 8 3 3l2 5-2 5Z",
    stop: "M4 4h8v8H4z",
    folder: "M2 4.5c0-.6.5-1 1-1h3l1 1.2h5c.6 0 1 .4 1 1v6.8c0 .6-.4 1-1 1H3c-.5 0-1-.4-1-1V4.5Z",
    file: "M4 2h5l3 3v9H4zM9 2v3h3",
    diff: "M4 5h8M4 11h8M6 2v3M10 9v3",
    terminal: "M3 4l4 4-4 4M8 12h5",
    chevron: "M4 6l4 4 4-4",
    more: "M3 8h.01M8 8h.01M13 8h.01",
    spark: "M8 2l1.3 4.7L14 8l-4.7 1.3L8 14l-1.3-4.7L2 8l4.7-1.3L8 2Z",
    branch: "M5 3v10M5 6c0 3 3 3 6 3v4",
    check: "M3 8l3 3 6-6",
    close: "M4 4l8 8M12 4l-8 8",
    arrow: "M3 8h10M9 4l4 4-4 4",
  };
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round">
      <path d={p[name]} />
    </svg>
  );
}

const SESSIONS = [
  { title: "重构工作区导航", time: "刚刚", group: "今天", live: true, dot: true },
  { title: "解释 Rust 所有权", time: "12:04", group: "今天" },
  { title: "把发布说明写短一点", time: "11:20", group: "今天" },
  { title: "排查 CI 缓存失效", time: "昨天", group: "昨天" },
  { title: "设计审批卡交互", time: "昨天", group: "昨天" },
  { title: "整理 MCP 连接器清单", time: "周一", group: "更早" },
];
const TOOLS = [
  { name: "fs.read", detail: "src/components/Sidebar.tsx", time: "0.4s", status: "done" },
  { name: "grep", detail: "workspace switcher", time: "0.2s", status: "done" },
  { name: "fs.read", detail: "packages/ui/ui-sessions/src/index.tsx", time: "0.3s", status: "done" },
  { name: "fs.write", detail: "prototype/Dense.tsx", time: "1.1s", status: "running" },
];
const FILES = [
  { name: "src/", kind: "dir", depth: 0 },
  { name: "components/", kind: "dir", depth: 1 },
  { name: "App.tsx", kind: "file", depth: 2, mod: true },
  { name: "Sidebar.tsx", kind: "file", depth: 2, mod: true },
  { name: "prototype/", kind: "dir", depth: 1 },
  { name: "Dense.tsx", kind: "file", depth: 2, add: true },
  { name: "workspace.ts", kind: "file", depth: 1, mod: true },
  { name: "docs/", kind: "dir", depth: 0 },
  { name: "27-ui-design-system.md", kind: "file", depth: 1 },
];
const COMMANDS = [
  { title: "新建会话", hint: "⌘N", key: "new" },
  { title: "切换工作区", hint: "⌘⇧W", key: "workspace" },
  { title: "切换主题：浅色 / 深色", hint: "⌘⇧L", key: "theme" },
  { title: "打开设置", hint: "⌘,", key: "settings" },
  { title: "查看键盘快捷键", hint: "?", key: "keys" },
];

function Logo() {
  return <img src={BRAND.logoLight} alt="" className="h-6 w-6 rounded-document object-contain" />;
}

function WorkspaceRow() {
  return (
    <button type="button" className="flex w-full items-center gap-2 rounded-control px-2 py-1.5 text-start hover:bg-foreground/[0.04]">
      <span className="flex h-6 w-6 items-center justify-center rounded-document bg-muted text-muted-foreground"><Icon name="folder" /></span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12px] font-medium text-foreground">zeokeel</span>
        <span className="block truncate text-[11px] text-muted-foreground">~/code/zeokeel</span>
      </span>
      <Icon name="chevron" size={12} />
    </button>
  );
}

function LeftRail() {
  let lastGroup = "";
  return (
    <aside className="flex w-[248px] shrink-0 flex-col border-e border-border bg-sidebar" aria-label="会话列表">
      <div className="flex items-center justify-between px-3 pt-3 pb-2">
        <div className="flex items-center gap-2"><Logo /><span className="text-[12px] font-semibold">ZeoKeel</span></div>
        <div className="flex items-center gap-0.5 text-muted-foreground">
          <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="搜索"><Icon name="search" size={13} /></button>
          <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="新建会话"><Icon name="plus" size={13} /></button>
        </div>
      </div>
      <div className="px-2"><div className="mb-2"><WorkspaceRow /></div>
        <div className="flex items-center gap-1.5 rounded-control border border-border bg-background/60 px-2 py-1 text-muted-foreground">
          <Icon name="search" size={12} /><span className="text-[11px]">搜索会话或 @文件</span><span className="ms-auto font-mono text-[10px]">⌘K</span>
        </div>
      </div>
      <nav className="mt-2 min-h-0 flex-1 overflow-y-auto px-2 pb-2" aria-label="会话">
        {SESSIONS.map((s) => {
          const showGroup = s.group !== lastGroup; lastGroup = s.group;
          return (
            <div key={s.title}>
              {showGroup ? <p className="mb-1 mt-2 px-1.5 text-[10px] font-medium uppercase tracking-[0.08em] text-muted-foreground/80">{s.group}</p> : null}
              <button type="button" className={`group flex w-full items-center gap-1.5 rounded-sm px-1.5 py-[5px] text-start text-[12px] ${s.title === "重构工作区导航" ? "bg-muted font-medium text-foreground shadow-[inset_2px_0_0_var(--zk-blue-500)]" : "text-muted-foreground hover:bg-foreground/[0.035] hover:text-foreground"}`}>
                <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${s.live ? "bg-live" : "bg-border"}`} aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">{s.title}</span>
                <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground/70">{s.time}</span>
              </button>
            </div>
          );
        })}
      </nav>
      <div className="flex items-center gap-1 border-t border-border px-2 py-1.5 text-muted-foreground">
        <button className="flex h-6 flex-1 items-center gap-1.5 rounded-sm px-1.5 text-[11px] hover:bg-foreground/[0.04]"><Icon name="sliders" size={12} />设置</button>
        <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.04]" aria-label="命令面板"><Icon name="command" size={13} /></button>
      </div>
    </aside>
  );
}

function Header() {
  return (
    <header className="flex h-11 shrink-0 items-center gap-2 border-b border-border px-3">
      <div className="min-w-0 flex-1">
        <h1 className="truncate text-[12px] font-semibold tracking-[-0.005em]">重构工作区导航</h1>
      </div>
      <div className="flex items-center gap-1.5 text-muted-foreground">
        <span className="hidden items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px] xl:flex"><Icon name="branch" size={11} />main</span>
        <span className="flex items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px]"><span className="h-1.5 w-1.5 rounded-capsule bg-live" />DeepSeek V3</span>
        <span className="hidden items-center gap-1 font-mono text-[10px] lg:flex">42% · 18.2k</span>
        <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="任务"><span className="relative"><Icon name="terminal" size={13} /><span className="absolute -right-1 -top-1 h-2 w-2 rounded-capsule bg-live ring-2 ring-background" /></span></button>
        <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="切换右侧栏"><Icon name="panel" size={13} /></button>
        <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="更多"><Icon name="more" size={13} /></button>
      </div>
    </header>
  );
}

function ToolRow({ t }: { t: (typeof TOOLS)[number] }) {
  return (
    <div className="flex items-center gap-2 rounded-sm px-1 py-1 font-mono text-[11px] text-muted-foreground hover:bg-foreground/[0.035]">
      <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${t.status === "running" ? "bg-live" : "bg-border"}`} />
      <span className="w-[76px] shrink-0 text-foreground">{t.name}</span>
      <span className="min-w-0 flex-1 truncate">{t.detail}</span>
      <span className="shrink-0 tabular-nums text-muted-foreground/70">{t.time}</span>
    </div>
  );
}

function Thread() {
  return (
    <div className="mx-auto flex w-full max-w-[760px] flex-col gap-4 px-5 py-4 text-[13px] leading-[1.6]">
      <div className="flex justify-end"><div className="max-w-[78%] rounded-thread bg-muted px-3 py-1.5 text-[13px]">帮我把工作区导航重构一下，顺便看看 DSH 是怎么做的。</div></div>
      <div className="flex flex-col gap-2">
        <div className="flex items-baseline gap-2"><span className="text-[11px] font-semibold text-foreground">ZeoKeel</span><span className="text-[10px] text-muted-foreground">已运行 4 个工具 · 12.4s</span></div>
        <p>我先检查当前结构，再给出更紧凑的三栏布局：左侧工作区/会话、中间对话、右侧文件与变更。</p>
        <div className="rounded-document border border-border/70 bg-card/60 px-2 py-1">
          {TOOLS.map((t) => <ToolRow key={t.name + t.detail} t={t} />)}
        </div>
        <div className="rounded-document border border-border bg-background px-2.5 py-2">
          <div className="flex items-center gap-2 text-[11px]"><span className="font-medium text-foreground">计划</span><span className="text-muted-foreground">3/5</span><span className="ms-auto flex items-center gap-1 text-[10px] text-muted-foreground"><Icon name="spark" size={11} />自动继续</span></div>
          <div className="mt-1.5 grid gap-1 text-[11px] text-muted-foreground">
            <div className="flex items-center gap-1.5"><Icon name="check" size={11} /><span className="text-foreground">读取现有 Sidebar/布局</span></div>
            <div className="flex items-center gap-1.5"><Icon name="check" size={11} /><span className="text-foreground">对比 DSH 三栏信息架构</span></div>
            <div className="flex items-center gap-1.5"><Icon name="check" size={11} /><span className="text-foreground">写高密度原型</span></div>
            <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-capsule border border-border" /><span>收敛间距/字号 token</span></div>
            <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-capsule border border-border" /><span>截图评审</span></div>
          </div>
        </div>
        <p>关键变化：把基础字号降到 13px，行高 1.55；侧栏 248px、右侧栏 288px；工具调用折叠为单行，计划/审批用紧凑卡；状态栏承载 workspace、分支、模型、上下文占用。</p>
      </div>
      <div className="flex justify-end"><div className="max-w-[78%] rounded-thread bg-muted px-3 py-1.5 text-[13px]">右侧文件/变更面板要能折叠，别一直占地方。</div></div>
      <div className="flex flex-col gap-1.5">
        <div className="flex items-baseline gap-2"><span className="text-[11px] font-semibold">ZeoKeel</span><span className="text-[10px] text-live">生成中…</span></div>
        <p>可以，默认展开文件，变更/终端按需切换；窄窗自动折叠成抽屉。<span className="ms-0.5 inline-block h-3.5 w-[2px] translate-y-0.5 animate-pulse bg-live" aria-hidden="true" /></p>
      </div>
    </div>
  );
}

function Composer() {
  return (
    <div className="mx-auto w-full max-w-[760px] px-5 pb-2">
      <div className="rounded-thread border border-border bg-background px-2.5 pt-1.5 pb-1.5 focus-within:border-foreground/35">
        <div className="flex items-center gap-2 text-[13px] text-muted-foreground">
          <button className="flex items-center gap-1 rounded-control px-1.5 py-1 text-[11px] hover:bg-foreground/[0.04]"><span className="font-mono">/</span> 命令 <Icon name="chevron" size={10} /></button>
          <span className="min-w-0 flex-1 text-foreground">右侧面板默认展开，但可以一键收起…</span>
          <span className="font-mono text-[10px] tabular-nums text-muted-foreground">3.1k / 32k</span>
        </div>
        <div className="mt-1 flex items-center gap-1 text-muted-foreground">
          <button className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]" aria-label="附件"><Icon name="attach" size={13} /></button>
          <button className="flex items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px] hover:bg-foreground/[0.04]"><Icon name="spark" size={11} />DeepSeek V3 <Icon name="chevron" size={10} /></button>
          <span className="ms-auto text-[10px] text-muted-foreground/80">Enter 发送 · Shift+Enter 换行</span>
          <button className="flex h-6 items-center gap-1 rounded-control bg-foreground px-2 text-[11px] font-medium text-background"><Icon name="send" size={11} />发送</button>
        </div>
      </div>
    </div>
  );
}

function StatusBar() {
  return (
    <div className="flex h-6 shrink-0 items-center gap-2 border-t border-border px-3 text-[10px] text-muted-foreground">
      <span className="flex items-center gap-1"><span className="h-1.5 w-1.5 rounded-capsule bg-live" />已连接本地 serve</span>
      <span className="text-border">|</span>
      <span className="font-mono">~/code/zeokeel</span>
      <span className="text-border">|</span>
      <span>9 packages</span>
      <span className="ms-auto flex items-center gap-2 font-mono">
        <span>⌘K 命令</span><span className="text-border">|</span><span>v0.1.0</span>
      </span>
    </div>
  );
}

function RightRail() {
  return (
    <aside className="hidden w-[288px] shrink-0 flex-col border-s border-border bg-sidebar/60 lg:flex" aria-label="文件与变更">
      <div className="flex h-9 items-center gap-0.5 border-b border-border px-2 text-[11px]">
        <button className="rounded-sm bg-muted px-2 py-1 font-medium text-foreground">文件</button>
        <button className="rounded-sm px-2 py-1 text-muted-foreground hover:bg-foreground/[0.04]">变更 <span className="ms-0.5 rounded-capsule bg-foreground/10 px-1 text-[10px]">3</span></button>
        <button className="rounded-sm px-2 py-1 text-muted-foreground hover:bg-foreground/[0.04]">终端</button>
        <button className="ms-auto flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.04]" aria-label="收起"><Icon name="close" size={12} /></button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1 font-mono text-[11px]">
        {FILES.map((f) => (
          <div key={f.name} className={`flex items-center gap-1.5 rounded-sm px-1 py-[3px] ${f.kind === "dir" ? "text-muted-foreground" : "text-foreground hover:bg-foreground/[0.04]"}`} style={{ paddingInlineStart: `${6 + f.depth * 10}px` }}>
            <span className="text-muted-foreground/70">{f.kind === "dir" ? "▾" : <Icon name="file" size={11} />}</span>
            <span className="min-w-0 flex-1 truncate">{f.name}</span>
            {f.mod ? <span className="text-[10px] text-live">M</span> : null}
            {f.add ? <span className="text-[10px] text-live">A</span> : null}
          </div>
        ))}
        <div className="mt-2 border-t border-border pt-2">
          <p className="px-1 text-[10px] font-medium uppercase tracking-[0.08em] text-muted-foreground">本次变更</p>
          {["App.tsx", "Sidebar.tsx", "workspace.ts"].map((n) => (
            <div key={n} className="flex items-center gap-1.5 rounded-sm px-1 py-1 hover:bg-foreground/[0.04]"><Icon name="diff" size={11} /><span className="truncate">{n}</span><span className="ms-auto text-[10px] text-muted-foreground">+{n === "workspace.ts" ? 18 : 42} −{n === "workspace.ts" ? 2 : 11}</span></div>
          ))}
        </div>
      </div>
      <div className="border-t border-border px-2 py-1.5 text-[10px] text-muted-foreground">⌘B 折叠 · 文件变更实时同步（mock）</div>
    </aside>
  );
}

function Palette() {
  return (
    <div className="absolute inset-0 z-30 flex justify-center bg-foreground/25 px-6 pt-[18vh] backdrop-blur-[2px]">
      <div className="h-fit w-full max-w-[520px] overflow-hidden rounded-xl border border-border bg-popover shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center gap-2 border-b border-border px-3 py-2 text-muted-foreground"><Icon name="search" size={14} /><span className="flex-1 text-[13px] text-foreground">输入命令…</span><span className="rounded-sm border border-border px-1 font-mono text-[10px]">Esc</span></div>
        <div className="p-1.5">
          {COMMANDS.map((c, i) => (
            <button key={c.key} className={`flex w-full items-center gap-2 rounded-control px-2 py-[7px] text-start text-[12px] ${i === 0 ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-foreground/[0.04]"}`}>
              <Icon name={c.key === "workspace" ? "folder" : c.key === "theme" ? "spark" : c.key === "settings" ? "sliders" : c.key === "keys" ? "command" : "plus"} size={12} />
              <span className="flex-1">{c.title}</span><span className="font-mono text-[10px] text-muted-foreground">{c.hint}</span>
            </button>
          ))}
        </div>
        <div className="flex items-center gap-3 border-t border-border px-3 py-1.5 text-[10px] text-muted-foreground"><span>↑↓ 移动</span><span>↵ 执行</span><span className="ms-auto">ZeoKeel 命令</span></div>
      </div>
    </div>
  );
}

function Main({ dark, palette }: { dark: boolean; palette: boolean }) {
  useEffect(() => {
    document.documentElement.dataset["theme"] = dark ? "dark" : "light";
    document.documentElement.lang = "zh-CN";
  }, [dark]);
  return (
    <div className="relative flex h-dvh flex-col overflow-hidden bg-background text-foreground">
      <div className="flex min-h-0 flex-1">
        <LeftRail />
        <main className="flex min-w-0 flex-1 flex-col">
          <Header />
          <div className="min-h-0 flex-1 overflow-y-auto"><Thread /></div>
          <Composer />
        </main>
        <RightRail />
      </div>
      <StatusBar />
      {palette ? <Palette /> : null}
    </div>
  );
}

export function DensePrototype() {
  const params = new URLSearchParams(window.location.search);
  const screen = params.get("screen") ?? "dense";
  const dark = params.get("theme") === "dark";
  if (screen === "palette") return <Main dark={false} palette />;
  if (screen === "dark") return <Main dark palette={false} />;
  return <Main dark={dark} palette={false} />;
}

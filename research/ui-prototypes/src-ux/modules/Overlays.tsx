/** Modules: command palette + settings dialog skeletons. */
import { Icon } from "./kit";

const COMMANDS = [
  { title: "新建会话", hint: "⌘N" },
  { title: "切换工作区", hint: "⌘⇧W" },
  { title: "切换主题：浅色 / 深色", hint: "⌘⇧L" },
  { title: "打开设置", hint: "⌘," },
  { title: "查看键盘快捷键", hint: "?" },
];

export function CommandPalette(props: { open: boolean; onClose: () => void }) {
  if (!props.open) {
    return null;
  }
  return (
    <div className="absolute inset-0 z-40 flex justify-center bg-foreground/25 px-6 pt-[18vh] backdrop-blur-[2px]" role="dialog" aria-modal="true" aria-label="命令面板">
      <button type="button" aria-label="关闭命令面板" className="absolute inset-0 border-0" onClick={props.onClose} />
      <div className="relative h-fit w-full max-w-[520px] overflow-hidden rounded-xl border border-border bg-popover shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center gap-2 border-b border-border px-3 py-2 text-muted-foreground"><Icon name="search" size={14} /><span className="flex-1 text-[13px] text-foreground">输入命令…</span><span className="rounded-sm border border-border px-1 font-mono text-[10px]">Esc</span></div>
        <div className="p-1.5">
          {COMMANDS.map((c, i) => <button key={c.title} type="button" className={`flex w-full items-center gap-2 rounded-control px-2 py-[7px] text-start text-[12px] ${i === 0 ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-foreground/[0.04]"}`}><Icon name="command" size={12} /><span className="flex-1">{c.title}</span><span className="font-mono text-[10px]">{c.hint}</span></button>)}
        </div>
        <div className="flex items-center gap-3 border-t border-border px-3 py-1.5 text-[10px] text-muted-foreground"><span>↑↓ 移动</span><span>↵ 执行</span><span className="ms-auto">ZeoKeel 命令</span></div>
      </div>
    </div>
  );
}

export function SettingsSkeleton(props: { open: boolean; onClose: () => void }) {
  if (!props.open) {
    return null;
  }
  return (
    <div className="absolute inset-0 z-40 flex items-center justify-center bg-foreground/25 p-4" role="dialog" aria-modal="true" aria-label="设置">
      <button type="button" aria-label="关闭设置" className="absolute inset-0 border-0" onClick={props.onClose} />
      <div className="relative w-[min(560px,100%)] rounded-xl border border-border bg-popover p-4 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between"><h2 className="text-[15px] font-semibold">设置</h2><button type="button" aria-label="关闭" onClick={props.onClose} className="flex h-7 w-7 items-center justify-center rounded-control hover:bg-foreground/[0.05]"><Icon name="close" size={12} /></button></div>
        <p className="mt-1 text-[12px] text-muted-foreground">个人版本地设置；密钥只保存在本机，不会回显。</p>
        <div className="mt-3 rounded-document bg-muted p-3 text-[12px]">
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="flex flex-col gap-1"><span className="text-[11px] text-muted-foreground">Provider</span><select className="rounded-control border border-border bg-background px-2 py-1.5"><option>Mock（离线演示）</option><option>OpenAI 兼容</option><option>Ollama</option></select></label>
            <label className="flex flex-col gap-1"><span className="text-[11px] text-muted-foreground">模型</span><input defaultValue="DeepSeek V3" className="rounded-control border border-border bg-background px-2 py-1.5" /></label>
            <label className="flex flex-col gap-1 sm:col-span-2"><span className="text-[11px] text-muted-foreground">Base URL</span><input placeholder="https://api.example.com/v1" className="rounded-control border border-border bg-background px-2 py-1.5" /></label>
            <label className="flex flex-col gap-1 sm:col-span-2"><span className="text-[11px] text-muted-foreground">API Key（留空保持已保存值）</span><input type="password" placeholder="不会回显" className="rounded-control border border-border bg-background px-2 py-1.5" /></label>
          </div>
        </div>
        <div className="mt-3 rounded-document bg-muted p-3 text-[12px]">
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="flex flex-col gap-1"><span className="text-[11px] text-muted-foreground">主题</span><select className="rounded-control border border-border bg-background px-2 py-1.5"><option>跟随系统</option><option>浅色</option><option>深色</option></select></label>
            <label className="flex flex-col gap-1"><span className="text-[11px] text-muted-foreground">语言</span><select className="rounded-control border border-border bg-background px-2 py-1.5"><option>中文（简体）</option><option>English</option></select></label>
          </div>
          <label className="mt-2 flex items-center gap-2"><span className="h-4 w-7 rounded-capsule bg-foreground/20 p-0.5"><span className="block h-3 w-3 translate-x-3 rounded-capsule bg-foreground" /></span><span className="text-[12px]">自动批准只读工具</span></label>
        </div>
        <div className="mt-4 flex justify-end gap-2"><button type="button" onClick={props.onClose} className="rounded-control border border-border px-3 py-1.5 text-[12px]">取消</button><button type="button" className="rounded-control bg-foreground px-3 py-1.5 text-[12px] font-medium text-background">保存</button></div>
      </div>
    </div>
  );
}

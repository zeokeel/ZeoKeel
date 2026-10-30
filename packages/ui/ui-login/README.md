# @zeokeel/ui-login (`ui.login`)

**功能说明（M2 P4）**：企业客户端登录/首配页 UI。

## 职责
- 官方企业版：`server_url` 为空 → 首配页显示服务器地址输入并本地保存；登录页本身不显示地址。
- OEM 渠道：`server_url` 由渠道预置 → 只显示账号/密码，地址不可编辑。
- 字段：账号、密码、可选服务器地址、设备名；提交中禁用；错误内联。
- 六态：初始、校验失败、提交中、服务器不可达、密码错误、锁定（429）。
- a11y：label 关联、错误 `role=alert`、密码显隐按钮、Enter 提交、焦点可见。

## Props（约定）
```ts
interface LoginPageProps {
  serverUrl: string | null;       // null = 需要首配
  channelLocked: boolean;         // OEM 渠道预置
  loading: boolean;
  error: string | null;
  onSubmit(input: { serverUrl?: string; username: string; password: string; deviceName: string }): void;
}
```

## 非目标
- 不直接 fetch/存 token（`auth.client` 负责）；不做 MFA UI（D53 可选）；不做 OIDC 浏览器跳转（后续）。

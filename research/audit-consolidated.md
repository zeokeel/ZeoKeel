# 全仓库文档审计：冲突点清单与处置结果

> 审计时间：2026-10-30 14:00–14:12（Asia/Shanghai）。
> 方法：3 个只读子代理分域审计（治理 / 构建交付 / 客户端服务端）+ 2 份机械扫描（链接、标题、编号、
> 已废弃关键词、主题断言矩阵）+ Lead 汇总与修复。子报告：
> [`audit-governance.md`](audit-governance.md)、[`audit-build-oem.md`](audit-build-oem.md)、
> [`audit-client-server.md`](audit-client-server.md)；机械报告：
> [`audit-automated.md`](audit-automated.md)、[`audit-topics.md`](audit-topics.md)。
>
> **结论：P0 冲突 0 个（审计中发现的 10+ 个已全部修复）；P1 冲突 0 个（含 09 的 Q 编号前缀化）；
> P2 剩余为文档门禁/历史文档/证据类，不影响实现决策。**

## 1. 审计范围

- `AGENTS.md`、`README.md`、`docs/00–18`、`docs/PLAN.md`
- `configs/*`、`experiments/*/README.md`、`experiments/*` 的命令与路径
- 对照 `research/harness-*.md`、`research/kernel.md`、`research/rig.md`、`research/frontend.md`
- 机械检查：相对链接、重复标题、代码块闭合、ADR/D 编号连续性、H1 与文件名、表格分隔行、
  “强管控/部门/Chat Completions/Windows 签名”等已废弃口径残留、“参考仓库当自己仓库”表述。

## 2. P0 冲突（已修复，记录以防复发）

| # | 冲突 | 位置（修复前） | 修复 |
| --- | --- | --- | --- |
| 1 | 01 顶部把“可控性/员工不能自由操作”写成最高优先级，与弱管控、00/17/ADR-030 相反 | `docs/01` 顶部 | 改为弱管控口径；`00` 明确权威顺序 |
| 2 | 企业版只写“追加/覆盖”，未 `exclude` 个人专属包，会把本地 provider 设置 UI、GitHub 更新器带进企业版 | `09` §1.4、`12` M2 | 统一为 `(personal − personal-only) + 企业包 + 覆盖`；03/10/13/PLAN 同步 |
| 3 | 更新包写 `.zip`，但交付/升级是 `.tar.zst` + `docker load`，升级链断 | `09` §7.2、ADR-041 | 统一为 `tar.zst` 实际文件名 |
| 4 | 03 组合模型无 channel 层，CLI 用 `--oem` 而 09 用 `--channel` | `03` §3/CLI | 改为 `edition × profile × channel`；CLI 统一 `--channel` |
| 5 | 登录地址四处矛盾：04 说只来自构建期、05 说空时可输入、09 说唯一真源、13 说不得提供改地址入口 | `04/05/09/13/15` | 统一：非空锁定免填；空 → 首配页输入并本地保存（设置可改，需重登）；登录请求不带地址 |
| 6 | JWT 写“access 最迟一个 TTL 失效”，与“禁用后立即失效”相反 | `04` §3、ADR-027 后果 | 改为每请求校验 `token_epoch`（≤5s 缓存，写操作查库），下一次请求即 401 |
| 7 | WASM 宿主函数两套清单：四个扁平函数 vs `db.define/tx_*` | `04` §9 | 统一为 `db.exec/db.query/log/asset.read` 四个；扩展必须先改 ADR-035 |
| 8 | 17 决策编号 `#23` 重复（WASM 语言 / demo 仓库） | `17` | demo 仓库改为 `#34` |
| 9 | 09/15/17 把参考仓库 `zeokeel/channels` 当作我们的索引仓 | `09` §2、`15 D42`、`17` | 改为“仅字段参考”；索引仓自建（demo `zeokeel/demo-oem-index`）；AGENTS 新增 C15 |
| 10 | 13 重复标题（风险登记 ×3）、03 重复标题（5.6 ×2）、PLAN 表格分隔行错位 | 多处 | 删除重复；修表格 |
| 11 | 四份 docs H1 编号错误（02/03/13/16 写成 01/02/07/04） | docs 头部 | 全部改为文件名编号 |

## 3. P1 冲突（已修复）

| # | 冲突 | 位置 | 修复 |
| --- | --- | --- | --- |
| 1 | “组织/org”残留：服务端职责、包清单、`/auth/me.org`、PLAN 架构图 | 04/PLAN/README/12 | 全部改为用户/角色；04 删 `org` 字段；12 改 `auth/users/...` |
| 2 | 角色枚举缺 `auditor`（04 只列超管/员工；06 权限枚举同） | 04/06 | 补 `auditor`；07/08 同步只读范围 |
| 3 | MCP OAuth：04 写服务端回调/refresh 存服务端/禁 stdio，与“客户端 OAuth、令牌本地、允许 stdio”冲突 | 04 §2.5、07 §6、15/17 | 统一客户端 OAuth + 令牌本地 + 允许 stdio（D33/D53） |
| 4 | 可见性残留“全员/私有”，与“全员/指定用户”冲突 | 12/15 D18/D29/17 #8/13 ADR-032 | 统一全员/指定用户；旧决策注明被 D47/ADR-036 修订 |
| 5 | Windows 签名矛盾：PLAN“我们必须签”、R22“采购证书”、11“Windows/macOS 签名校验” | PLAN/13/11/10 | 统一不签名（ADR-033）；macOS 需公证（ADR-037，证书后配） |
| 6 | macOS 公证“待定/是否保留”与 ADR-037“需要”冲突 | 13 ADR-033、09 Q1、15 D36 | 统一“需要公证，本地先测 → CI 就绪 → 证书后启用” |
| 7 | 正式决策仍标“待确认”：网关协议、契约漂移已验证、macOS、MFA、灰度、令牌存放 | 01/04/06/09/13/17 | 改为已定/引用 D 编号；未定项标记为细节 |
| 8 | ADR 状态机制缺失：ADR-032 可见性被 036 取代未标注；ADR-029 后果仍是强管控 | 13 | 加 supersede 说明；ADR-029 后果标“历史，已由 ADR-030 取代” |
| 9 | 07 与 02 契约/管理台包清单不一致（7 vs 22；约 10 vs 11） | 07/02/08 | 07 明确“服务端域契约 8 个 + 客户端契约见 02”；08 补 wallet/pricing/usage；计数改“由 zk plan 派生” |
| 10 | 06/09 `.env` 键名与数量冲突（4 vs 6；`ZEOKEEL_ADMIN_PASSWORD` vs `ADMIN_PASSWORD`） | 06/09 | 统一 6 键；09 说明至少改 4 个 |
| 11 | 06 升级写“拉新镜像”，与手动离线导入冲突 | 06 | 改为 `docker load` → 改 `SERVER_IMAGE` → compose → readyz |
| 12 | 下载/更新路径三套：`/updates/*`、`/updates/client`、`/api/updates/client` | 04/06/09 | 统一 `/api/updates/*` |
| 13 | 04 越权测试写 403，与“未授权资源 404”规则冲突 | 06 | 拆 404（不存在/无权）与 403（RBAC 不足） |
| 14 | UI 六态门禁只有五态，缺 offline | 11 | 补 offline；与 AGENTS/16 一致 |
| 15 | 17 过期声明：ADR 范围 030…034、未决项混入已决 | 17 | 改 030…044；未决只留细节 |
| 16 | 03 `cargo nextest` 缺 `run`、18 `zk db lock --check` 不存在 | 03/18 | 统一可执行命令 |
| 17 | 10 用 `[defaults]`，03 规定 `[packages]`；包目录二层/三层冲突 | 10/16/02 | 统一 `[packages]` 与三层 `packages/<group>/<name>` |
| 18 | 16 引用 02 §6/§5.5（应为 03）；实验 README 引用 02/05/08 旧路径 | 16/experiments | 修正引用与 vendor profile 路径 |
| 19 | 01 §9 仍列已决问题（未登录仅登录页、允许导出） | 01 | 删除已决项，指向 15 |
| 20 | README 缺 `brands/`、`oem/` 语义与 10 不一致、公式缺 exclude | README/10 | 补齐并统一措辞 |

## 4. 仍存问题（P2 若干，均不阻塞）

| 严重度 | 问题 | 位置 | 处置建议 |
| --- | --- | --- | --- |
| P2 | `D53` 一条捆绑 12+ 个不相关决策，难追溯/supersede | 15 D53 | 拆成独立 D 编号（下次决策变更时做） |
| P2 | 01 §9 仍有两条问题不在 15（个人版本地服务端、OEM 改核心交互边界） | 01 §9 | 登记进 15 或删除 |
| P2 | ADR-013（wasm 叶子包，open）与 ADR-035（WASM 应用）边界未说明 | 13 | 加一句“ADR-013 指客户端工具 wasm，与应用中心无关” |
| P2 | 14 历史文档交叉引用大量失效（引用 v2 章节），虽已加“历史快照”警示 | 14 | 全量修引用或整篇冻结（不再维护） |
| P2 | 无文档结构 CI 门禁（H1/重复标题/表格/链接/编号/废弃口径） | CI | 增加 `scripts/docs-check`（本次手工检查的规则固化） |
| P2 | wasm POC 的 golden 输出未落证据文件（README 自述） | experiments | 把 `build.sh`/`golden/run.py` 输出写 `target/evidence/*.log` 或 CI artifact |
| P2 | demo 仓“已创建”缺 URL/commit 证据 | 15/17 | 已补三个仓链接；commit 由 git 历史提供 |
| P2 | 04 仍有一些合法未定项（refresh 寿命、OpenAPI 是否对外、强制更新离线宽限） | 04 §11 | 保持“待确认”，不要误当冲突 |

## 5. 冲突分类统计（本次发现）

| 类别 | 发现数 | 已修 | 剩余 |
| --- | --- | --- | --- |
| 版本/edition/企业版组合 | 9 | 9 | 0 |
| 渠道/official/server_url/登录 | 8 | 8 | 0 |
| 更新通道/交付/签名 | 7 | 7 | 0 |
| 弱管控/部门/可见性/角色 | 10 | 10 | 0 |
| 认证/JWT | 3 | 3 | 0 |
| WASM ABI/应用 API | 4 | 4 | 0 |
| MCP/连接器 | 4 | 4 | 0 |
| 契约/包清单/命名 | 8 | 8 | 0 |
| 部署/环境/命令 | 7 | 7 | 0 |
| 文档结构/编号/链接/过期声明 | 18 | 18 | 0 |
| 证据/门禁 | 3 | 1 | 2（docs-check、wasm evidence） |

## 6. 防复发（建议立即进 CI）

1. `docs-check`：H1 == 文件名编号、无重复标题、表格分隔行合法、相对链接存在、代码块成对。
2. 决策一致性：`15` 为唯一已定清单；其它文档出现“待确认/待定”时必须能在 15 找到对应项或被白名单豁免。
3. 参考仓库禁令：扫描 `zeokeel/channels|ZeoKeel-Harness|ZeoKeel-Next`，只允许出现在“参考/研究”上下文，禁止出现在 `依赖/索引仓/构建输入/数据源` 上下文。
4. 数字由生成器产出：包数/端点数/契约数禁止手写；`docs/generated/` + `--check`。
5. ADR 状态机：`accepted` 不得互相矛盾；被取代必须 `superseded-by` 并回写引用。

> 本次审计的所有机械证据可用 `python3 /tmp/doc_audit.py`（脚本在临时目录，未入库）复跑；
> 若需长期使用，应把规则落到 `scripts/docs-check` 并进 CI。

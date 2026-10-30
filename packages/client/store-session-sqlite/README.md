# zk-store-session-sqlite

会话持久化实现：SQLite（WAL），表 `conversations`/`messages`。默认路径
`$ZEOKEEL_HOME/sessions.db`（否则系统 data 目录）。规范见 `docs/24` §7。

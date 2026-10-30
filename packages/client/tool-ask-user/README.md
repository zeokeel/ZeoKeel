# tool-ask-user (`tool.ask_user`)

**功能说明**：`ask_user_question` 发 QuestionAsked 事件并阻塞等待 UI 回答（`question.answer`），超时 300s 返回 null；无 broker 时拒绝。

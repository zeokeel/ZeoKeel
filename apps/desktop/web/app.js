/*
 * ZeoKeel 最小 Demo — static UI (no bundler, no external CDN).
 *
 * Talks to the Tauri main process through the single `ipc_call` command:
 *   demo.meta  -> { edition, packages }
 *   demo.chat  -> { text, tool_calls: [{ name, args, result }] }
 */
(function () {
  "use strict";

  var chatEl = document.getElementById("chat");
  var metaEl = document.getElementById("meta");
  var noticeEl = document.getElementById("notice");
  var inputEl = document.getElementById("input");
  var sendEl = document.getElementById("send");

  var busy = false;

  function hasTauri() {
    return (
      typeof window.__TAURI__ !== "undefined" &&
      window.__TAURI__ !== null &&
      typeof window.__TAURI__.core !== "undefined" &&
      window.__TAURI__.core !== null &&
      typeof window.__TAURI__.core.invoke === "function"
    );
  }

  function ipc(name, payload) {
    return window.__TAURI__.core.invoke("ipc_call", {
      name: name,
      payload: payload,
    });
  }

  function formatValue(value) {
    if (typeof value === "string") {
      return value;
    }
    if (value === undefined || value === null) {
      return String(value);
    }
    try {
      var json = JSON.stringify(value, null, 2);
      return typeof json === "string" ? json : String(value);
    } catch (err) {
      return String(value);
    }
  }

  function errText(err) {
    if (typeof err === "string") {
      return err;
    }
    if (err !== null && typeof err === "object" && typeof err.message === "string") {
      return err.message;
    }
    return String(err);
  }

  function showNotice(text) {
    noticeEl.hidden = false;
    noticeEl.textContent = text;
  }

  function hideNotice() {
    noticeEl.hidden = true;
    noticeEl.textContent = "";
  }

  function scrollToBottom() {
    chatEl.scrollTop = chatEl.scrollHeight;
  }

  function removeHint() {
    var hint = chatEl.querySelector(".hint");
    if (hint !== null) {
      hint.remove();
    }
  }

  function appendMessage(kind, text) {
    removeHint();

    var wrap = document.createElement("div");
    wrap.className = "msg msg-" + kind;

    var label = document.createElement("div");
    label.className = "msg-label";
    label.textContent =
      kind === "user" ? "你" : kind === "error" ? "错误" : "ZeoKeel";

    var bubble = document.createElement("div");
    bubble.className = "bubble";
    bubble.textContent = text;

    wrap.appendChild(label);
    wrap.appendChild(bubble);
    chatEl.appendChild(wrap);
    scrollToBottom();
    return wrap;
  }

  function field(label, value) {
    var row = document.createElement("div");
    row.className = "tool-call-field";

    var name = document.createElement("span");
    name.textContent = label;
    row.appendChild(name);

    var pre = document.createElement("pre");
    pre.textContent = formatValue(value);
    row.appendChild(pre);
    return row;
  }

  function renderToolCalls(toolCalls) {
    var details = document.createElement("details");
    details.className = "tool-calls";

    var summary = document.createElement("summary");
    summary.textContent = "工具调用（" + toolCalls.length + "）";
    details.appendChild(summary);

    toolCalls.forEach(function (call) {
      if (call === null || typeof call !== "object") {
        return;
      }
      var item = document.createElement("div");
      item.className = "tool-call";

      var callName = document.createElement("div");
      callName.className = "tool-call-name";
      callName.textContent = typeof call.name === "string" ? call.name : "unknown";
      item.appendChild(callName);

      item.appendChild(field("参数", call.args));
      item.appendChild(field("结果", call.result));
      details.appendChild(item);
    });

    return details;
  }

  function renderReply(wrap, result) {
    var bubble = wrap.querySelector(".bubble");
    if (bubble === null) {
      return;
    }
    bubble.classList.remove("bubble-pending");

    var text = "";
    var toolCalls = [];
    if (typeof result === "string") {
      text = result;
    } else if (result !== null && typeof result === "object") {
      text = typeof result.text === "string" ? result.text : formatValue(result);
      if (Array.isArray(result.tool_calls)) {
        toolCalls = result.tool_calls;
      }
    } else {
      text = formatValue(result);
    }

    bubble.textContent = text.trim() === "" ? "（空回复）" : text;
    if (toolCalls.length > 0) {
      wrap.appendChild(renderToolCalls(toolCalls));
    }
    scrollToBottom();
  }

  function renderError(wrap, err) {
    wrap.classList.add("msg-error");
    var bubble = wrap.querySelector(".bubble");
    if (bubble !== null) {
      bubble.classList.remove("bubble-pending");
      bubble.textContent = "错误：" + errText(err);
    }
    scrollToBottom();
  }

  function setBusy(next) {
    busy = next;
    inputEl.disabled = next;
    sendEl.disabled = next;
    sendEl.textContent = next ? "发送中…" : "发送";
  }

  function send() {
    if (busy) {
      return;
    }
    var text = inputEl.value.trim();
    if (text === "") {
      showNotice("请输入内容后再发送。");
      return;
    }
    hideNotice();

    appendMessage("user", text);
    inputEl.value = "";
    setBusy(true);

    var wrap = appendMessage("assistant", "思考中…");
    var pending = wrap.querySelector(".bubble");
    if (pending !== null) {
      pending.classList.add("bubble-pending");
    }

    ipc("demo.chat", { text: text })
      .then(function (result) {
        renderReply(wrap, result);
      })
      .catch(function (err) {
        renderError(wrap, err);
      })
      .then(function () {
        setBusy(false);
        inputEl.focus();
        scrollToBottom();
      });
  }

  function loadMeta() {
    ipc("demo.meta", {})
      .then(function (data) {
        var edition =
          data !== null && typeof data === "object" && typeof data.edition === "string"
            ? data.edition
            : "未知";
        var packages =
          data !== null && typeof data === "object" && Array.isArray(data.packages)
            ? data.packages
            : [];
        var inventory = packages.length > 0 ? packages.join("、") : "未配置包（占位）";
        metaEl.textContent = "版本：" + edition + " · 包：" + inventory;
      })
      .catch(function (err) {
        metaEl.textContent = "组合信息读取失败：" + errText(err);
      });
  }

  if (!hasTauri()) {
    metaEl.textContent = "非桌面环境";
    showNotice("请用桌面程序打开（当前页面未检测到 Tauri 运行时）。");
    inputEl.disabled = true;
    sendEl.disabled = true;
    return;
  }

  sendEl.addEventListener("click", send);
  inputEl.addEventListener("keydown", function (event) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  });

  loadMeta();
  inputEl.focus();
})();

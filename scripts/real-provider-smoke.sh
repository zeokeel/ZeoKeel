#!/usr/bin/env bash
# B14 — real provider / media / embedding online smoke (env-gated; honest: no fake pass).
#
# Runs real HTTPS calls against the same provider endpoints the shipped clients use.
# Each check runs only when its env is configured; missing checks are reported as
# UNVERIFIED (never as pass). With no provider env at all the script prints
# UNVERIFIED and exits 2. No mock/fixture response can make it pass.
#
# Checks:
#   chat          POST {ZEOKEEL_REAL_PROVIDER_BASE_URL|ZEOKEEL_OPENAI_BASE_URL}/chat/completions
#                 (client llm-openai-compatible wire). Env: *_API_KEY, *_MODEL.
#   embedding     POST ZEOKEEL_EMBEDDING_BASE_URL/embeddings (cap-vector wire).
#   image         POST <IMAGE>/images/generations (cap-image).
#   audio         POST <AUDIO>/audio/speech (cap-audio).
#   transcription POST <TRANSCRIPTION>/audio/transcriptions multipart (cap-transcription).
#   rerank        POST <RERANK>/rerank (cap-rerank; voyageai-compatible wire; provider-specific).
#   gateway       POST <provider base>/responses — the exact upstream endpoint the server
#                 gateway calls (C7 Responses-only); env: ZEOKEEL_REAL_RESPONSES_MODEL.
#
# Environment (prefixes are the app's own capability env names):
#   ZEOKEEL_REAL_PROVIDER_BASE_URL / _API_KEY / _MODEL   chat (fallback ZEOKEEL_OPENAI_*)
#   ZEOKEEL_EMBEDDING_BASE_URL / _API_KEY / _MODEL       embeddings
#   ZEOKEEL_IMAGE_BASE_URL / _API_KEY / _MODEL           image generation
#   ZEOKEEL_AUDIO_BASE_URL / _API_KEY / _MODEL           text-to-speech
#   ZEOKEEL_TRANSCRIPTION_BASE_URL / _API_KEY / _MODEL   speech-to-text
#   ZEOKEEL_RERANK_BASE_URL / _API_KEY / _MODEL          rerank
#   ZEOKEEL_REAL_RESPONSES_MODEL                          gateway upstream Responses call
#   ZEOKEEL_REAL_SMOKE_TIMEOUT_SECS                       per-request timeout (default 60)
#   ZEOKEEL_REAL_SMOKE_OUT                                JSON evidence path
#
# Exit: 0 = every executed check passed (unverified list reported); 1 = a check
# failed; 2 = UNVERIFIED (no provider env configured / python3 missing).
set -uo pipefail
cd "$(dirname "$0")/.."

OUT="${ZEOKEEL_REAL_SMOKE_OUT:-target/evidence/release-1.0-rc1/phase-b/b12b14/real-provider-smoke.json}"
TIMEOUT_SECS="${ZEOKEEL_REAL_SMOKE_TIMEOUT_SECS:-60}"

usage() {
  sed -n '2,/^set -uo pipefail/p' "$0" | sed 's/^# \{0,1\}//; /^set -uo pipefail/d'
}

while (($# > 0)); do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    *) echo "real-provider-smoke: unknown argument: $1 (try --help)" >&2; exit 2 ;;
  esac
done

command -v python3 >/dev/null 2>&1 || { echo "real-provider-smoke: UNVERIFIED — python3 not available"; exit 2; }
mkdir -p "$(dirname "$OUT")"

python3 - "$OUT" "$TIMEOUT_SECS" <<'PY'
import json
import os
import struct
import sys
import tempfile
import time
import urllib.error
import urllib.request

out_path, timeout = sys.argv[1], float(sys.argv[2])


def env(*names, default=""):
    for name in names:
        value = os.environ.get(name, "").strip()
        if value:
            return value
    return default


def base_of(prefix, fallback_prefix=""):
    value = env(prefix + "_BASE_URL", (fallback_prefix + "_BASE_URL") if fallback_prefix else "")
    return value.rstrip("/")


def http_json(method, url, body, key, extra_headers=None):
    data = json.dumps(body).encode()
    request = urllib.request.Request(url, data=data, method=method)
    request.add_header("content-type", "application/json")
    request.add_header("authorization", "Bearer " + key)
    for header, value in (extra_headers or {}).items():
        request.add_header(header, value)
    started = time.perf_counter()
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = response.read()
            return {"ok": True, "status": response.status, "body": payload,
                    "latency_ms": round((time.perf_counter() - started) * 1000.0, 1)}
    except urllib.error.HTTPError as error:
        return {"ok": False, "status": error.code, "body": error.read(),
                "latency_ms": round((time.perf_counter() - started) * 1000.0, 1),
                "error": "http %d: %s" % (error.code, error.read()[:200])}
    except Exception as error:
        return {"ok": False, "status": 0, "body": b"",
                "latency_ms": round((time.perf_counter() - started) * 1000.0, 1),
                "error": "%s: %s" % (type(error).__name__, error)}


def decode_body(result):
    try:
        return json.loads(result["body"])
    except Exception:
        return None


checks = []


def record(name, configured, status, detail, latency_ms=None):
    checks.append({"name": name, "configured": configured, "status": status,
                   "detail": detail, "latency_ms": latency_ms})


# ------------------------------------------------------------------ chat completion
chat_base = base_of("ZEOKEEL_REAL_PROVIDER", "ZEOKEEL_OPENAI")
chat_key = env("ZEOKEEL_REAL_PROVIDER_API_KEY", "ZEOKEEL_OPENAI_API_KEY")
chat_model = env("ZEOKEEL_REAL_PROVIDER_MODEL", "ZEOKEEL_OPENAI_MODEL")
if chat_base and chat_key and chat_model:
    result = http_json("POST", chat_base + "/chat/completions", {
        "model": chat_model,
        "messages": [{"role": "user", "content": "Reply with the single word: pong"}],
        "max_tokens": 16,
    }, chat_key)
    decoded = decode_body(result)
    if result["ok"] and isinstance(decoded, dict):
        choices = decoded.get("choices")
        content = ""
        if isinstance(choices, list) and choices:
            message = choices[0].get("message") if isinstance(choices[0], dict) else None
            content = message.get("content") if isinstance(message, dict) else None
        if isinstance(content, str) and content.strip():
            record("chat", True, "pass", "model=%s reply=%r" % (chat_model, content.strip()[:40]), result["latency_ms"])
        else:
            record("chat", True, "fail", "200 but no message content: %s" % (result["body"][:200],), result["latency_ms"])
    else:
        record("chat", True, "fail", result.get("error", "no response"), result["latency_ms"])
else:
    record("chat", False, "unverified", "set ZEOKEEL_REAL_PROVIDER_BASE_URL/_API_KEY/_MODEL (or ZEOKEEL_OPENAI_*)")


# ------------------------------------------------------------------ embeddings
embed_base = env("ZEOKEEL_EMBEDDING_BASE_URL").rstrip("/")
embed_key = env("ZEOKEEL_EMBEDDING_API_KEY")
embed_model = env("ZEOKEEL_EMBEDDING_MODEL", default="text-embedding-3-small")
if embed_base and embed_key:
    result = http_json("POST", embed_base + "/embeddings",
                       {"model": embed_model, "input": "zeokeel real provider smoke"}, embed_key)
    decoded = decode_body(result)
    vector = None
    if result["ok"] and isinstance(decoded, dict):
        data = decoded.get("data")
        if isinstance(data, list) and data and isinstance(data[0], dict):
            vector = data[0].get("embedding")
    if isinstance(vector, list) and vector:
        record("embedding", True, "pass", "model=%s dims=%d" % (embed_model, len(vector)), result["latency_ms"])
    else:
        record("embedding", True, "fail", result.get("error", "no embedding vector"), result["latency_ms"])
else:
    record("embedding", False, "unverified", "set ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY")


# ------------------------------------------------------------------ image generation
image_base = env("ZEOKEEL_IMAGE_BASE_URL").rstrip("/")
image_key = env("ZEOKEEL_IMAGE_API_KEY")
image_model = env("ZEOKEEL_IMAGE_MODEL", default="")
if image_base and image_key and image_model:
    result = http_json("POST", image_base + "/images/generations",
                       {"model": image_model, "prompt": "a single red dot on a white background", "n": 1}, image_key)
    decoded = decode_body(result)
    produced = None
    if result["ok"] and isinstance(decoded, dict):
        data = decoded.get("data")
        if isinstance(data, list) and data and isinstance(data[0], dict):
            produced = data[0].get("url") or data[0].get("b64_json")
    if produced:
        record("image", True, "pass", "model=%s" % image_model, result["latency_ms"])
    else:
        record("image", True, "fail", result.get("error", "no image url/b64"), result["latency_ms"])
else:
    record("image", False, "unverified", "set ZEOKEEL_IMAGE_BASE_URL/_API_KEY/_MODEL")


# ------------------------------------------------------------------ text to speech
audio_base = env("ZEOKEEL_AUDIO_BASE_URL").rstrip("/")
audio_key = env("ZEOKEEL_AUDIO_API_KEY")
audio_model = env("ZEOKEEL_AUDIO_MODEL", default="tts-1")
if audio_base and audio_key:
    result = http_json("POST", audio_base + "/audio/speech",
                       {"model": audio_model, "input": "hello from the ZeoKeel smoke test", "voice": "alloy"}, audio_key)
    if result["ok"] and len(result["body"]) > 100:
        record("audio", True, "pass", "model=%s bytes=%d" % (audio_model, len(result["body"])), result["latency_ms"])
    else:
        record("audio", True, "fail", result.get("error", "empty audio body (%d bytes)" % len(result["body"])), result["latency_ms"])
else:
    record("audio", False, "unverified", "set ZEOKEEL_AUDIO_BASE_URL/_API_KEY/_MODEL")


def silence_wav(seconds=1, rate=16000):
    frames = b"\x00\x00" * int(rate * seconds)
    data_size = len(frames)
    header = b"RIFF" + struct.pack("<I", 36 + data_size) + b"WAVE"
    header += b"fmt " + struct.pack("<IHHIIHH", 16, 1, 1, rate, rate * 2, 2, 16)
    header += b"data" + struct.pack("<I", data_size)
    return header + frames


# ------------------------------------------------------------------ transcription
trans_base = env("ZEOKEEL_TRANSCRIPTION_BASE_URL").rstrip("/")
trans_key = env("ZEOKEEL_TRANSCRIPTION_API_KEY")
trans_model = env("ZEOKEEL_TRANSCRIPTION_MODEL", default="whisper-1")
if trans_base and trans_key and trans_model:
    boundary = "----zeokeel-smoke-%d" % int(time.time() * 1000)
    wav = silence_wav()
    parts = []
    parts.append(("--%s\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n%s\r\n" % (boundary, trans_model)).encode())
    parts.append(("--%s\r\nContent-Disposition: form-data; name=\"file\"; filename=\"smoke.wav\"\r\n"
                  "Content-Type: audio/wav\r\n\r\n" % boundary).encode())
    parts.append(wav)
    parts.append(("\r\n--%s--\r\n" % boundary).encode())
    request = urllib.request.Request(trans_base + "/audio/transcriptions", data=b"".join(parts), method="POST")
    request.add_header("content-type", "multipart/form-data; boundary=" + boundary)
    request.add_header("authorization", "Bearer " + trans_key)
    started = time.perf_counter()
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = response.read()
            latency = round((time.perf_counter() - started) * 1000.0, 1)
        decoded = None
        try:
            decoded = json.loads(payload)
        except Exception:
            decoded = None
        if response.status == 200 and isinstance(decoded, dict) and isinstance(decoded.get("text"), str):
            record("transcription", True, "pass", "model=%s text=%r" % (trans_model, decoded["text"][:40]), latency)
        else:
            record("transcription", True, "fail", "200 but no text field: %s" % payload[:200], latency)
    except urllib.error.HTTPError as error:
        record("transcription", True, "fail", "http %d: %s" % (error.code, error.read()[:200]))
    except Exception as error:
        record("transcription", True, "fail", "%s: %s" % (type(error).__name__, error))
else:
    record("transcription", False, "unverified", "set ZEOKEEL_TRANSCRIPTION_BASE_URL/_API_KEY/_MODEL")


# ------------------------------------------------------------------ rerank (provider-specific wire)
rerank_base = env("ZEOKEEL_RERANK_BASE_URL").rstrip("/")
rerank_key = env("ZEOKEEL_RERANK_API_KEY")
rerank_model = env("ZEOKEEL_RERANK_MODEL", default="rerank-2.5")
if rerank_base and rerank_key:
    result = http_json("POST", rerank_base + "/rerank", {
        "model": rerank_model,
        "query": "zeokeel smoke test",
        "documents": ["zeokeel smoke test", "unrelated document"],
        "top_n": 1,
    }, rerank_key)
    decoded = decode_body(result)
    ranked = decoded.get("data") if isinstance(decoded, dict) else None
    if result["ok"] and isinstance(ranked, list) and ranked:
        record("rerank", True, "pass", "model=%s top=%r" % (rerank_model, ranked[0]), result["latency_ms"])
    else:
        record("rerank", True, "fail", result.get("error", "no rerank data (provider wire is provider-specific)"), result["latency_ms"])
else:
    record("rerank", False, "unverified", "set ZEOKEEL_RERANK_BASE_URL/_API_KEY/_MODEL")


# ------------------------------------------------------------------ gateway upstream (Responses API)
responses_model = env("ZEOKEEL_REAL_RESPONSES_MODEL")
if chat_base and chat_key and responses_model:
    result = http_json("POST", chat_base + "/responses",
                       {"model": responses_model, "input": "Reply with the single word: pong", "stream": False}, chat_key)
    decoded = decode_body(result)
    if result["ok"] and isinstance(decoded, dict) and (decoded.get("usage") or decoded.get("response") or decoded.get("output")):
        record("gateway", True, "pass", "model=%s responses-api reachable" % responses_model, result["latency_ms"])
    else:
        record("gateway", True, "fail", result.get("error", "not a Responses payload: %s" % (result["body"][:200],)), result["latency_ms"])
else:
    record("gateway", False, "unverified", "set ZEOKEEL_REAL_RESPONSES_MODEL (Responses-capable upstream base/key)")


# ------------------------------------------------------------------ report
executed = [check for check in checks if check["configured"]]
unverified = [check["name"] for check in checks if not check["configured"]]
failed = [check for check in executed if check["status"] == "fail"]
if not executed:
    payload = {
        "schema": 1,
        "status": "unverified",
        "checks": checks,
        "unverified": unverified,
        "note": "no real provider env configured; nothing was called (never treated as pass)",
    }
    with open(out_path, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, indent=2) + "\n")
    print("real-provider-smoke: UNVERIFIED — no provider env configured (chat/env names in --help)")
    sys.exit(2)

payload = {
    "schema": 1,
    "status": "fail" if failed else "pass",
    "checks": checks,
    "executed": [check["name"] for check in executed],
    "unverified": unverified,
    "note": "only configured checks were executed; unverified items remain UNVERIFIED",
}
with open(out_path, "w", encoding="utf-8") as handle:
    handle.write(json.dumps(payload, indent=2) + "\n")
for check in checks:
    if check["status"] == "pass":
        print("real-provider-smoke: %-13s pass %s (%sms)" % (check["name"], check["detail"], check["latency_ms"]))
    elif check["status"] == "fail":
        print("real-provider-smoke: %-13s FAIL %s" % (check["name"], check["detail"]))
    else:
        print("real-provider-smoke: %-13s UNVERIFIED %s" % (check["name"], check["detail"]))
print("real-provider-smoke: %s (executed=%s unverified=%s) -> %s" %
      (payload["status"], ",".join(payload["executed"]), ",".join(unverified) or "-", out_path))
sys.exit(1 if failed else 0)
PY
PY_EXIT=$?
if [[ "$PY_EXIT" = "0" ]]; then
  exit 0
fi
if [[ "$PY_EXIT" = "2" ]]; then
  exit 2
fi
exit 1

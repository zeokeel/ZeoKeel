package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"unsafe"
)

//go:wasmimport zeokeel db_exec
func dbExec(ptr, length uint32) int32

//go:wasmimport zeokeel db_query
func dbQuery(ptr, length, out, cap uint32) int32

//go:wasmimport zeokeel log
func logMsg(ptr, length uint32)

func ptrOf(s string) (uint32, uint32) {
	if len(s) == 0 {
		return 0, 0
	}
	return uint32(uintptr(unsafe.Pointer(unsafe.StringData(s)))), uint32(len(s))
}

func dbExecStr(s string) int32 {
	p, l := ptrOf(s)
	return dbExec(p, l)
}

func dbQueryStr(s string) (string, error) {
	p, l := ptrOf(s)
	buf := make([]byte, 256*1024)
	n := dbQuery(p, l, uint32(uintptr(unsafe.Pointer(&buf[0]))), uint32(len(buf)))
	if n < 0 {
		return "", fmt.Errorf("db_query error %d", n)
	}
	return string(buf[:n]), nil
}

func readFrame() ([]byte, error) {
	raw, err := io.ReadAll(os.Stdin)
	if err != nil {
		return nil, err
	}
	pos := -1
	for i, b := range raw {
		if b == '\n' {
			pos = i
			break
		}
	}
	if pos < 0 {
		return nil, fmt.Errorf("no frame header")
	}
	var length int
	if _, err := fmt.Sscanf(string(raw[:pos]), "RS%d", &length); err != nil {
		return nil, fmt.Errorf("bad header")
	}
	body := raw[pos+1:]
	if len(body) < length {
		return nil, fmt.Errorf("short frame")
	}
	return body[:length], nil
}

func writeFrame(v any) {
	body, _ := json.Marshal(v)
	fmt.Printf("RS%d\n", len(body))
	os.Stdout.Write(body)
}

func main() {
	body, err := readFrame()
	if err != nil {
		writeFrame(map[string]any{"status": 400, "body": map[string]any{"error": err.Error()}})
		return
	}
	var req struct {
		Body struct {
			Op   string `json:"op"`
			Name string `json:"name"`
		} `json:"body"`
	}
	_ = json.Unmarshal(body, &req)
	switch req.Body.Op {
	case "add":
		name := req.Body.Name
		if name == "" {
			name = "anonymous"
		}
		payload, _ := json.Marshal(map[string]any{
			"sql":    "INSERT INTO items(name) VALUES(?)",
			"params": []string{name},
		})
		rc := dbExecStr(string(payload))
		msg := "inserted"
		p, l := ptrOf(msg)
		logMsg(p, l)
		if rc == 0 {
			writeFrame(map[string]any{"status": 200, "body": map[string]any{"ok": true, "name": name}})
		} else {
			writeFrame(map[string]any{"status": 500, "body": map[string]any{"error": fmt.Sprintf("db_exec rc=%d", rc)}})
		}
	default:
		rows, err := dbQueryStr(`{"sql":"SELECT id,name FROM items ORDER BY id","params":[]}`)
		if err != nil {
			writeFrame(map[string]any{"status": 500, "body": map[string]any{"error": err.Error()}})
			return
		}
		var parsed any
		_ = json.Unmarshal([]byte(rows), &parsed)
		writeFrame(map[string]any{"status": 200, "body": map[string]any{"rows": parsed}})
	}
}

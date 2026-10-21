#!/usr/bin/env python3
"""A stand-in for Micold's tool server, for recording permission-prompt fixtures (#942, T002).

Serves MCP over plain HTTP JSON-RPC on loopback: `initialize`, `tools/list` (the one
`permission_prompt` tool of specs/942-structured-permission-prompts/contracts/claude-permission.md)
and `tools/call`. Each `tools/call` is answered from a scripted list, one answer per call in order
(`allow` once the list runs out), and logged as one JSON line {"arguments", "_meta", "answer"}.

Answers are written as names, built from the call they answer, so a script never needs to know the
input in advance:

  allow           {"behavior":"allow","updatedInput":<input unchanged>}
  always-allow    allow + "updatedPermissions" with a session rule for the call
  deny:<message>  {"behavior":"deny","message":<message>}
  answer-first    allow, the input plus "answers": every question's first option, the first two
                  for a multi-select question (joined by ", ")

Usage: prompt-stub.py --port-file <path> --log <path> [--answer <name>]...

Standard library only. Used by scripts/capture-permission-prompt.sh; never by CI.
"""

import argparse
import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TOOL = {
    "name": "permission_prompt",
    "description": "Asks the user of the Micold window whether a tool use may run.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "tool_name": {"type": "string"},
            "input": {"type": "object"},
            "tool_use_id": {"type": "string"},
        },
        "required": ["tool_name", "input"],
    },
}

# The input field an always-allow rule of each tool names (data-model *AllowRule*).
RULE_FIELD = {
    "Bash": "command",
    "Edit": "file_path",
    "MultiEdit": "file_path",
    "Write": "file_path",
    "NotebookEdit": "notebook_path",
}


def allow(call_input):
    return {"behavior": "allow", "updatedInput": call_input}


def always_allow(tool_name, call_input):
    rule = {"toolName": tool_name}
    field = RULE_FIELD.get(tool_name)
    if field and isinstance(call_input.get(field), str):
        rule["ruleContent"] = call_input[field]
    answer = allow(call_input)
    answer["updatedPermissions"] = [
        {"type": "addRules", "rules": [rule], "behavior": "allow", "destination": "session"}
    ]
    return answer


def answer_first(call_input):
    answers = {}
    for question in call_input.get("questions") or []:
        labels = [option.get("label", "") for option in question.get("options") or []]
        chosen = labels[:2] if question.get("multiSelect") else labels[:1]
        answers[question.get("question", "")] = ", ".join(chosen)
    updated = dict(call_input)
    updated["answers"] = answers
    return allow(updated)


def build_answer(name, tool_name, call_input):
    if name == "allow":
        return allow(call_input)
    if name == "always-allow":
        return always_allow(tool_name, call_input)
    if name.startswith("deny:"):
        return {"behavior": "deny", "message": name[len("deny:"):]}
    if name == "answer-first":
        return answer_first(call_input)
    raise ValueError(f"unknown answer {name!r}")


class Stub:
    def __init__(self, answers, log_path):
        self.answers = list(answers)
        self.log_path = log_path
        self.lock = threading.Lock()

    def call(self, params):
        arguments = params.get("arguments") or {}
        with self.lock:
            name = self.answers.pop(0) if self.answers else "allow"
            answer = build_answer(
                name, arguments.get("tool_name", ""), arguments.get("input") or {}
            )
            with open(self.log_path, "a", encoding="utf-8") as log:
                record = {"arguments": arguments, "_meta": params.get("_meta"), "answer": answer}
                log.write(json.dumps(record, ensure_ascii=False) + "\n")
        text = json.dumps(answer, ensure_ascii=False, separators=(",", ":"))
        return {"content": [{"type": "text", "text": text}], "isError": False}

    def reply(self, message):
        """The JSON-RPC reply to one message, or None for a notification."""
        if "id" not in message:
            return None
        method, params = message.get("method"), message.get("params") or {}
        if method == "initialize":
            result = {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "micold", "version": "capture-stub"},
            }
        elif method == "tools/list":
            result = {"tools": [TOOL]}
        elif method == "tools/call":
            if params.get("name") != TOOL["name"]:
                return {"jsonrpc": "2.0", "id": message["id"],
                        "error": {"code": -32602, "message": "unknown tool"}}
            result = self.call(params)
        elif method == "ping":
            result = {}
        else:
            return {"jsonrpc": "2.0", "id": message["id"],
                    "error": {"code": -32601, "message": f"method not found: {method}"}}
        return {"jsonrpc": "2.0", "id": message["id"], "result": result}


def handler_for(stub):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *_):
            pass

        def send(self, status, body=b"", content_type="application/json"):
            self.send_response(status)
            if body:
                self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            length = int(self.headers.get("Content-Length") or 0)
            try:
                message = json.loads(self.rfile.read(length) or b"null")
            except ValueError:
                self.send(400)
                return
            batch = isinstance(message, list)
            replies = [r for r in map(stub.reply, message if batch else [message]) if r]
            if not replies:
                self.send(202)
                return
            body = json.dumps(replies if batch else replies[0]).encode()
            self.send(200, body)

        def do_GET(self):
            # No server-to-client stream: the CLI falls back to POST replies.
            self.send(405)

        def do_DELETE(self):
            self.send(200)

    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--port-file", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--answer", action="append", default=[])
    args = parser.parse_args()
    for name in args.answer:
        build_answer(name, "", {})  # refuse an unknown name before the CLI starts
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler_for(Stub(args.answer, args.log)))
    with open(args.port_file, "w", encoding="utf-8") as port_file:
        port_file.write(str(server.server_address[1]))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())

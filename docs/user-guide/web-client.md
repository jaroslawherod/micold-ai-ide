# The web client and the local stack

The web client shows your daemons and their sessions in a browser, from a real orchestrator, in
English or Polish. Use it to watch sessions and read what they said; it does not run sessions.

## Start everything on this computer

```sh
mise run local-stack
```

This starts, on this computer only:

| What | Address |
|---|---|
| The orchestrator's client API | `ws://127.0.0.1:8730/api/ws` |
| The web client | `http://127.0.0.1:8731` |
| One daemon, linked to the orchestrator | no address; it dials the orchestrator |

The daemon serves a small demo project with a second worktree and two sessions, kept in a private
temporary directory that is removed when the stack stops. Open `http://127.0.0.1:8731` in a browser,
choose **Sign in with Local orchestrator**, and the daemon, its project, the worktrees and both
sessions are listed. Press `Ctrl+C` in the terminal to stop the orchestrator, the daemon and the web
server together.

If another copy of the stack is running, the command names the port in use and exits. To use other
ports, set `MICOLD_LOCAL_API_PORT` and `MICOLD_LOCAL_WEB_PORT` before the command.

## Reading a session

Select a session in the list to read its content: the messages and tool calls the daemon holds.
The content is a snapshot taken when you open it. Choose **Refresh** to read it again; it does not
update by itself. A long entry may be shortened, and the client says so. Terminal sessions have no
readable content here.

## The connection

The list follows the orchestrator live: a session created, renamed or ended appears within about two
seconds, with no reload.

- **Connecting to the orchestrator**: the first connection is opening.
- **Connection lost, reconnecting (attempt N)**: the client retries with growing pauses and picks up
  where it left off, so no change is missed and none is shown twice. If the orchestrator restarted
  or the gap is too large, the client loads the whole list again instead.
- **Cannot reach the orchestrator**: after eight failed attempts the client stops and shows
  **Retry**. Check that the orchestrator is running, then choose **Retry**.

A session whose daemon cannot be reached stays in the list marked **Unreachable**, and returns to
normal when the daemon reconnects.

## Which pages may connect

The orchestrator accepts the API connection only from the web addresses it is told to allow. The
local stack allows its own web client. For another setup, start the orchestrator with
`--api-addr 127.0.0.1:<port>` and one `--allow-origin <scheme>://<host>:<port>` for each page
address (or the `MICOLD_ORCHESTRATOR_API_ADDR` and `MICOLD_ORCHESTRATOR_ALLOW_ORIGINS` variables, the
latter comma separated). A page from any other address is refused. The API address must be a
loopback address until encrypted connections are supported.

## Pointing the client elsewhere

The web client connects to the orchestrator address set when it was built:
`EXPO_PUBLIC_ORCHESTRATOR_URL`, which defaults to `ws://127.0.0.1:8730/api/ws`. Set it before
`mise run client-web` to build for another address.

## Trying the client without an orchestrator

`mise run client-web-mock` starts the client against built-in sample data, with no orchestrator. It
is for design and testing work, not for your real sessions.

## Language

Choose English or Polish with the language picker at the top; every message of the connection and
session views is translated.

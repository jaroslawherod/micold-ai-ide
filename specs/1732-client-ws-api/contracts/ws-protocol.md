# Contract: `/api/ws` (#1732)

Text frames, one JSON object each. Binary frames from a client get `error {code: "malformed"}`; the server never
sends binary (FR-006).

## Upgrade
`GET /api/ws` with `Origin`. Foreign origin: `403`, no upgrade, logged `refused origin <origin>` at WARN. No `Origin`:
accepted for a loopback peer only. Other paths: `404`.

## Requests and replies
```json
{"id":1,"type":"list_daemons"}
{"id":1,"type":"daemons","rows":[{"id":"d1","name":"local","status":"online"}]}
{"id":2,"type":"read_session","daemon":"d1","project":"p1","session":"s1"}
{"id":2,"type":"session_content","session":"s1","entries":[{"seq":1,"role":"user","kind":"text","text":"hi"}]}
{"id":3,"type":"subscribe","since":{"epoch":77,"seq":120}}
{"id":3,"type":"subscribed","epoch":77,"seq":120}
{"type":"changes","epoch":77,"changes":[{"seq":121,"kind":"session_state","session_id":"s1","state":"idle"}]}
{"id":3,"type":"resync_needed","epoch":91,"seq":4,"reason":"epoch_changed"}
```
A reply echoes the request `id`. `subscribe` without `since` replies `subscribed` with a `snapshot`. After `resync_needed`
the client sends `subscribe` with no `since`. Missed changes arrive in order, each `seq` once, before any newer one.

## Heartbeat
Client `{"id":9,"type":"ping"}` every 20 s; server `{"id":9,"type":"pong"}`. Silence for 40 s: either side closes
and the client reconnects.

## Errors
`{"id":N,"type":"error","code":"unknown_type","message":"no request type 'foo'"}`; the connection stays open
(FR-008). `read_session` on a terminal session: `terminal_not_supported` ("terminal output is not offered on this API").

## Close codes
Server closes 1008 on policy (not after upgrade: origin is a 403), 1011 on internal failure; clients treat 1001, 1006,
1011 as reconnect-and-resume and 1008 as unreachable.

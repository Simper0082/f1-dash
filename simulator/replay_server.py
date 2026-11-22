#!/usr/bin/env python3
"""Replay an f1-dash recording (simulator `save` format) over a SignalR-style websocket.

Line 1 of the file is the initial state, the rest are raw feed messages.
Answers the SignalR handshake and the Subscribe call the way the real feed does,
then streams the recorded messages.  Usage: replay_server.py FILE [delay_ms] [port]
"""
import asyncio, json, sys
import websockets

RS = "\x1e"
path = sys.argv[1]
delay = float(sys.argv[2]) / 1000 if len(sys.argv) > 2 else 0.1
port = int(sys.argv[3]) if len(sys.argv) > 3 else 8765
lines = [l.rstrip("\n").rstrip(RS) for l in open(path) if l.strip()]
initial, updates = lines[0], lines[1:]

async def handler(ws):
    async def read_msgs():
        data = await ws.recv()
        return [m for m in data.split(RS) if m.strip()]
    await read_msgs()                    # protocol handshake
    await ws.send("{}" + RS)
    invocation = None
    while invocation is None:            # wait for Subscribe
        for m in await read_msgs():
            o = json.loads(m)
            if o.get("target") == "Subscribe":
                invocation = o["invocationId"]
    await ws.send(json.dumps({"type": 3, "invocationId": invocation, "result": json.loads(initial)}) + RS)
    for u in updates:
        if u.startswith('{"type":1,"target":"feed","arguments":["SessionInfo"'):
            continue                      # realtime restarts on SessionInfo; skip it in replays
        await ws.send(u + RS)
        await asyncio.sleep(delay)
    await ws.close()

async def main():
    async with websockets.serve(handler, "127.0.0.1", port, max_size=None):
        await asyncio.Future()
asyncio.run(main())

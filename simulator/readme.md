# simulator

the simulator is a development tool to simulate the f1 singalr websocket endpoint
its not really made to record and replay races or sessions for that the dedicated service is more suited

## commands

save - save the current live data to a file

replay - replay the recorded data from a file

## replay_server.py

The Rust `replay` command does not answer the SignalR handshake that `realtime` expects now. `replay_server.py` replays a recorded file (the `save` format) the way the real feed behaves:

```
pip install websockets
python3 simulator/replay_server.py recording.txt 100 8765   # file, delay per message in ms, port
F1_DEV_URL=ws://127.0.0.1:8765 ./target/debug/realtime
```

It skips `SessionInfo` updates in the file, because `realtime` restarts when it sees one. With `F1_DEV_URL` set, the signalr client skips the real negotiate call, so this works offline.

# Deploying f1-dash on Proxmox

Runs the dashboard, the realtime feed and the API as one Docker stack inside an LXC, reachable from
outside the house through a Cloudflare Tunnel (no router ports opened). The wheel screen pulls its
data from `/api/wheel`.

## 1. Create the LXC (on the Proxmox host)

Copy `create-lxc.sh` to the host and run it as root. It makes an unprivileged Debian 12 LXC with
nesting and keyctl enabled, installs Docker, and clones this repo to `/opt/f1-dash`.

```
./create-lxc.sh <free-id>            # e.g. ./create-lxc.sh 502; with no ID it uses the next free one
STORAGE=mypool ./create-lxc.sh       # to use a different storage (default local-lvm)
```

Defaults: 2 cores, 4 GB RAM, 24 GB disk. The first image build compiles Rust and Next.js and needs the
RAM; you can lower it afterwards.

## 2. Cloudflare Tunnel (once)

The domain needs to be on Cloudflare DNS. In Zero Trust > Networks > Tunnels create a tunnel, then add
two public hostnames:

| Hostname | Service |
|---|---|
| `f1.example.com` | `http://web:3000` |
| `f1-live.example.com` | `http://realtime:80` |

Use your own domain in place of `example.com`. Copy the tunnel token into `/opt/f1-dash/deploy/.env` as
`TUNNEL_TOKEN`, and set `LIVE_URL` and `ORIGINS` there to your hostnames. `.env` is git-ignored; never commit it.

## 3. Start it

```
cd /opt/f1-dash/deploy
docker compose --profile tunnel up -d --build
```

Without `--profile tunnel` the stack runs LAN-only.

## 4. Check it

- Dashboard: `https://f1.example.com` (LAN: `http://<lxc-ip>:3000`)
- Wheel preview: `https://f1.example.com/wheel?driver=1`
- Board data: `https://f1-live.example.com/api/wheel?driver=1` (LAN: `http://<lxc-ip>:4000/api/wheel?driver=1`)

Outside a session the endpoint answers with `"live": false` and nulls. The board should show its idle
screen in that case.

## `/api/wheel`

One small JSON object for one driver (`?driver=<racing number>`, default 3), about 600 bytes:
session name and location, track and session status, air and track temperature (always °C, convert on
the board), driver tla and team colour, position, number of cars, lap and total laps, tyre compound
and age, pit stops, last and best lap, interval, and best-lap delta to P1.

## Notes

- `ORIGINS` and `LIVE_URL` in `.env` must match your hostnames. Add your LAN address to `ORIGINS` if you
  open the dashboard from `http://<lxc-ip>:3000`.
- The live data endpoints are public read-only. If you want to lock them to the board, add a Cloudflare
  WAF or Access rule on `f1-live.example.com`.
- Update: `cd /opt/f1-dash && git pull && cd deploy && docker compose --profile tunnel up -d --build`.
- Not tested on real Proxmox hardware or with a Docker build in the authoring environment (no Docker
  daemon there); the Rust crate builds and its tests pass.

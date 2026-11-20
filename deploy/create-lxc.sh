#!/usr/bin/env bash
# Run on the Proxmox host (as root). Creates a Docker-ready LXC and installs this stack in it.
# Usage: ./create-lxc.sh [CTID]
set -euo pipefail

CTID="${1:-$(pvesh get /cluster/nextid)}"
HOSTNAME="${HOSTNAME_OVERRIDE:-f1-dash}"
STORAGE="${STORAGE:-local-lvm}"   # Proxmox storage for the container disk (override: STORAGE=mypool ./create-lxc.sh)
DISK_GB="${DISK_GB:-24}"
CORES="${CORES:-2}"
RAM_MB="${RAM_MB:-4096}"            # the Rust/Next builds need ~3-4 GB; this can be lowered afterwards
BRIDGE="${BRIDGE:-vmbr0}"
REPO="${REPO:-https://github.com/Simper0082/f1-dash.git}"

echo "Creating LXC $CTID ($HOSTNAME) on $STORAGE"

pveam update >/dev/null
TEMPLATE="$(pveam available --section system | awk '/debian-12-standard/ {print $2}' | sort -V | tail -1)"
[ -n "$TEMPLATE" ] || { echo "No Debian 12 template found"; exit 1; }
if ! pveam list local | grep -q "$TEMPLATE"; then
  pveam download local "$TEMPLATE"
fi

pct create "$CTID" "local:vztmpl/$TEMPLATE" \
  --hostname "$HOSTNAME" \
  --cores "$CORES" --memory "$RAM_MB" --swap 1024 \
  --rootfs "$STORAGE:$DISK_GB" \
  --net0 "name=eth0,bridge=$BRIDGE,ip=dhcp" \
  --unprivileged 1 \
  --features nesting=1,keyctl=1 \
  --onboot 1 \
  --start 1

echo "Waiting for the container to get a network address..."
for _ in $(seq 1 30); do
  pct exec "$CTID" -- getent hosts deb.debian.org >/dev/null 2>&1 && break
  sleep 2
done

pct exec "$CTID" -- bash -lc "
  set -e
  apt-get update
  apt-get install -y curl git ca-certificates
  curl -fsSL https://get.docker.com | sh
  git clone '$REPO' /opt/f1-dash
  cp /opt/f1-dash/deploy/.env.example /opt/f1-dash/deploy/.env
"

IP="$(pct exec "$CTID" -- hostname -I | awk '{print $1}')"
cat <<MSG

LXC $CTID is up at $IP.
Next:
  1. pct exec $CTID -- nano /opt/f1-dash/deploy/.env     (paste your TUNNEL_TOKEN)
  2. pct exec $CTID -- bash -lc 'cd /opt/f1-dash/deploy && docker compose --profile tunnel up -d --build'
     (first build takes a while: Rust and Next.js both compile from source)
  3. Test on your LAN:  http://$IP:3000/wheel   and   http://$IP:4000/api/wheel?driver=3
MSG

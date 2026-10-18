# A cheap remote daemon on a private Tailscale network

To run the session service on a VPS (a small Hetzner or Contabo VM is enough) without exposing any
public port, put the VPS on a [Tailscale](https://tailscale.com) network (a *tailnet*) and add it
to Micold as a [daemon over SSH](remote-daemons.md). Micold's remote daemon already listens on no
network port: the app starts it over an SSH session and talks through that session. Tailscale
supplies the other half, an encrypted private path to the SSH server, so port 22 need not be
reachable from the internet.

## What you need

- A Linux VPS (`x86_64` or `aarch64`) you can log in to as root once.
- A Tailscale account, and this computer joined to the same tailnet.
- A Tailscale **auth key** from the admin console (Settings > Keys). A single-use, expiring key is
  enough.

## Set up the VPS

Copy [`scripts/remote-daemon-tailscale-setup.sh`](../../scripts/remote-daemon-tailscale-setup.sh)
to the VPS, put the auth key in a file there, and run it as root:

```sh
umask 077; printf '%s' 'tskey-auth-...' > /root/ts.key
./remote-daemon-tailscale-setup.sh --authkey-file /root/ts.key --hostname micold-vps --lock-down
shred -u /root/ts.key
```

It creates an unprivileged `micold` account, installs Tailscale, joins the tailnet with **Tailscale
SSH** on (your tailnet's access rules decide who may log in; no SSH key to copy), and creates
`/home/micold/.micold/api-keys.env`. Use `--dry-run` first to see the plan, `--user` to pick another
account name.

`--lock-down` makes `ufw` refuse all incoming traffic except on the `tailscale0` interface. Check
that you can reach the host over the tailnet (`ssh micold@micold-vps` from this computer) **before**
you rely on it, and keep your provider's console at hand: lock-down closes public SSH. Without
`ufw`, close the public ports in your provider's firewall instead.

## Add it to Micold

1. From this computer, run `ssh micold@micold-vps` once to confirm it connects.
2. Open **Settings > Daemons**, choose **Add daemon**, set **Runs in** to **Remote over SSH**, and
   enter the tailnet name (`micold-vps`) and user (`micold`).

Micold copies its own daemon to `~/.micold/remote/` on the VPS, as for any [SSH
daemon](remote-daemons.md#what-happens-when-it-starts). A repository on the VPS is added as
described in [that page](remote-daemons.md#adding-a-repository-that-lives-on-the-host).

## Give the AI CLIs their API keys

Edit `/home/micold/.micold/api-keys.env` on the VPS and fill in the keys, for example:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
```

The file is mode 600 and readable only by the `micold` account. Then, in Settings, set the
environment-include script of the worktrees bound to this daemon to source that file (the "source
my startup file" setting, see [Settings](settings.md)), so every session, including a respawn, gets
the keys. Alternatively `claude` can sign in on the VPS once from a terminal session.

## Checklist

- [ ] The VPS shows up in `tailscale status` on this computer.
- [ ] `ssh micold@micold-vps` works, and `ssh micold@<public ip>` is refused or times out.
- [ ] The daemon's row shows **Connected** with the tailnet name.
- [ ] A new session sees the key: run `printenv ANTHROPIC_API_KEY` in a terminal pane.

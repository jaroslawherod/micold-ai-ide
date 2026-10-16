# Quickstart (805)

## Part A: automated
`mise run test-core`, then `mise run gate`.

## Part B: visual pass
1. Settings > Daemons > Add daemon, runtime Container, leave Port empty: the field hint says a free
   port is chosen. Save: no port error.
2. Edit that daemon: the Port field shows a number. Clear it and save: the number is unchanged.
3. Add a second container daemon with an empty port: a different number.
4. Type `0` as the port: "Enter a port between 1 and 65535."
5. Occupy a daemon's port (`python3 -m http.server <port> --bind 127.0.0.1`), start the daemon: the
   error names the port and says to change it in Settings > Daemons.

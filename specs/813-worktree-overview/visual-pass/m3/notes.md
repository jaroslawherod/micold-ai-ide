# M3 visual pass (B6, B7, B11): NOT RUN

Display tooling is incomplete: Xvfb and import (ImageMagick) are present, but xdotool, sway, grim,
xte, ydotool and python Xlib are not. Without input injection the Ctrl+Shift+O toggle (B7), the
disabled button in Settings (B11) and the permission grant that turns the badge red to yellow (B6)
cannot be driven. No screenshots were taken and no results are claimed.
A full gate build also held the cargo build lock, so the client was not built.

#!/bin/bash

# Script to configure Percolist as a system panel

echo "Configuring Percolist as system panel..."

# Wait for window to appear
sleep 2

# Find Percolist window ID
WINDOW_ID=$(xdotool search --name "Percolist" | head -1)

if [ -z "$WINDOW_ID" ]; then
    echo "Percolist window not found!"
    exit 1
fi

echo "Found Percolist window ID: $WINDOW_ID"

# Set window type as dock
xprop -id $WINDOW_ID -f _NET_WM_WINDOW_TYPE 32a -set _NET_WM_WINDOW_TYPE _NET_WM_WINDOW_TYPE_DOCK

# Make window sticky (appear on all desktops)
xprop -id $WINDOW_ID -f _NET_WM_DESKTOP 32c -set _NET_WM_DESKTOP 0xFFFFFFFF

# Set strut to reserve 32px at top
xprop -id $WINDOW_ID -f _NET_WM_STRUT_PARTIAL 32c -set _NET_WM_STRUT_PARTIAL 0,0,32,0,0,0,0,0,0,1919,0,0

# Alternative strut method
xprop -id $WINDOW_ID -f _NET_WM_STRUT 32c -set _NET_WM_STRUT 0,0,32,0

# Make window sticky with wmctrl
wmctrl -i -r $WINDOW_ID -b add,sticky

# Position window at top
wmctrl -i -r $WINDOW_ID -e 0,0,0,1920,32

echo "Percolist configured as system panel!"
echo "Other applications should now appear 32px below the top of the screen."

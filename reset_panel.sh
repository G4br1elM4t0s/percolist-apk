#!/bin/bash

# Script to reset Percolist panel configuration and fix any issues

echo "Resetting Percolist panel configuration..."

# Kill any remaining Percolist processes
pkill -f "Percolist"
pkill -f "target/debug/app"

# Find any Percolist windows and reset their properties
WINDOW_IDS=$(xdotool search --name "Percolist" 2>/dev/null || true)

for WINDOW_ID in $WINDOW_IDS; do
    if [ ! -z "$WINDOW_ID" ]; then
        echo "Resetting window ID: $WINDOW_ID"

        # Remove dock type
        xprop -id $WINDOW_ID -remove _NET_WM_WINDOW_TYPE 2>/dev/null || true

        # Remove struts
        xprop -id $WINDOW_ID -remove _NET_WM_STRUT 2>/dev/null || true
        xprop -id $WINDOW_ID -remove _NET_WM_STRUT_PARTIAL 2>/dev/null || true

        # Remove desktop sticky
        xprop -id $WINDOW_ID -remove _NET_WM_DESKTOP 2>/dev/null || true

        # Close the window
        wmctrl -i -c $WINDOW_ID 2>/dev/null || true
    fi
done

# Restart window manager to clear any cached struts
echo "Restarting window manager..."

# Try different window managers
if pgrep -x "gnome-shell" > /dev/null; then
    echo "Restarting GNOME Shell..."
    busctl --user call org.gnome.Shell /org/gnome/Shell org.gnome.Shell Eval s 'Meta.restart("Restarting...")' 2>/dev/null || true
elif pgrep -x "openbox" > /dev/null; then
    echo "Restarting Openbox..."
    openbox --restart 2>/dev/null || true
elif pgrep -x "xfwm4" > /dev/null; then
    echo "Restarting XFCE window manager..."
    xfwm4 --restart 2>/dev/null || true
else
    echo "Window manager not recognized, trying generic restart..."
fi

echo "Panel configuration reset complete!"
echo "You may need to restart your desktop session if issues persist."

#!/bin/bash
# update_path.sh
# Usage: sudo ./update_path.sh <base_name> <env_file_path>
# Example: sudo ./update_path.sh myreview /path/to/review/.env

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <base_name> <env_file_path>"
    exit 1
fi

base_name="$1"
env_file="$2"
path_file="/etc/systemd/system/${base_name}.path"

# Source the environment file to load MONITOR_DIR.
# Make sure the .env file uses syntax that bash can understand.
if [ ! -f "$env_file" ]; then
    echo "Environment file $env_file not found."
    exit 1
fi

source "$env_file"

# Check that MONITOR_DIR is set.
if [ -z "$MONITOR_DIR" ]; then
    echo "MONITOR_DIR is not set in $env_file"
    exit 1
fi

# Create or update the .path unit file with the new directory.
cat <<EOF > "$path_file"
[Unit]
Description=Monitor directory \$MONITOR_DIR for changes

[Path]
PathModified=$MONITOR_DIR
Unit=${base_name}.service

[Install]
WantedBy=multi-user.target
EOF

echo "Updated $path_file with MONITOR_DIR = $MONITOR_DIR"

# Reload systemd daemon and restart the .path unit.
systemctl daemon-reload
systemctl restart ${base_name}.path
echo "Systemd daemon reloaded and ${base_name}.path restarted."

#!/bin/bash
# combined_install.sh
# This script creates systemd unit files based on user's choice of activation method:
# 1. Manual activation (regular service)
# 2. Time-based (specific time of day)
# 3. Interval-based (every X minutes/hours)
# 4. Path-based (when a directory changes)
#
# It reads the binary name from Cargo.toml, ensures it’s built in target/release,
# and sources .env for DIRECTORY when needed.
#
# Usage: sudo ./combined_install.sh

# Ensure the script is run as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (e.g., sudo $0)"
    exit 1
fi

# Get the project directory (current directory)
project_root=$(pwd)

# Parse the binary name from Cargo.toml
cargo_toml="${project_root}/Cargo.toml"
if [ ! -f "$cargo_toml" ]; then
    echo "Cargo.toml not found at $cargo_toml"
    exit 1
fi

# Extract the package name from Cargo.toml
binary_name=$(grep -m1 '^name' "$cargo_toml" | head -n1 | sed -E 's/.*= *["'\'']([^"'\'']+)["'\''].*/\1/')
if [ -z "$binary_name" ]; then
    echo "Could not extract package name from Cargo.toml"
    exit 1
fi

binary_path="${project_root}/target/release/${binary_name}"
if [ ! -x "$binary_path" ]; then
    echo "Binary not found or not executable at $binary_path"
    echo "Run: cargo build --release"
    exit 1
fi

# Use project root as the working directory for all services
working_dir="$project_root"

# Function: create a simple oneshot service unit
create_base_service() {
    local service_file=$1
    local description=$2
    local exec_start=$3

    cat <<EOF > "$service_file"
[Unit]
Description=$description
After=network.target

[Service]
Type=oneshot
WorkingDirectory=${working_dir}
# load .env so DIRECTORY is in the environment
EnvironmentFile=${working_dir}/.env
ExecStart=${exec_start}

[Install]
WantedBy=multi-user.target
EOF
}

# Prompt helpers
get_valid_input() {
    local prompt=$1 min=$2 max=$3
    local input
    while true; do
        read -p "$prompt" input
        if [[ "$input" =~ ^[0-9]+$ ]] && (( input>=min && input<=max )); then
            echo "$input"; return
        fi
        echo "Enter a number between $min and $max."
    done
}

get_valid_time() {
    local prompt=$1
    local re='^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$'
    local input
    while true; do
        read -p "$prompt" input
        [[ $input =~ $re ]] && { echo "$input"; return; }
        echo "Time must be HH:MM:SS."
    done
}

get_interval_unit() {
    local prompt=$1 input
    while true; do
        read -p "$prompt" input
        [[ $input == "m" || $input == "h" ]] && { echo "$input"; return; }
        echo "Enter 'm' for minutes or 'h' for hours."
    done
}

get_integer() {
    local prompt=$1 input
    while true; do
        read -p "$prompt" input
        [[ $input =~ ^[0-9]+$ ]] && { echo "$input"; return; }
        echo "Enter a positive integer."
    done
}

# Choose mode
echo "Choose configuration type:"
echo " 1) Manual service"
echo " 2) Daily at specific time"
echo " 3) Every N minutes/hours"
echo " 4) On directory change"
choice=$(get_valid_input "Enter 1-4: " 1 4)

# Get unit name
while true; do
    read -p "Unit name (e.g., review): " unit_name
    [[ -n "$unit_name" ]] && break
    echo "Name cannot be empty."
done

case $choice in
  1)  # Manual
    svc="/etc/systemd/system/${unit_name}.service"
    cmd="/usr/bin/systemd-inhibit --what=sleep --mode=block ${binary_path}"
    create_base_service "$svc" "Run ${binary_name} manually" "$cmd"
    echo "Created $svc"
    echo "-> sudo systemctl start ${unit_name}.service"
    ;;
  2)  # Daily
    tod=$(get_valid_time "Time (HH:MM:SS): ")
    svc="/etc/systemd/system/${unit_name}.service"
    cmd="/usr/bin/systemd-inhibit --what=sleep --mode=block ${binary_path}"
    create_base_service "$svc" "Run ${binary_name} daily" "$cmd"
    tmr="/etc/systemd/system/${unit_name}.timer"
    cat <<EOF > "$tmr"
[Unit]
Description=Timer for ${unit_name}.service at ${tod} daily

[Timer]
OnCalendar=*-*-* ${tod}
Persistent=true

[Install]
WantedBy=timers.target
EOF
    echo "Created $svc and $tmr"
    echo "-> sudo systemctl enable --now ${unit_name}.timer"
    ;;
  3)  # Interval
    unit=$(get_interval_unit "Interval in minutes or hours? (m/h): ")
    val=$(get_integer "Interval value: ")
    [[ $unit == m ]] && dur="${val}min" || dur="${val}h"
    svc="/etc/systemd/system/${unit_name}.service"
    cmd="/usr/bin/systemd-inhibit --what=sleep --mode=block ${binary_path}"
    create_base_service "$svc" "Run ${binary_name} every ${dur}" "$cmd"
    tmr="/etc/systemd/system/${unit_name}.timer"
    cat <<EOF > "$tmr"
[Unit]
Description=Timer for ${unit_name}.service every ${dur}

[Timer]
OnBootSec=10min
OnUnitActiveSec=${dur}
Persistent=true

[Install]
WantedBy=timers.target
EOF
    echo "Created $svc and $tmr"
    echo "-> sudo systemctl enable --now ${unit_name}.timer"
    ;;
  4)  # Path
    envf="${project_root}/.env"
    [[ -f "$envf" ]] || { echo ".env not found"; exit 1; }
    source "$envf"
    [[ -n "$DIRECTORY" ]] || { echo "DIRECTORY not set in .env"; exit 1; }
    esc=$(systemd-escape --path "$DIRECTORY")
    svc="/etc/systemd/system/${unit_name}.service"
    cmd="/usr/bin/systemd-inhibit --what=sleep --mode=block ${binary_path}"
    create_base_service "$svc" "Run ${binary_name} on changes" "$cmd"
    pth="/etc/systemd/system/${unit_name}.path"
    cat <<EOF > "$pth"
[Unit]
Description=Watch ${DIRECTORY} for changes

[Path]
PathModified=${esc}
Unit=${unit_name}.service

[Install]
WantedBy=multi-user.target
EOF
    echo "Created $svc and $pth"
    echo "-> sudo systemctl enable --now ${unit_name}.path"
    ;;
esac

# Reload systemd
echo "Reloading systemd…"
systemctl daemon-reload

echo "Done."

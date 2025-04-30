## 🛠 Systemd Integration

Running an AI code review can take minutes—or even hours—and you don’t want it tying up your terminal or interrupting other tasks. On Linux, we use **systemd** to run reviews as a **background process**, so they can work quietly on their own.

## Unit Files

systemd uses simple text files called **units**, stored in `/etc/systemd/system/`:

1. **`.service`**  
   - Defines **what** to run: your compiled review binary, wrapped in `systemd-inhibit` so the machine doesn’t sleep mid-review.  
   - Loads your `.env` (API key and `DIRECTORY`) into the environment.

2. **`.timer`**  
   - Defines **when** to run a `.service`:  
     - **OnCalendar=** for specific times (e.g. `03:00:00` daily)  
     - **OnUnitActiveSec=** for intervals (e.g. every 6h)  
   - Use `Persistent=true` to catch up missed runs after downtime.

3. **`.path`**  
   - Defines **which** folder or file changes trigger a `.service` immediately (via inotify).  
   - Great for event-driven reviews as soon as code is updated.  
   > ⚠️ **Path units don’t support spaces** in directory names.

### Limitations on WSL
- **`.timer`** and **`.path`** units may or may not fire reliably under Windows Subsystem for Linux (WSL).  
- The **`.service`** unit itself works fine on WSL, so manual runs still work.  
- For full timer/path support, use a **native Linux** or VM.

---

## One-Step Installation

Instead of hand-editing unit files (which requires `sudo nano …` or `sudo vim …`), run our installer:

Make sure the DIRECTORY variable in the .env file is not empty and is a real path.
Make sure the is a compiled binary in the /target/release folder.
If theres not, or if you're not sure, make one with: cargo build --release

cd /path/to/your/project
sudo bash install.sh

1. Manual Service

Creates a service that runs on demand. 
Use when you want to manually trigger reviews. 
Run with: sudo systemctl start unit-name.service

2. Daily Timer

Runs at a specific time each day. 
Format: HH:MM (24-hour/military time). 
Example: 21:00:00 runs at 9:00 PM daily. 
Starts on the next occurrence of the specified time. 
Use for scheduled daily code reviews. 
Enabled with: sudo systemctl enable unit-name.timer. 
If user wants the first run to be after enabling,
use: sudo systemctl enable --now unit-name.timer

3. Interval Timer

Runs repeatedly at fixed intervals. 
Choose minutes (m) or hours (h). 
Example: Setting "2h" runs every 2 hours. 
First run occurs after boot, then follows interval. 
Use for continuous review cycles. 
Enabled with: sudo systemctl enable unit-name.timer. 
If user wants the first run to be after enabling,
use: sudo systemctl enable --now unit-name.timer

4. Directory Watch

Monitors a directory for changes. 
Automatically runs when files are created/modified. 
Directory path is taken from DIRECTORY in your .env file. 
If path is changed in .env file, user must also change the .path file manually. 
Use for real-time reviews as you update code. 
⚠️ Path cannot contain spaces. 
Enabled with: sudo systemctl enable unit-name.path. 
If user wants the first run to be after enabling,
use: sudo systemctl enable --now unit-name.path

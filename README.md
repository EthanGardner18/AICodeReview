## 🛠 Systemd Integration

Running an AI code review can take minutes—or even hours—and you don’t want it tying up your terminal or interrupting other tasks. On Linux, we use **systemd** to run reviews as a **background process**, so they can work quietly on their own.

### What’s a background process?
A background process runs without keeping a terminal window open. You can close your terminal, keep coding, or even log out—your review keeps running.

### What is systemd?
systemd is the built-in “service manager” on most Linux distributions. It can:
- **Start** programs automatically at boot  
- **Schedule** tasks by clock time or interval  
- **Watch** folders for file changes and react immediately  
- **Keep** all logs in one place (`journalctl`)  
- **Restart** services if they crash  

---

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
- **`.timer`** and **`.path`** units often don’t fire reliably under Windows Subsystem for Linux (WSL).  
- The **`.service`** unit itself works fine on WSL, so manual runs still work.  
- For full timer/path support, use a **native Linux** or VM.

---

## One-Step Installation

Instead of hand-editing unit files (which requires `sudo nano …` or `sudo vim …`), run our installer:

cd /path/to/your/project
sudo bash install.sh

You’ll be prompted to choose one of four modes:

1. Manual
Installs only the .service (run on demand).

2. Specific time
Installs a .service plus a daily .timer.

3. Interval
Installs a .service plus a repeating .timer.

4. Directory watch
Installs a .service plus a .path unit (event-driven).

All unit files land in /etc/systemd/system/ and require root privileges.
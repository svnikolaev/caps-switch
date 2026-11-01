## v0.4.0

### Added
- **Check for updates** in the tray menu: queries the latest GitHub release, compares
  versions and — after confirmation — downloads the new `caps-switch.exe`, replaces it
  after the process exits and restarts automatically.
- **About** item in the tray menu: version, purpose, GitHub link, license.
- CLI `--check-updates` (prints the latest release tag and exits) — handy for scripts and tests.

### Notes
- Update check uses `curl` (built into Windows 10/11) against
  `api.github.com/repos/svnikolaev/caps-switch/releases/latest`; no extra dependencies.
- The installer (v0.3.2+) offers an explicit autostart choice: normal (HKCU Run) or
  elevated (Task Scheduler with highest privileges).

### Install
```powershell
# single-file installer (recommended)
caps-switch-setup-0.4.0.exe

# or script
irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1 | iex
```

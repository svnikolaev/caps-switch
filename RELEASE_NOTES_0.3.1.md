## v0.3.1

### Changed
- **Tray icon redesign**: dark graphite keycap with a **recessed (inset) top** — the inner
  pad has an inner shadow at the top edge and a light rim at the bottom, so the key reads
  as pressed/concave. Two light arrows sit inside the recess.
- Still a 3D keycap: near-black side wall + top bezel, 7 sizes (16–256 px).
- `tools/make-icon.ps1` regenerates it (System.Drawing, no external tools).

### Install
```powershell
caps-switch.exe --install              # normal privileges
caps-switch.exe --install --elevated   # highest privileges (works in elevated windows too)
```

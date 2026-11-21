# SPDX-License-Identifier: GPL-3.0-or-later
param([int]$X, [int]$Y)
Add-Type 'using System; using System.Runtime.InteropServices; public class C { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e); }'
[C]::SetProcessDPIAware() | Out-Null
[C]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 150
[C]::mouse_event(0x2,0,0,0,[UIntPtr]::Zero); [C]::mouse_event(0x4,0,0,0,[UIntPtr]::Zero)

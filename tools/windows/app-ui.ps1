# SPDX-License-Identifier: GPL-3.0-or-later
param([string]$Steps)
# Steps: semicolon-separated "click:x,y" (window-relative), "type:text", "key:{ENTER}", "wait:ms"
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Runtime.InteropServices;
public class U {
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
 [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
 [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
 public struct RECT { public int L, T, R, B; }
}
"@
[U]::SetProcessDPIAware() | Out-Null
$p = Get-Process foamcraft-app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = $p.MainWindowHandle
[U]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 400
$r = New-Object U+RECT; [U]::GetWindowRect($h, [ref]$r) | Out-Null
foreach ($s in $Steps.Split(';')) {
  $kind, $arg = $s.Split(':', 2)
  switch ($kind) {
    'click' { $x, $y = $arg.Split(','); [U]::SetCursorPos($r.L + [int]$x, $r.T + [int]$y) | Out-Null; Start-Sleep -Milliseconds 120; [U]::mouse_event(2,0,0,0,[UIntPtr]::Zero); [U]::mouse_event(4,0,0,0,[UIntPtr]::Zero) }
    'type' { [System.Windows.Forms.SendKeys]::SendWait($arg) }
    'key' { [System.Windows.Forms.SendKeys]::SendWait($arg) }
    'wait' { Start-Sleep -Milliseconds ([int]$arg) }
    'wheel' { $x, $y, $d = $arg.Split(','); [U]::SetCursorPos($r.L + [int]$x, $r.T + [int]$y) | Out-Null; Start-Sleep -Milliseconds 80; [U]::mouse_event(0x0800,0,0,[BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$d),0),[UIntPtr]::Zero) }
  }
  Start-Sleep -Milliseconds 250
}

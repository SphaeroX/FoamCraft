# SPDX-License-Identifier: GPL-3.0-or-later
param([string]$Out, [int]$DragDx = 0)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W {
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
 [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
 [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
 public struct RECT { public int L, T, R, B; }
}
"@
[W]::SetProcessDPIAware() | Out-Null
$p = Get-Process foamcraft-app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = $p.MainWindowHandle
[W]::ShowWindow($h, 9) | Out-Null
[W]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 600
$r = New-Object W+RECT; [W]::GetWindowRect($h, [ref]$r) | Out-Null
if ($DragDx -ne 0) {
  $cx = [int](($r.L + $r.R) / 2); $cy = [int](($r.T + $r.B) / 2) + 80
  [W]::SetCursorPos($cx, $cy); Start-Sleep -Milliseconds 100
  [W]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero)
  for ($i = 1; $i -le 20; $i++) { [W]::SetCursorPos($cx + [int]($DragDx * $i / 20), $cy); Start-Sleep -Milliseconds 20 }
  [W]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 400
}
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"$w x $hh"

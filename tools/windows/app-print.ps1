# SPDX-License-Identifier: GPL-3.0-or-later
# Captures the FoamCraft window without bringing it to the front and without touching the mouse
# (PrintWindow with full content), so a check does not disturb someone working at the computer.
param([string]$Out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class P {
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
 [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
 public struct RECT { public int L, T, R, B; }
}
"@
[P]::SetProcessDPIAware() | Out-Null
$p = Get-Process foamcraft-app | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = $p.MainWindowHandle
$r = New-Object P+RECT; [P]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
# 2 = PW_RENDERFULLCONTENT: also content drawn by the GPU.
$ok = [P]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"$w x $hh ok=$ok"

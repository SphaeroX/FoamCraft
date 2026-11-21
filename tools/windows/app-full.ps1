# SPDX-License-Identifier: GPL-3.0-or-later
param([string]$Out)
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type 'using System.Runtime.InteropServices; public class D { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }'
[D]::SetProcessDPIAware() | Out-Null
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.X, $b.Y, 0, 0, $bmp.Size)
$small = New-Object System.Drawing.Bitmap $bmp, ([int]($b.Width/2)), ([int]($b.Height/2))
$small.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)

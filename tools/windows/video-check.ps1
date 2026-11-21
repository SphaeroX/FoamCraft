# SPDX-License-Identifier: GPL-3.0-or-later
# Checks that Windows itself (Media Foundation, the engine of the built-in players) opens a video:
# prints duration, size and codec and writes the decoded thumbnail frame of Windows.
#   powershell -ExecutionPolicy Bypass -File tools\windows\video-check.ps1 -Video <file> -Frame <png>
param([string]$Video, [string]$Frame)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Editing.MediaClip, Windows.Media.Editing, ContentType = WindowsRuntime]
$asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
  $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
function Wait-Op($op, [type]$type) {
  $task = $asTask.MakeGenericMethod($type).Invoke($null, @($op))
  $task.Wait() | Out-Null
  $task.Result
}
$file = Wait-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync((Resolve-Path $Video).Path)) ([Windows.Storage.StorageFile])
$clip = Wait-Op ([Windows.Media.Editing.MediaClip]::CreateFromFileAsync($file)) ([Windows.Media.Editing.MediaClip])
$props = $clip.GetVideoEncodingProperties()
"duration_s=$($clip.OriginalDuration.TotalSeconds) size=$($props.Width)x$($props.Height) subtype=$($props.Subtype)"
# The video thumbnail of Windows: a frame decoded by Media Foundation.
$null = [Windows.Storage.FileProperties.StorageItemThumbnail, Windows.Storage, ContentType = WindowsRuntime]
$stream = Wait-Op ($file.GetThumbnailAsync([Windows.Storage.FileProperties.ThumbnailMode]::VideosView, 1024)) ([Windows.Storage.FileProperties.StorageItemThumbnail])
$out = [System.IO.File]::Create($Frame)
[System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($stream).CopyTo($out)
$out.Close()
"frame=$Frame"

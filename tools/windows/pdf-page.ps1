# SPDX-License-Identifier: GPL-3.0-or-later
# Renders pages of a PDF to PNG with the PDF engine of Windows (Windows.Data.Pdf), to look at
# generated reports:  powershell -ExecutionPolicy Bypass -File tools\windows\pdf-page.ps1 -Pdf <file> -Out <prefix>
param([string]$Pdf, [string]$Out, [int]$First = 1, [int]$Last = 99)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
$ext = [System.WindowsRuntimeSystemExtensions].GetMethods()
$asTaskOp = $ext | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
  $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
$asTaskAction = $ext | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
  $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' } | Select-Object -First 1
function Wait-Op($op, [type]$type) { $t = $asTaskOp.MakeGenericMethod($type).Invoke($null, @($op)); $t.Wait() | Out-Null; $t.Result }
function Wait-Action($op) { $t = $asTaskAction.Invoke($null, @($op)); $t.Wait() | Out-Null }
$file = Wait-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync((Resolve-Path $Pdf).Path)) ([Windows.Storage.StorageFile])
$doc = Wait-Op ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
"pages=$($doc.PageCount)"
for ($i = $First - 1; $i -lt [Math]::Min($doc.PageCount, $Last); $i++) {
  $page = $doc.GetPage($i)
  $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
  Wait-Action ($page.RenderToStreamAsync($stream))
  $path = "$Out-$($i + 1).png"
  $fs = [System.IO.File]::Create($path)
  [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($stream.GetInputStreamAt(0)).CopyTo($fs)
  $fs.Close()
  $path
}

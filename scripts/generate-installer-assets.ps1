$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$logoPath = Join-Path $projectRoot "src\assets\seekdash-logo.png"
$assetDirectory = Join-Path $projectRoot "src-tauri\installer-assets"
New-Item -ItemType Directory -Force -Path $assetDirectory | Out-Null

$logo = [System.Drawing.Image]::FromFile($logoPath)
$navy = [System.Drawing.Color]::FromArgb(8, 22, 40)
$blue = [System.Drawing.Color]::FromArgb(38, 155, 255)
$white = [System.Drawing.Color]::FromArgb(246, 249, 253)
$muted = [System.Drawing.Color]::FromArgb(153, 174, 197)

function New-Canvas([int]$width, [int]$height) {
  $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $graphics.Clear($navy)
  return @($bitmap, $graphics)
}

$header = New-Canvas 150 57
$headerBitmap = $header[0]
$headerGraphics = $header[1]
$headerGraphics.FillRectangle((New-Object System.Drawing.SolidBrush($blue)), 0, 54, 150, 3)
$headerGraphics.DrawImage($logo, 96, 3, 54, 54)
$headerBitmap.Save((Join-Path $assetDirectory "header.bmp"), [System.Drawing.Imaging.ImageFormat]::Bmp)
$headerGraphics.Dispose()
$headerBitmap.Dispose()

$sidebar = New-Canvas 164 314
$sidebarBitmap = $sidebar[0]
$sidebarGraphics = $sidebar[1]
$sidebarGraphics.DrawImage($logo, 24, 52, 116, 116)
$titleFont = New-Object System.Drawing.Font("Segoe UI", 18, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$versionFont = New-Object System.Drawing.Font("Segoe UI", 11, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$titleBrush = New-Object System.Drawing.SolidBrush($white)
$mutedBrush = New-Object System.Drawing.SolidBrush($muted)
$blueBrush = New-Object System.Drawing.SolidBrush($blue)
$titleFormat = New-Object System.Drawing.StringFormat
$titleFormat.Alignment = [System.Drawing.StringAlignment]::Center
$sidebarGraphics.DrawString("SeekDash", $titleFont, $titleBrush, (New-Object System.Drawing.RectangleF(0, 178, 164, 28)), $titleFormat)
$sidebarGraphics.DrawString("1.0.0", $versionFont, $mutedBrush, (New-Object System.Drawing.RectangleF(0, 208, 164, 20)), $titleFormat)
$sidebarGraphics.FillRectangle($blueBrush, 48, 244, 68, 3)
$sidebarBitmap.Save((Join-Path $assetDirectory "sidebar.bmp"), [System.Drawing.Imaging.ImageFormat]::Bmp)

$titleFormat.Dispose()
$blueBrush.Dispose()
$mutedBrush.Dispose()
$titleBrush.Dispose()
$versionFont.Dispose()
$titleFont.Dispose()
$sidebarGraphics.Dispose()
$sidebarBitmap.Dispose()
$logo.Dispose()

# Генератор иконки caps-switch: скруглённый ключ + встречные стрелки.
# Результат: assets/icon.ico (PNG-энтри для Vista+) и assets/preview-256.png

Add-Type -AssemblyName System.Drawing

$root = 'C:\Users\kas\dev\caps-switch'
$assets = Join-Path $root 'assets'
New-Item -ItemType Directory -Path $assets -Force | Out-Null

function New-RoundedRectPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-ArrowPath([float]$fromX, [float]$toX, [float]$y, [float]$headW, [float]$headH) {
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $dir = if ($toX -gt $fromX) { 1 } else { -1 }
    $tipX = $toX
    $baseX = $tipX - $dir * $headW
    $path.AddLine($fromX, $y, $baseX, $y)
    $points = [System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new($tipX, $y),
        [System.Drawing.PointF]::new($baseX, $y - $headH / 2),
        [System.Drawing.PointF]::new($baseX, $y + $headH / 2)
    )
    $path.AddPolygon($points)
    return $path
}

function New-IconBitmap([int]$size) {
    $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $s = [float]$size
    $margin = $s * 0.05
    $depth = [float]([Math]::Max(1.0, $s * 0.11))   # высота «боковины» — 3D
    $radius = $s * 0.20
    $topH = $s - 2 * $margin - $depth
    $rectW = $s - 2 * $margin

    # Нижняя часть (основание) — тёмно-серый «бок» клавиши.
    $basePath = New-RoundedRectPath $margin ($margin + $depth) $rectW $topH $radius
    $baseBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 58, 58, 60))
    $g.FillPath($baseBrush, $basePath)

    # Верхняя площадка — светло-серый градиент.
    $topPath = New-RoundedRectPath $margin $margin $rectW $topH $radius
    $topBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        [System.Drawing.PointF]::new(0, $margin),
        [System.Drawing.PointF]::new(0, $margin + $topH),
        [System.Drawing.Color]::FromArgb(255, 214, 214, 216),
        [System.Drawing.Color]::FromArgb(255, 154, 154, 158)
    )
    $g.FillPath($topBrush, $topPath)

    # Тонкая светлая кромка сверху — блик.
    $edgePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(160, 255, 255, 255), [float]([Math]::Max(0.6, $s * 0.02)))
    $g.DrawPath($edgePen, $topPath)

    # Стрелки — тёмные, чтобы читались на сером.
    $penWidth = [float]([Math]::Max(1.0, $s * 0.075))
    $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 34, 34, 36), $penWidth)
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

    $left = $s * 0.27
    $right = $s * 0.73
    $centerY = $margin + $topH / 2
    $topY = $centerY - $topH * 0.17
    $bottomY = $centerY + $topH * 0.17
    $headW = [float]($s * 0.13)
    $headH = [float]($s * 0.17)

    $topArrow = New-ArrowPath $left $right $topY $headW $headH
    $bottomArrow = New-ArrowPath $right $left $bottomY $headW $headH

    $g.DrawPath($pen, $topArrow)
    $g.DrawPath($pen, $bottomArrow)

    $g.Dispose()
    $pen.Dispose()
    $edgePen.Dispose()
    $topBrush.Dispose()
    $baseBrush.Dispose()
    $topArrow.Dispose()
    $bottomArrow.Dispose()
    $topPath.Dispose()
    $basePath.Dispose()

    return $bmp
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$pngs = @{}

foreach ($size in $sizes) {
    $bmp = New-IconBitmap $size
    $ms = [System.IO.MemoryStream]::new()
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngs[$size] = $ms.ToArray()
    if ($size -eq 256) {
        $bmp.Save((Join-Path $assets 'preview-256.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    }
    $bmp.Dispose()
    $ms.Dispose()
}

# Сборка .ico вручную: заголовок + записи + PNG-данные (Vista+ понимает PNG в ICO).
$out = [System.IO.MemoryStream]::new()
$bw = [System.IO.BinaryWriter]::new($out)

$bw.Write([UInt16]0)           # reserved
$bw.Write([UInt16]1)           # type = icon
$bw.Write([UInt16]$sizes.Count)

$offset = 6 + 16 * $sizes.Count
foreach ($size in $sizes) {
    $data = $pngs[$size]
    $dim = if ($size -ge 256) { 0 } else { $size }
    $bw.Write([Byte]$dim)      # width
    $bw.Write([Byte]$dim)      # height
    $bw.Write([Byte]0)         # palette
    $bw.Write([Byte]0)         # reserved
    $bw.Write([UInt16]1)       # planes
    $bw.Write([UInt16]32)      # bpp
    $bw.Write([UInt32]$data.Length)
    $bw.Write([UInt32]$offset)
    $offset += $data.Length
}

foreach ($size in $sizes) {
    $bw.Write($pngs[$size])
}

$bw.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $assets 'icon.ico'), $out.ToArray())
$bw.Dispose()
$out.Dispose()

"icon.ico: $((Get-Item (Join-Path $assets 'icon.ico')).Length) байт"
"preview-256.png: $((Get-Item (Join-Path $assets 'preview-256.png')).Length) байт"

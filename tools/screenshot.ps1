param(
    [int]$x,
    [int]$y,
    [int]$w,
    [int]$h,
    [string]$out
)
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, [System.Drawing.Size]::new($w, $h))
$g.Dispose()
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "saved $out ($w x $h)"

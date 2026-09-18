param(
    [int]$ProcId,
    [string]$out,
    [string]$Click = "",     # optional "x,y" in CLIENT coordinates, clicked before capture
    [int]$SettleMs = 400     # wait after the click before capturing
)
# Capture the app window itself via PrintWindow (PW_RENDERFULLCONTENT) —
# no desktop background bleeding at the edges, works when overlapped.
# With -Click, first brings the window to the foreground and clicks the
# given client point (used to switch tabs for specific-view screenshots).
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinShot {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public struct POINT { public int X; public int Y; }
}
"@
$p = Get-Process -Id $ProcId
$hwnd = $p.MainWindowHandle
if ($hwnd -eq [IntPtr]::Zero) { throw "no main window for pid $ProcId" }

if ($Click -ne "") {
    [WinShot]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Milliseconds 250
    $parts = $Click.Split(",")
    $pt = New-Object WinShot+POINT
    $pt.X = [int]$parts[0]; $pt.Y = [int]$parts[1]
    [WinShot]::ClientToScreen($hwnd, [ref]$pt) | Out-Null
    [WinShot]::SetCursorPos($pt.X, $pt.Y) | Out-Null
    Start-Sleep -Milliseconds 120
    [WinShot]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero) # LEFTDOWN
    [WinShot]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero) # LEFTUP
    Start-Sleep -Milliseconds $SettleMs
    # the window may have moved (e.g. OS cascade placement); re-resolve the handle
    $p.Refresh()
    $hwnd = $p.MainWindowHandle
    if ($hwnd -eq [IntPtr]::Zero) { throw "main window lost after click" }
}

$rect = New-Object WinShot+RECT
[WinShot]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
if ($w -le 0 -or $h -le 0) { throw "bad window rect" }

$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
# 2 = PW_RENDERFULLCONTENT (includes DirectComposition/WebView2 content)
[WinShot]::PrintWindow($hwnd, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$g.Dispose()
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "saved $out ($w x $h)"

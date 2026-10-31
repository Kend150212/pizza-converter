param(
    [int]$HoldMs = 850,
    [string]$Mode = "ctrl+shift"
)

$source = @"
using System;
using System.Runtime.InteropServices;
public class WinKey {
    [DllImport("user32.dll")]
    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
    public const int VK_LCONTROL = 0xA2;
    public const int VK_LSHIFT = 0xA0;
    public const int VK_LMENU = 0xA4;
    public const byte SCAN_LCTRL = 0x1D;
    public const byte SCAN_LSHIFT = 0x2A;
    public const byte SCAN_LALT = 0x38;
    public const int KEYEVENTF_KEYUP = 0x0002;

    public static void HoldCtrlShift(int ms) {
        keybd_event(VK_LCONTROL, SCAN_LCTRL, 0, UIntPtr.Zero);
        keybd_event(VK_LSHIFT, SCAN_LSHIFT, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(ms);
        keybd_event(VK_LSHIFT, SCAN_LSHIFT, KEYEVENTF_KEYUP, UIntPtr.Zero);
        keybd_event(VK_LCONTROL, SCAN_LCTRL, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }

    public static void HoldShift(int ms) {
        keybd_event(VK_LSHIFT, SCAN_LSHIFT, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(ms);
        keybd_event(VK_LSHIFT, SCAN_LSHIFT, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }
}
"@

Add-Type -TypeDefinition $source
if ($Mode -eq "shift") {
    [WinKey]::HoldShift($HoldMs)
} else {
    [WinKey]::HoldCtrlShift($HoldMs)
}
Write-Host "Simulated $Mode hold for $HoldMs ms"

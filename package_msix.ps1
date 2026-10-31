param(
    [string]$IdentityName = "NeelFlow.PizzaConverter",
    [string]$Publisher = "CN=NeelFlow",
    [string]$PublisherDisplayName = "Ken Dao / NeelFlow",
    [string]$Version = "0.1.0.0"
)

$ErrorActionPreference = "Stop"

$MakeAppx = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\makeappx.exe"
$StageDir = "dist\msix_stage"
$OutputDir = "dist\store_packages"

# 1. Clean & recreate staging directories
if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
if (!(Test-Path $OutputDir)) { New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null }
New-Item -ItemType Directory -Force -Path "$StageDir\Assets" | Out-Null

# 2. Copy latest compiled binary
Copy-Item "target\release\pizza-converter.exe" -Destination "$StageDir\pizza-converter.exe" -Force

# 3. Generate exact Microsoft Store required logo sizes from src-tauri\icons\128x128.png
$pyScript = @"
from PIL import Image
import os

src = r'src-tauri\icons\128x128.png'
out_dir = r'dist\msix_stage\Assets'
os.makedirs(out_dir, exist_ok=True)

im = Image.open(src).convert('RGBA')

# Square logos
im.resize((50, 50), Image.Resampling.LANCZOS).save(os.path.join(out_dir, 'StoreLogo.png'))
im.resize((44, 44), Image.Resampling.LANCZOS).save(os.path.join(out_dir, 'Square44x44Logo.png'))
im.resize((150, 150), Image.Resampling.LANCZOS).save(os.path.join(out_dir, 'Square150x150Logo.png'))

# Wide 310x150 logo (centered 120x120 icon on transparent 310x150 canvas)
wide = Image.new('RGBA', (310, 150), (0, 0, 0, 0))
icon_120 = im.resize((120, 120), Image.Resampling.LANCZOS)
wide.paste(icon_120, ((310 - 120) // 2, (150 - 120) // 2), icon_120)
wide.save(os.path.join(out_dir, 'Wide310x150Logo.png'))
"@

python -c $pyScript

# 4. Generate AppxManifest.xml compliant with Microsoft Store full validation
$Manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">

  <Identity
    Name="$IdentityName"
    ProcessorArchitecture="x64"
    Publisher="$Publisher"
    Version="$Version" />

  <Properties>
    <DisplayName>Pizza Converter</DisplayName>
    <PublisherDisplayName>$PublisherDisplayName</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
    <Description>Ultra-Fast, Offline &amp; Privacy-First Universal Media Converter with Radial Wheel Menu.</Description>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22621.0" />
  </Dependencies>

  <Resources>
    <Resource Language="en-us" />
  </Resources>

  <Applications>
    <Application Id="App" Executable="pizza-converter.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="Pizza Converter"
        Description="Fast universal file converter with radial menu"
        BackgroundColor="#090d16"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile ShortName="PizzaConverter" Wide310x150Logo="Assets\Wide310x150Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
"@

[System.IO.File]::WriteAllText((Join-Path (Get-Location) "$StageDir\AppxManifest.xml"), $Manifest, (New-Object System.Text.UTF8Encoding($false)))

# 5. Create MSIX package via MakeAppx WITH full validation (no /nv!)
$MsixFile = "$OutputDir\PizzaConverter_0.1.0.0_x64.msix"
if (Test-Path $MsixFile) { Remove-Item -Force $MsixFile }

& $MakeAppx pack /d $StageDir /p $MsixFile /o

if (Test-Path $MsixFile) {
    Write-Host "SUCCESS: Validated MSIX package created at $MsixFile"
    Get-Item $MsixFile | Select-Object Name, Length, LastWriteTime
} else {
    Write-Error "Failed to package MSIX"
}

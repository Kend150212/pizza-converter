<#
// build-msix.ps1 — KHUNG script đóng gói MSIX cho Pizza Converter (Tauri v2)
// QA & Phát hành · 29/10/2026 (cập nhật tên 29/10: "Pizza Converter" đã được anh Ken giữ trên Partner Center)
// Chạy trên máy build Windows có: Tauri build output + Windows SDK (makeappx, signtool).
//
// Cách dùng:
//   .\build-msix.ps1 -Version "0.1.0.0" -BuildDir "..\pizza-converter\src-tauri\target\release" -OutDir ".\out"
//
// Luồng:
//   1. Stage: copy exe + resources của Tauri vào staging\ (giữ cấu trúc thư mục con Pie\)
//   2. Sinh AppxManifest.xml từ template (thay version, publisher CN, exe path)
//   3. makeappx pack -> .msix
//   4. Ký bằng cert test (sideload) — bản Store thì Partner Center tự ký lại.
//
// TODO khi Dev có build thật:
//   - Chốt tên file exe của Tauri (hiện để placeholder, sửa $ExeName)
//   - Thêm Assets\ (logo các size) từ team Thiết kế
//   - Tạo cert test 1 lần: New-SelfSignedCertificate -Type CodeSigningCert ...
#>
param(
    [Parameter(Mandatory = $true)][string]$Version,      # vd "0.1.0.0" (MSIX bắt buộc 4 phần)
    [Parameter(Mandatory = $true)][string]$BuildDir,     # thư mục chứa output của `tauri build --no-bundle`
    [string]$OutDir = ".\out",
    [string]$CertPath = "",                              # .pfx test; bỏ trống = không ký (chỉ pack)
    [string]$CertPassword = ""
)

$ErrorActionPreference = "Stop"
$ExeName = "Pie.exe"   # TODO: chốt theo tên exe thật do Dev build ra
$Stage = Join-Path $OutDir "staging"
$Template = Join-Path $PSScriptRoot "AppxManifest.xml"

if (-not (Get-Command makeappx.exe -ErrorAction SilentlyContinue)) {
    throw "Không tìm thấy makeappx.exe — cài Windows SDK rồi chạy lại."
}

# 1. Stage
Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
$AppDir = Join-Path $Stage "Pie"
New-Item -ItemType Directory $AppDir | Out-Null
$exe = Join-Path $BuildDir $ExeName
if (-not (Test-Path $exe)) { throw "Không thấy exe: $exe (kiểm tra BuildDir và tên exe)" }
Copy-Item $exe $AppDir
# TODO: copy thêm resources/webview2 files nếu Tauri output có thư mục kèm theo
Copy-Item (Join-Path $PSScriptRoot "Assets") (Join-Path $Stage "Assets") -Recurse -ErrorAction SilentlyContinue

# 2. Manifest từ template
$manifest = Get-Content $Template -Raw
$manifest = $manifest.Replace("__IDENTITY_NAME__", "NeelFlow.PizzaConverter")   # sideload; lúc nộp Store thay theo Partner Center
$manifest = $manifest.Replace("__PUBLISHER_CN__", "CN=PizzaConverterTest")          # sideload; lúc nộp Store thay theo Partner Center
$manifest = $manifest.Replace("__VERSION_4PART__", $Version)
$manifest = $manifest.Replace("__EXE_REL_PATH__", "Pie\$ExeName")
$manifest | Set-Content (Join-Path $Stage "AppxManifest.xml") -Encoding UTF8

# 3. Pack
$msix = Join-Path $OutDir "Pizza-Converter-$Version.msix"
makeappx pack /d $Stage /p $msix /nv | Out-Null
Write-Host "Đã pack: $msix"

# 4. Ký (bản test sideload)
if ($CertPath -ne "") {
    $pw = ConvertTo-SecureString $CertPassword -AsPlainText -Force
    & signtool sign /fd SHA256 /a /f $CertPath /p $CertPassword $msix | Out-Null
    Write-Host "Đã ký: $msix"
} else {
    Write-Host "CHƯA ký (thiếu -CertPath). File .msix chưa cài được — ký trước khi gửi anh Ken test."
}

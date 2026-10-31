# CHECKLIST KỸ THUẬT ĐÓNG GÓI — ConvertX
**QA & Phát hành · 29/10/2026 · gửi chat Điều phối build (đợt 1: mục 1+2 kỹ thuật, mục 3 tài khoản)**

## 0. Kết luận nhanh
- **Windows:** global hotkey (giữ Shift ~0.5s) + đọc file đang chọn trong Explorer **KHÔNG** chạy được trong sandbox Store thuần. **Bắt buộc fullTrust** — đúng như app gốc Converit (MSIX + capability duy nhất `runFullTrust`). Đã xác minh.
- **Tauri v2 KHÔNG tự build MSIX** — Dev phải dựng pipeline tay (cộng đồng đã có mẫu chạy thật) hoặc nộp thẳng installer NSIS/MSI lên Store.
- **macOS:** "giữ Shift" + đọc selection Finder **KHÔNG** làm được trong sandbox App Store (CGEventTap bị chặn, Apple Events tới Finder bị sandbox giết trước khi hỏi quyền). Kênh v1 nên là **DMG notarized (Developer ID)** giữ nguyên UX; App Store chỉ là bản rút gọn làm sau.
- **Tài khoản:** Microsoft Partner cá nhân hiện có luồng **miễn phí** (xác minh ID + selfie); luồng cũ $19/lần. Apple Developer anh Ken **đã có**. Chưa mua/đăng ký gì — chờ lệnh anh Ken.

## 1. WINDOWS — MSIX / Microsoft Store
### 1.1. Hai đường lên Store
- **A. MSIX tự đóng gói (khuyến nghị — giống app gốc):** `tauri build --no-bundle` → stage exe + assets → viết `AppxManifest.xml` tay → `makeappx pack` → nộp Partner Center. Đã có người làm thành công với Tauri (WSL-UI, WumaTracker, 6 app của Illusanato trên Store).
- **B. Nộp thẳng installer NSIS/MSI:** Store chấp nhận Win32 installer — bắt buộc **cài im lặng** (NSIS: `MyApp-setup.exe /S`; MSI: `/quiet`) + **WebView2 offline installer**. Đơn giản hơn nhưng mất lợi ích MSIX (gỡ sạch, update nền qua Store).
### 1.2. Capability phải khai báo (phương án A)
- Namespace: `xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"`
- Capability duy nhất cần: `<rescap:Capability Name="runFullTrust" />` — **restricted capability**: khi nộp Store phải ghi lý do (global hotkey low-level + đọc Explorer selection qua COM); thường được duyệt cho Desktop Bridge. Sideload test nội bộ không cần duyệt.
- **KHÔNG cần** `broadFileSystemAccess` (fullTrust đã có toàn quyền file của user qua Win32).
- `EntryPoint="Windows.FullTrustApplication"`.
### 1.3. "Start with Windows"
Khai báo extension `windows.startupTask` trong manifest (desktop extension, TaskId riêng, Enabled=true) — thay cho ghi registry Run.
### 1.4. Lưu ý build & test
- Version MSIX phải **4 phần** (vd `1.0.0.0`).
- **TẮT Tauri updater trong bản Store** (feature flag lúc build) — update do Store lo. Bản DMG/sideload mới dùng Tauri updater (cần key `TAURI_SIGNING_*`, host trên GitHub Releases hoặc tự host).
- `Publisher` trong Identity phải khớp **Publisher ID** của Partner Center (lấy sau khi đăng ký xong).
- Test sideload nội bộ: tự ký cert (`New-SelfSignedCertificate`) → cài cert vào Trusted People → `Add-AppxPackage`.
### 1.5. Pro upgrade — $19.99 ONE-TIME (Điều phối chốt 29/10, quyền anh Ken)
Chính sách Store 10.8.2: digital unlock trong app **bắt buộc dùng Microsoft Store in-product purchase API** (không được dẫn user ra cổng thanh toán khác). Phí **15%** nếu dùng commerce của Microsoft. (App phi-game được dùng commerce riêng giữ 100%, nhưng không áp dụng cho digital unlock trong app.)
- Giá: **$19.99, mua một lần, KHÔNG subscription**. Test bằng **sandbox IAP của Store** (tài khoản test, không trừ tiền thật) — chi tiết trong `quy-trinh-test-build-20261029.md` §E.
- Mở khóa Pro: **Pro** = AVIF, TIFF, HEIC (xuất), GIF, toàn bộ PDF tools (trích DOCX/TXT/PNG, nén, tách, gộp), Audio·Video, image→PDF. **Free** = PNG, JPG, WEBP, BMP, ICO + xem PDF cơ bản. (Chốt cuối 29/10 theo mockup anh Ken duyệt — GIF = Pro, PDF merge/split/compress = Pro; không còn điểm lệch.)
### 1.6. ffmpeg BUNDLE SẴN trong installer (Điều phối chốt 29/10, quyền anh Ken)
- Dùng build **BtbN win64-lgpl** (LGPL — an toàn cho app closed-source; tránh build GPL như GyanD). ffmpeg.exe chạy riêng, engine gọi qua command line.
- Installer tăng **~100MB** (ghi rõ trong "Ghi chú bản test" gửi anh Ken để không bất ngờ).
- **Checklist LGPL khi bundle:** host source ffmpeg tương ứng ở chỗ user tìm được (link trong About/Settings), ghi attribution, EULA của app KHÔNG được cấm reverse engineering. Tải ffmpeg khi dùng lần đầu bị LOẠI (Store policy 10.2.2 cấm tải remote executable).
- Engine dùng ffmpeg cho: decode HEIC/HEIF → ảnh, toàn bộ Audio·Video convert. (Dev đã từ chối libheif vì LGPL xung đột quy tắc MIT/Apache-only của engine — ffmpeg LGPL gọi ngoài tiến trình thì OK.)

## 2. macOS — DMG NOTARIZED + APP STORE
### 2.1. DMG notarized (kênh chính v1 — khuyến nghị)
- Tauri v2 tự ký + notarize khi có biến môi trường: `APPLE_SIGNING_IDENTITY` (`Developer ID Application: ...`), `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID`; hoặc làm tay: `xcrun notarytool submit --wait` → `xcrun stapler staple`.
- Cần từ anh Ken **khi tới lúc build** (qua Secure Vault, không gửi chat): file `.p12` Developer ID Application + mật khẩu, Team ID, Apple ID + app-specific password.
- Build universal: `--target universal-apple-darwin` (1 bản chạy cả Intel + Apple Silicon).
- Giữ Shift 0.5s: dùng **CGEventTap** — cần quyền Input Monitoring/Accessibility, user cấp 1 lần trong System Settings. Đọc Finder selection: AppleScript + `NSAppleEventsUsageDescription` (user cấp quyền Automation 1 lần). **Cả hai đều chạy được ở bản DMG.**
### 2.2. App Store (sandbox) — giới hạn đã xác minh
- `CGEventTap` **KHÔNG** chạy khi `com.apple.security.app-sandbox=true` (tạo tap trả về nil) → không làm được "giữ Shift".
- Apple Events tới Finder bị sandbox chặn trước cả khi hỏi quyền → **không đọc được file đang chọn trong Finder**.
- Hotkey toàn cục trong sandbox chỉ làm được **tổ hợp phím thật** (Carbon `RegisterEventHotKey`, không cần quyền) — không nuốt phím hệ thống, UX khác bản gốc.
- Đường chính thống lấy file từ Finder trong sandbox: **Finder Quick Action / Share extension** (user chuột phải → Quick Actions) — cần target extension native riêng, việc lớn cho Dev, để sau.
- Entitlements bản MAS (nếu làm sau): `app-sandbox=true`, `files.user-selected.read-only`, login item qua `SMAppService`.
- **Đề xuất:** v1 = DMG notarized giữ nguyên UX; MAS là bản rút gọn (hotkey tổ hợp + Quick Action) làm sau khi DMG ổn định.

## 3. CHECKLIST TÀI KHOẢN & CHI PHÍ
- [ ] **Microsoft Partner Center (cá nhân):** luồng mới **miễn phí** (xác minh giấy tờ + selfie tại storedeveloper.microsoft.com); luồng cũ $19/lần. **CHƯA ĐĂNG KÝ — chờ lệnh anh Ken.** Sau đăng ký: reserve tên app, lấy Publisher ID, khai payout/thuế trước khi bán Pro.
- [ ] **Apple Developer:** anh Ken **ĐÃ CÓ** ($99/năm). Cần khi build: Developer ID Application cert, Team ID, app-specific password (lấy qua Secure Vault).
- [ ] Không mua thêm gì. **Tổng chi phí dự kiến: $0–19 một lần.**

## 4. Quyết định cần anh Ken (qua Điều phối)
1. Windows: MSIX tự build (khuyến nghị) hay nộp thẳng NSIS lên Store?
2. macOS v1: DMG notarized giữ nguyên UX (khuyến nghị) — App Store để sau?
3. (đợt 1 đã có) tên app chính thức · phạm vi v1. **ffmpeg: CHỐT bundle sẵn (BtbN win64-lgpl) — Điều phối quyết 29/10 (quyền anh Ken), xem §1.6.**

## 5. Việc tiếp theo của QA (mục 4)
Dựng pipeline build Windows MSIX + macOS DMG từ cùng source khi Dev có PoC chạy được; test cài sạch trên VM/máy thật; test matrix theo ma trận format của brief. **Không đẩy bất kỳ bản nào lên Store khi chưa có lệnh anh Ken qua Điều phối; bản test chỉ phân phối nội bộ.**

---
Nguồn kiểm chứng: [Tauri – Microsoft Store](https://tauri.app/distribute/microsoft-store/) · [MS Learn – free individual onboarding](https://learn.microsoft.com/en-us/windows/apps/publish/whats-new-individual-developer) · [Store Policies 10.8.2](https://learn.microsoft.com/en-us/windows/apps/publish/store-policy-archive/store-policy-7-16-1) · [QUICOPY – macOS sandbox keyboard shortcuts](https://www.quicopy.com/blog/macos-sandbox-keyboard-shortcuts) · [dev.to – WSL-UI MSIX journey](https://dev.to/octasoft-ltd/building-wsl-ui-the-microsoft-store-journey-2428)

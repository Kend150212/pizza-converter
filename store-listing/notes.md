# Hồ sơ nộp Microsoft Store — Pizza Converter

> Nghiên cứu ngày 29/10/2026. Tên "Pizza Converter" đã reserve trên Partner Center của Ken (status: Not started). KHÔNG nộp gì, KHÔNG mua gì — chỉ nghiên cứu + soạn thảo.

---

## 1. Checklist nộp app MSIX fullTrust

### Trước khi nộp (kỹ thuật)
- [x] Package `.msix` (x64) đã đóng gói: `dist/store_packages/PizzaConverter_0.1.0.0_x64.msix`
- [x] `AppxManifest.xml`: **Identity Name + Publisher**: `NeelFlow.PizzaConverter`, `CN=NeelFlow`
- [x] Version **4 phần**: `0.1.0.0`
- [x] `TargetDeviceFamily`: `Windows.Desktop`; `MinVersion` ≥ `10.0.17763.0` (Windows 10 1809)
- [x] Capability duy nhất: `<rescap:Capability Name="runFullTrust" />` (+ namespace `rescap`)
- [x] **TẮT Tauri updater** trong bản Store (Store tự cập nhật app)
- [x] App cài/gỡ sạch, không đòi quyền admin, không crash khi mở lần đầu
- [ ] Chạy thử Windows App Certification Kit (WACK) local trước khi upload

### Các bước trong Partner Center (Apps and games → Pizza Converter → Start your submission)
1. **Pricing and Availability** — chọn Free (app miễn phí, thu tiền qua IAP Pro), thị trường (mặc định toàn cầu), ngày publish (Manual/as soon as possible).
2. **Properties** — Category: **Utilities & tools**; khai báo capability (runFullTrust sẽ hiện bước giải trình riêng).
3. **Age Ratings** — làm questionnaire IARC → dự kiến **Everyone** (app tiện ích, không nội dung nhạy cảm).
4. **Packages** — upload `.msix` / `.msixupload`; điền **lý do xin runFullTrust** (xem mẫu §5).
5. **Store Listings** — mô tả EN + VI (§2), screenshots, Store logo 300×300, keywords, product features, support contact (`pizza@neeflow.com`).
6. **Add-ons (IAP)** — tạo durable add-on "Pizza Converter Pro" (§4).
7. **Submission Options** — **Notes for certification** (hướng dẫn tester, xem mẫu §5), có thể đặt lịch publish.
8. **Submit for certification** — Partner Center validate thiếu gì báo ngay.

### Chứng nhận (certification)
- Test tự động (malware, crash khi khởi động) + review thủ công (tuân thủ chính sách).
- Lần đầu: **24–48 giờ**; các bản update sau thường <12–24 giờ.
- App fullTrust có thể bị soi kỹ hơn ở vòng manual — phần giải trình runFullTrust + notes for certification phải rõ ràng.

### ⚠️ QUAN TRỌNG: tên reserve chỉ giữ được 3 THÁNG
Theo Microsoft Learn: reserve tên trước khi app sẵn sàng tối đa **3 tháng**. Tên "Pizza Converter" reserve ngày 29/10/2026 → phải nộp (ít nhất 1 submission) trước khoảng **29/01/2027**, không là mất tên.

---

## 2. Nội dung listing

### 2.1. Mô tả tiếng Anh (EN)
```
Pizza Converter turns file conversion into a single gesture.

Select a file in File Explorer, press shortcut (Ctrl + G or custom key), and a beautiful glass pie menu appears around your cursor. Click a slice to convert — images, documents, spreadsheets, PDFs, audio, and video. No uploads, no waiting: everything runs 100% locally on your PC, so your files never leave your machine.

KEY FEATURES
• One-gesture conversion: press shortcut over selected files, flick to the format you want
• 40+ formats: PNG, JPG, WebP, AVIF, HEIC (iPhone photos), BMP, ICO, GIF, TIFF, PDF, DOCX, XLSX, CSV, JSON, MP3, MP4, and more
• Word/Excel/PowerPoint → PDF, PDF → Word/Excel, merge, split, compress, rotate and watermark PDFs
• One-click image background removal (Pro)
• Smart menu learns your favorite formats; preview output size before converting
• Full history with one-click undo — never lose the original file
• 9 languages, works fully offline

Free forever for everyday formats. Upgrade to Pro with a one-time purchase (no subscription) to unlock AVIF, TIFF, HEIC, audio/video conversion, batch conversion, and neural background removal.

SUPPORT & PRIVACY
• Support Email: pizza@neeflow.com
• Legal & Support Desk: support@neeflow.com
• Privacy Policy: 100% on-device processing, zero telemetry
```

### 2.2. Mô tả tiếng Việt (VI)
```
Pizza Converter biến đổi file thành một cử chỉ duy nhất.

Chọn file trong File Explorer, bấm phím tắt (Ctrl + G hoặc phím tùy chỉnh), menu bánh tròn bằng kính hiện ra quanh con trỏ chuột. Bấm vào lát cần để convert — ảnh, tài liệu, bảng tính, PDF, audio, video. Không upload, không chờ đợi: mọi thứ chạy 100% trên máy bạn, file không bao giờ rời khỏi máy.

TÍNH NĂNG CHÍNH
• Convert bằng một cử chỉ: bấm phím tắt trên file đã chọn, click định dạng cần
• 40+ định dạng: PNG, JPG, WebP, AVIF, HEIC (ảnh iPhone), BMP, ICO, GIF, TIFF, PDF, DOCX, XLSX, CSV, JSON, MP3, MP4...
• Word/Excel/PowerPoint → PDF, PDF → Word/Excel, gộp/tách/nén/xoay/đóng dấu PDF
• Xóa nền ảnh một chạm (Pro)
• Menu thông minh học thói quen của bạn; xem trước dung lượng file trước khi convert
• Lịch sử đầy đủ + hoàn tác một chạm — không bao giờ mất file gốc
• 9 ngôn ngữ, chạy hoàn toàn offline

Miễn phí trọn đời cho các định dạng thông dụng. Nâng cấp Pro bằng một lần mua duy nhất (không thuê bao) để mở AVIF, TIFF, HEIC, convert audio/video, convert hàng loạt và xóa nền ảnh.

HỖ TRỢ & BẢO MẬT
• Email hỗ trợ: pizza@neeflow.com
• Trung tâm pháp lý: support@neeflow.com
• Chính sách bảo mật: Xử lý 100% nội bộ trên máy, hoàn toàn không thu thập dữ liệu người dùng.
```

### 2.3. Product features (bullet ngắn, mỗi cái ≤200 ký tự, tối đa 20)
1. Convert files with one gesture — press shortcut, click a slice
2. 40+ formats: images, documents, spreadsheets, PDF, audio, video
3. HEIC iPhone photos → JPG/PNG in one click
4. Word/Excel/PowerPoint → PDF and back
5. Merge, split, compress, rotate, watermark PDFs
6. One-click background removal (Pro)
7. Smart menu learns your favorite formats
8. Preview output file size before converting
9. Full history with one-click undo — never lose originals
10. 100% offline — your files never leave your PC
11. 9 languages supported

### 2.4. Keywords (tối đa 7)
`file converter`, `convert`, `image converter`, `pdf converter`, `heic to jpg`, `video converter`, `format converter`

### 2.5. Category / Age rating
- **Category**: Utilities & tools (nhóm Productivity)
- **Age rating**: Everyone (qua questionnaire IARC)
- **Support Contact Email**: `pizza@neeflow.com`

---

## 3. Assets & Documentation

| Asset / Tài liệu | Đường dẫn / Yêu cầu | Trạng thái |
|---|---|---|
| Privacy Policy HTML | `store-listing/privacy.html` | ✅ Đã soạn thảo đầy đủ chuẩn GDPR/CCPA |
| Terms of Service HTML | `store-listing/terms.html` | ✅ Đã soạn thảo đầy đủ chuẩn EULA/DMCA |
| Third Party Licenses | `store-listing/THIRD_PARTY_LICENSES.md` | ✅ Đã tổng hợp đầy đủ LGPL/MIT/Apache |
| Store Package (.msix) | `dist/store_packages/PizzaConverter_0.1.0.0_x64.msix` | ✅ Đã đóng gói xong |
| Support Routing | `pizza@neeflow.com` -> `ken.d150212@gmail.com` | ✅ Đã kích hoạt trên Cloudflare Email Routing |

---

## 4. Mẫu văn bản giải trình (dùng lúc nộp)

### Lý do xin capability runFullTrust
```
Pizza Converter is a Win32 desktop application (built with Tauri v2 / Rust).
It requires the runFullTrust capability because its core features cannot work inside the UWP sandbox:
(1) a low-level keyboard hook detects user-configured global shortcuts (e.g. Ctrl + G) to summon the radial menu;
(2) the app inspects selected files in File Explorer via COM/Shell Windows APIs;
(3) it executes local background child processes (ffmpeg, LibreOffice) for multimedia and document transformation.
All processing runs 100% locally on the device with zero cloud telemetry. These are the app's advertised features and have no sandboxed equivalent.
```

### Notes for certification (hướng dẫn tester của Microsoft)
```
How to test the app:
1. Launch Pizza Converter from the Start menu — it runs in the system tray.
2. Open File Explorer, select any image file (e.g. a PNG or JPG).
3. Press the global shortcut (default: Ctrl + G) — the radial converter wheel appears at the cursor.
4. Click any format slice (e.g. WebP) — the file is converted locally next to the original.
5. Press Esc or click outside to dismiss the radial wheel.
6. Open Settings from the system tray icon to view Guide, Pro license activation, History, About, Privacy Policy, and Terms of Service.
Note: Some extended multimedia features utilize local command-line tools. The Pro redeem code feature can be tested via the Settings Pro tab. No user account or internet login is required.
Support email: pizza@neeflow.com
```

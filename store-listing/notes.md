# Hồ sơ nộp Microsoft Store — Pizza Converter

> Nghiên cứu ngày 29/10/2026. Tên "Pizza Converter" đã reserve trên Partner Center của Ken (status: Not started). KHÔNG nộp gì, KHÔNG mua gì — chỉ nghiên cứu + soạn thảo.

---

## 1. Checklist nộp app MSIX fullTrust

### Trước khi nộp (kỹ thuật)
- [ ] Package `.msixupload` (bundle). Upload bản **chưa ký** — Store tự ký khi publish.
- [ ] `AppxManifest.xml`: **Identity Name + Publisher** phải khớp CHÍNH XÁC với trang Product identity trong Partner Center (sai là upload lỗi).
- [ ] Version **4 phần** (vd `1.0.0.0`).
- [ ] `TargetDeviceFamily`: `Windows.Desktop`; `MinVersion` ≥ `10.0.17763.0` (Windows 10 1809).
- [ ] Capability duy nhất: `<rescap:Capability Name="runFullTrust" />` (+ namespace `rescap`).
- [ ] **TẮT Tauri updater** trong bản Store (Store tự cập nhật app).
- [ ] App cài/gỡ sạch, không đòi quyền admin, không crash khi mở lần đầu.
- [ ] Chạy thử Windows App Certification Kit (WACK) local trước khi upload.

### Các bước trong Partner Center (Apps and games → Pizza Converter → Start your submission)
1. **Pricing and Availability** — chọn Free (app miễn phí, thu tiền qua IAP Pro), thị trường (mặc định toàn cầu trừ khi muốn giới hạn), ngày publish (để Manual/as soon as possible).
2. **Properties** — Category: **Utilities & tools**; khai báo capability (runFullTrust sẽ hiện bước giải trình riêng).
3. **Age Ratings** — làm questionnaire IARC → dự kiến **Everyone** (app tiện ích, không nội dung nhạy cảm).
4. **Packages** — upload `.msixupload`; điền **lý do xin runFullTrust** (xem mẫu §5).
5. **Store Listings** — mô tả EN + VI (§2), screenshots, Store logo 300×300, keywords, product features, support contact.
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

Select a file in File Explorer, hold Shift, and a beautiful glass pie menu appears
around your cursor. Flick to a slice to convert — images, documents, spreadsheets,
PDFs, audio, and video. No uploads, no waiting: everything runs 100% on your PC,
so your files never leave your machine.

KEY FEATURES
• One-gesture conversion: hold Shift over a selected file, flick to the format you want
• 40+ formats: PNG, JPG, WebP, AVIF, HEIC (iPhone photos), BMP, ICO, GIF, TIFF,
  PDF, DOCX, XLSX, CSV, JSON, MP3, MP4, and more
• Word/Excel/PowerPoint → PDF, PDF → Word/Excel, merge, split, compress, rotate
  and watermark PDFs
• One-click image background removal (Pro)
• Smart menu learns your favorite formats; preview output size before converting
• Full history with one-click undo — never lose the original file
• 9 languages, works fully offline

Free forever for everyday formats. Upgrade to Pro with a one-time purchase
(no subscription) to unlock AVIF, TIFF, HEIC, audio/video conversion, batch
conversion, and background removal.
```

### 2.2. Mô tả tiếng Việt (VI)
```
Pizza Converter biến đổi file thành một cử chỉ duy nhất.

Chọn file trong File Explorer, giữ phím Shift, menu bánh tròn bằng kính hiện ra
quanh con trỏ chuột. Lướt tới lát cần để convert — ảnh, tài liệu, bảng tính, PDF,
audio, video. Không upload, không chờ đợi: mọi thứ chạy 100% trên máy bạn,
file không bao giờ rời khỏi máy.

TÍNH NĂNG CHÍNH
• Convert bằng một cử chỉ: giữ Shift trên file đã chọn, lướt tới định dạng cần
• 40+ định dạng: PNG, JPG, WebP, AVIF, HEIC (ảnh iPhone), BMP, ICO, GIF, TIFF,
  PDF, DOCX, XLSX, CSV, JSON, MP3, MP4...
• Word/Excel/PowerPoint → PDF, PDF → Word/Excel, gộp/tách/nén/xoay/đóng dấu PDF
• Xóa nền ảnh một chạm (Pro)
• Menu thông minh học thói quen của bạn; xem trước dung lượng file trước khi convert
• Lịch sử đầy đủ + hoàn tác một chạm — không bao giờ mất file gốc
• 9 ngôn ngữ, chạy hoàn toàn offline

Miễn phí trọn đời cho các định dạng thông dụng. Nâng cấp Pro bằng một lần mua duy
nhất (không thuê bao) để mở AVIF, TIFF, HEIC, convert audio/video, convert hàng
loạt và xóa nền ảnh.
```

### 2.3. Product features (bullet ngắn, mỗi cái ≤200 ký tự, tối đa 20)
1. Convert files with one gesture — hold Shift, flick to a slice
2. 40+ formats: images, documents, spreadsheets, PDF, audio, video
3. HEIC iPhone photos → JPG/PNG in one flick
4. Word/Excel/PowerPoint → PDF and back
5. Merge, split, compress, rotate, watermark PDFs
6. One-click background removal (Pro)
7. Smart menu learns your favorite formats
8. Preview output file size before converting
9. Full history with one-click undo — never lose originals
10. 100% offline — your files never leave your PC
11. 9 languages

### 2.4. Keywords (tối đa 7)
`file converter`, `convert`, `image converter`, `pdf converter`, `heic to jpg`, `video converter`, `format converter`

### 2.5. Category / Age rating
- **Category**: Utilities & tools (nhóm Productivity)
- **Age rating**: Everyone (qua questionnaire IARC)

---

## 3. Assets cần chuẩn bị

| Asset | Yêu cầu | Trạng thái |
|---|---|---|
| Store logo (1:1 app tile icon) | PNG **300×300**, bắt buộc | ⏳ chờ chốt logo (3 concept A/B/C) |
| Screenshots (Desktop) | PNG, **tối thiểu 1366×768**, tối đa 3840×2160, ≤50MB/file; bắt buộc ≥1, khuyến nghị **4–8** | ⏳ chưa chụp |
| In-package icons | StoreLogo 50×50→200×200 (scale 100–400%), Square44x44Logo, Square150x150Logo, Wide310x150Logo, SplashScreen 620×300 | ⏳ Dev làm lúc build |
| Trailer | Không bắt buộc; nếu có: 30–90 giây | ⏳ tùy chọn |
| Poster art 2:3 (720×1080) | Khuyến nghị mạnh cho game; với app không bắt buộc | ⏳ tùy chọn |

Gợi ý 6 screenshots: (1) radial menu đang mở trên file thật, (2) hover 1 lát format, (3) Settings Liquid Glass, (4) lịch sử + hoàn tác, (5) xem trước dung lượng, (6) xóa nền ảnh.

### Privacy policy
- Partner Center có câu hỏi "app có truy cập/thu thập/truyền personal info không". App đọc file người dùng (fullTrust) → **nên trả lời Yes + cung cấp privacy policy URL** để chắc chắn qua certification (nhiều nguồn ghi Store yêu cầu URL này kể cả app không thu thập).
- App chạy local 100%, không thu thập/không gửi dữ liệu đi đâu — privacy policy viết đúng sự thật này.
- ⏳ **Việc còn thiếu**: cần 1 URL host privacy policy (vd trang trên lacastay.com). Microsoft có chương trình host privacy policy miễn phí (công bố Build 2025) — kiểm tra lại lúc nộp.

---

## 4. IAP "Pro" — one-time purchase qua Microsoft Store

- Dùng **durable add-on** tên "Pizza Converter Pro" (mua một lần, dùng vĩnh viễn, không subscription).
- **Bắt buộc dùng Microsoft commerce** cho hàng số trong app (chính sách Store 10.8.2) → Microsoft lấy **15%** mỗi giao dịch.
- Giá Pro: ⏳ **anh Ken chưa chốt** (mockup để minh họa $19.99).
- Dev phải tích hợp Store IAP API + verify license; **test IAP trong môi trường sandbox** trước khi nộp (IAP chưa test = lý do fail certification phổ biến).
- Lưu ý: app non-game được phép dùng cổng thanh toán riêng giữ 100%, nhưng với unlock tính năng trong app thì chính sách yêu cầu dùng IAP của Microsoft — đã chốt dùng IAP Microsoft.

---

## 5. Mẫu văn bản giải trình (dùng lúc nộp)

### Lý do xin capability runFullTrust
```
Pizza Converter is a Win32 desktop application (built with Tauri/Rust).
It requires the runFullTrust capability because its core features cannot
work inside the UWP sandbox:
(1) a low-level keyboard hook detects holding the Shift key anywhere in
Windows to summon the radial menu;
(2) the app reads the currently selected files in File Explorer via
COM/Shell APIs;
(3) it launches helper processes (ffmpeg, LibreOffice) for media and
document conversion.
These are the app's advertised features and have no sandboxed equivalent.
```

### Notes for certification (hướng dẫn tester của Microsoft)
```
How to test the app:
1. Launch Pizza Converter from the Start menu — it runs in the system tray.
2. Open File Explorer, select any image file (e.g. a PNG).
3. Press and HOLD the Shift key for ~0.5s — a radial menu appears at the cursor.
4. Move the mouse to a slice (e.g. JPG) and release Shift — the file is converted
   next to the original.
5. Release Shift over the center circle or press Esc to cancel.
6. Open Settings from the tray icon to change the trigger key, language
   (9 languages), and output folder.
Note: some formats (Office documents, audio/video) require free external tools
(LibreOffice, ffmpeg); the app guides the user if they are missing. The "Pro"
in-app purchase unlocks premium formats; please test the purchase flow in
sandbox. No login or test account is needed.
```

---

## 6. Rủi ro / điểm cần chốt trước khi nộp

1. **Deadline tên**: nộp submission đầu tiên trước ~29/01/2027 (3 tháng từ ngày reserve).
2. **Privacy policy URL**: chưa có — cần chốt chỗ host.
3. **Giá Pro**: anh Ken chưa chốt (đang để $19.99 minh họa).
4. **ffmpeg**: bundle sẵn hay tải khi cần — anh Ken chưa chốt; ảnh hưởng kích thước package + trải nghiệm tester.
5. **IAP phải test sandbox xong** trước khi nộp.
6. Logo + screenshots + in-package icons: chờ chốt logo (A/B/C) rồi Dev/QA sản xuất lúc build.
7. Không nộp gì lên Store khi chưa có lệnh duyệt của anh Ken (quy tắc tuyệt đối của dự án).

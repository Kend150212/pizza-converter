# PROMPT CHO MUSE — NỘP PIZZA CONVERTER LÊN MICROSOFT STORE QUA CHROME

## 1. Bối cảnh & Mục tiêu
Toàn bộ file gói `.msix`, bộ ảnh chụp màn hình (`1920x1080`), logo (`300x300`), nội dung mô tả (EN/VI), và văn bản giải trình `runFullTrust` của ứng dụng **Pizza Converter** đã được chuẩn bị sẵn sàng 100% trên máy.
Nhiệm vụ của bạn (Muse) là sử dụng trình duyệt **Chrome** (đã đăng nhập Microsoft Partner Center của Ken) để thực hiện nộp ứng dụng **Pizza Converter** lên Microsoft Store từ đầu đến bước bấm **Submit to the Store**.

---

## 2. Đường dẫn toàn bộ File & Tài liệu có sẵn trên máy
Thư mục gốc dự án: `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\`

1. **Gói cài đặt `.msix` (đã pass MakeAppx full validation)**:
   `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\PizzaConverter_0.1.0.0_x64.msix`
2. **Script đóng gói lại `.msix` theo đúng Identity của Partner Center**:
   `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\package_msix.ps1`
3. **Bộ ảnh Screenshots (`1920x1080`) & Store Logo (`300x300`)**:
   - `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\Screenshot_1_Radial_Menu_1920x1080.png`
   - `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\Screenshot_2_Settings_1920x1080.png`
   - `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\Screenshot_3_Guide_1920x1080.png`
   - `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\StoreLogo_300x300.png`
   - `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\StoreLogo_150x150.png`
4. **Hồ sơ nội dung Store Listing & Pháp lý**:
   - Toàn bộ văn bản mô tả EN/VI, keywords, features, giải trình `runFullTrust`, Notes for certification:
     `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\notes.md`
   - Privacy Policy & Terms of Service HTML:
     `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\privacy.html`
     `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\terms.html`

---

## 3. Quy trình thực hiện từng bước trên Microsoft Partner Center (Chrome)

### Bước 1: Lấy thông số Product Identity thật trên Partner Center và đóng lại gói `.msix`
1. Mở Microsoft Partner Center (`https://partner.microsoft.com/dashboard/windowsoverview`) → chọn ứng dụng **Pizza Converter**.
2. Ở menu bên trái, vào **Product management → Product identity** (hoặc **App identity**).
3. Copy 3 giá trị chính xác do Microsoft cấp cho tài khoản của Ken:
   - `Package/Identity/Name` (ví dụ: `12345NeelFlow.PizzaConverter` hoặc `NeelFlow.PizzaConverter`)
   - `Package/Identity/Publisher` (ví dụ: `CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`)
   - `Package/Properties/PublisherDisplayName` (ví dụ: `Ken Dao` hoặc `NeelFlow`)
4. Chạy lệnh PowerShell sau để đóng gói lại file `.msix` khớp 100% với Product Identity vừa lấy (mất 3 giây):
   ```powershell
   cd "C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter"
   powershell -ExecutionPolicy Bypass -File .\package_msix.ps1 `
     -IdentityName "<GIÁ_TRỊ_NAME_TRÊN_PARTNER_CENTER>" `
     -Publisher "<GIÁ_TRỊ_PUBLISHER_CN_TRÊN_PARTNER_CENTER>" `
     -PublisherDisplayName "<GIÁ_TRỊ_PUBLISHER_DISPLAY_NAME>"
   ```
   File xuất ra tại: `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\PizzaConverter_0.1.0.0_x64.msix`.

---

### Bước 2: Bắt đầu Submission (`Application overview` → `Start your submission`)

#### Mục 1: Pricing and availability
- **Markets**: Để mặc định toàn bộ thị trường (All markets).
- **Base price**: Chọn **Free**.
- **Visibility / Discoverability**: Make this product available and discoverable in the Microsoft Store.
- **Publish date**: Release as soon as it passes certification.
- Bấm **Save**.

#### Mục 2: Properties
- **Category**: **Utilities & tools** (Subcategory: File managers hoặc không chọn nếu không bắt buộc).
- **Support contact info**:
  - Support email: `pizza@neeflow.com`
  - Website / Privacy Policy URL: Nếu cần URL công khai, kiểm tra xem domain `neeflow.com` hoặc Cloudflare Worker/Pages của dự án đã trỏ URL chưa, hoặc điền URL chính sách bảo mật tương ứng.
- **System requirements**:
  - Touch screen: Not required
  - Keyboard: Recommended
  - Mouse: Recommended
- **Restricted capabilities declaration (`runFullTrust`)**:
  Nếu Partner Center hiện ô yêu cầu giải trình quyền `runFullTrust`, điền nội dung sau (lấy từ `store-listing/notes.md`):
  ```text
  Pizza Converter is a Win32 desktop application (built with Tauri v2 / Rust).
  It requires the runFullTrust capability because its core features cannot work inside the UWP sandbox:
  (1) a low-level keyboard hook detects user-configured global shortcuts (e.g. Ctrl + G) to summon the radial menu;
  (2) the app inspects selected files in File Explorer via COM/Shell Windows APIs;
  (3) it executes local background child processes for multimedia and document transformation.
  All processing runs 100% locally on the device with zero cloud telemetry. These are the app's advertised features and have no sandboxed equivalent.
  ```
- Bấm **Save**.

#### Mục 3: Age ratings
- Chọn ** Questionnaire (IARC)**:
  - Loại ứng dụng: **Utility / Productivity / Reference** (Tiện ích).
  - Trả lời **No** cho toàn bộ các câu hỏi về nội dung bạo lực, nhạy cảm, cờ bạc, chat người lạ, chia sẻ vị trí, mua hàng không kiểm soát.
- Kết quả sẽ ra **Everyone (3+)** → bấm **Save and close** → **Save**.

#### Mục 4: Packages
- Tải lên file:
  `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\PizzaConverter_0.1.0.0_x64.msix`
- Chờ thanh trạng thái báo **Validated** (xanh lá) và tích chọn thiết bị hỗ trợ **Windows 10/11 Desktop**.
- Bấm **Save**.

#### Mục 5: Store listings (English - United States & Vietnamese)
Đọc file `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\notes.md` để copy và dán chính xác:
- **Description**: Dán phần mô tả EN (cho ngôn ngữ English) và VI (nếu thêm ngôn ngữ Vietnamese).
- **Short description**:
  `One-gesture file converter with a glass radial wheel menu — convert images, PDFs, Office docs, audio & video 100% offline.`
- **Screenshots (Desktop)**: Upload 3 file ảnh trong `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\store-listing\assets\`:
  1. `Screenshot_1_Radial_Menu_1920x1080.png`
  2. `Screenshot_2_Settings_1920x1080.png`
  3. `Screenshot_3_Guide_1920x1080.png`
- **Store logos (300x300)**: Upload `StoreLogo_300x300.png` trong cùng thư mục `store-listing\assets\`.
- **Product features**: Copy 11 dòng bullet từ mục `2.3` trong `store-listing\notes.md`.
- **Search terms (Keywords)**:
  `file converter`, `convert`, `image converter`, `pdf converter`, `heic to jpg`, `video converter`, `format converter`
- Bấm **Save**.

#### Mục 6: Submission options (`Notes for certification`)
- Dán đoạn hướng dẫn kiểm thử sau vào ô **Notes for certification**:
  ```text
  How to test the app:
  1. Launch Pizza Converter from the Start menu — the Settings window opens and the app icon appears in the system tray.
  2. Open File Explorer, select any image, PDF, or document file (e.g. a PNG or JPG).
  3. Press the global shortcut (default: Ctrl + G) — the radial converter wheel appears around the cursor.
  4. Click any format slice (e.g. WebP or JPG) — the file is converted locally next to the original and a progress toast appears.
  5. Press Esc or click outside to dismiss the radial wheel.
  6. Open Settings from the system tray icon to view Guide, Pro license activation, History, About, Privacy Policy, and Terms of Service.
  Note: For certification testing, you can test the Pro redeem code feature in the Settings -> Pro tab using test code: PIZZA-TEST-2026 (or LIFETIME). No user account or login is required.
  Support email: pizza@neeflow.com
  ```
- Bấm **Save**.

### Bước 3: Kiểm tra & Gửi duyệt
- Kiểm tra tất cả các mục trên trang **Submission overview** đã hiện tích xanh **Complete**.
- Nếu có bất kỳ trường nào thiếu hoặc cần xác nhận từ người dùng, báo lại ngay cho anh Ken; nếu mọi mục đều hợp lệ, bấm **Submit to the Store** và chụp màn hình kết quả báo cáo.


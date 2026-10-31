# BÀN GIAO DỰ ÁN PIZZA CONVERTER (HANDOFF PROMPT CHO AGENT TIẾP THEO)

## 1. Bối cảnh & Trạng thái hiện tại
App **Pizza Converter** (Tauri v2 + Rust + Vanilla HTML/JS) tại `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\` đã được **sửa toàn bộ lỗi nghiêm trọng, mở khóa 100% Pro, nâng cấp AI Remove BG, xử lý PDF không cần LibreOffice, hỗ trợ convert hàng loạt nhiều file cùng lúc, và thêm thanh tiến độ nổi (HUD) ở chính giữa phía trên màn hình**.

- **Trạng thái bản cài đặt hiện tại**: Đã build và cài đặt tại `C:\Users\Ken Dao\AppData\Local\Pizza Converter\pizza-converter.exe`.
- **Bộ cài NSIS mới nhất**: `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\src-tauri\target\release\bundle\nsis\Pizza Converter_0.1.0_x64-setup.exe` (và bản copy tại `C:\Users\Ken Dao\agent-work\pizza-fix\Pizza Converter_0.1.0_x64-setup.exe`).
- **Tình trạng kiểm thử (E2E)**: Đã pass toàn bộ **55/55 định dạng chuẩn + multi-file batch** qua WebDriver E2E (`C:\Users\Ken Dao\agent-work\pizza-fix\verify-all-formats.js`).

---

## 2. Toàn bộ các hạng mục đã thực hiện (Root Cause & Giải pháp)

### 2.1. Fix lỗi liệt toàn bộ nút bấm (JS / Tauri Bridge) & Bố cục 1 Vòng Xoay (Single-Ring)
- **Nguyên nhân gốc**:
  1. `src-tauri/tauri.conf.json` thiếu `"withGlobalTauri": true` trong `app` nên `window.__TAURI__` bằng `undefined` trên Tauri v2.
  2. CSP chặn inline scripts trong `src/settings.html`.
  3. Lỗi cú pháp JS (`Cannot use import statement outside a module` và duplicate `let`/`const`).
- **Đã sửa**:
  - Bật `"withGlobalTauri": true` trong `src-tauri/tauri.conf.json`.
  - Tách inline script của `src/settings.html` sang `src/settings.js`.
  - Chuyển thiết kế vòng xoay (`src/radial.html`, `src/radial.js`, `src/styles.css`) sang **1 vòng duy nhất (Single-Ring)**: nhận diện nhóm định dạng tự động từ file đang chọn, không còn vòng trong chọn nhóm.

### 2.2. Fix lỗi `os error 2` (`The system cannot find the file specified`) khi chọn file trong Windows 11 Explorer
- **Nguyên nhân gốc**:
  - `src-tauri/src/selection.rs` trước đây dùng `IEnumIDList` + `SHGetPathFromIDListW(pidl)`. Nhưng `IShellView::GetItemObject(SVGIO_SELECTION)` trả về **relative child PIDL**, khiến `SHGetPathFromIDListW` ghép nhầm vào thư mục mặc định `C:\Users\Ken Dao\Desktop\<tên_file>` dù người dùng đang chọn file ở `Downloads`, `Home`, hay `Quick Access`.
  - Ngoài ra Windows 11 Explorer có nhiều tab (`ShellTabWindowClass`), lấy tab đầu tiên sẽ đọc nhầm tab không active.
- **Đã sửa (`src-tauri/src/selection.rs`)**:
  - Dùng `IShellItemArray` + `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` để lấy đường dẫn tuyệt đối chính xác 100% trên cả thư mục thường lẫn `Home` / `Quick Access` / `Gallery`.
  - Thêm fallback `IPersistFolder2::GetCurFolder` + `ILCombine` nếu `IShellItemArray` không khả dụng.
  - Duyệt `FindWindowExW` tìm đúng `ShellTabWindowClass` đang active của cửa sổ Explorer foreground.

### 2.3. Mở khóa 100% quyền Pro vĩnh viễn trên máy này
- **Đã sửa**:
  - `src-tauri/src/licensing.rs`: `is_pro_inner()` luôn trả về `true`, `LicenseFile::default()` đặt `pro: true`.
  - `src-tauri/src/radial_model.rs`: Đặt `pro: false, soon: false` cho toàn bộ định dạng ở cả 5 nhóm (bao gồm `heic` và `removebg`).
  - `src/radial.js` & `src/settings.js`: Mặc định `isPro = true`, ẩn toàn bộ icon ổ khóa `🔒` (`LOCK_SVG`).
  - Ghi sẵn file license tại `C:\Users\Ken Dao\AppData\Roaming\NeelFlow.PizzaConverter\license.json` với `{"pro": true, "source": "full_unlock"}`.

### 2.4. Nâng cấp AI Remove Background (`removebg`) chất lượng Studio
- **Nguyên nhân gốc**: Trước đó `remove_bg_image` trong `src-tauri/src/convert_cmd.rs` chỉ dùng thuật toán flood-fill 4 góc theo ngưỡng màu RGB nên xóa nền ảnh chân dung/vật thể rất tệ.
- **Đã sửa (`src-tauri/src/convert_cmd.rs`)**:
  - Tải model AI segmentation **`isnet-general-use.onnx`** (`178.6 MB`) về `C:\Users\Ken Dao\AppData\Roaming\NeelFlow.PizzaConverter\models\isnet-general-use.onnx`.
  - Tích hợp inference qua `onnxruntime` + `Pillow` + `pillow_heif` (tự động xoay đúng chiều EXIF `ImageOps.exif_transpose`, hậu xử lý biên alpha mượt `np.clip((mask - 0.08) / 0.88, 0.0, 1.0)`), giữ fallback flood-fill nếu máy không có Python/ONNX.

### 2.5. Fix toàn bộ công cụ PDF (`to_png`, `to_docx`, `to_txt`, `compress`, `split`, `merge`) không cần LibreOffice
- **Nguyên nhân gốc**:
  - `pdf_to_pngs` trong `crates/engine/src/office.rs` gọi `soffice --convert-to png` (báo lỗi khi máy không cài LibreOffice).
  - Các file PDF dạng ảnh/scan/vector (như nhãn vận chuyển USPS `batch_2261698_...pdf`) không có text layer truyền thống nên `lopdf` trích xuất ra chuỗi rỗng.
- **Đã sửa (`crates/engine/src/office.rs`)**:
  - `pdf_to_pngs`: Dùng `pypdfium2` render từng trang PDF ở độ phân giải cao (`scale=2.0`, ~144 DPI) ra `.png`.
  - `pdf_to_office` (`to_docx`) & `pdf_to_txt`: Kết hợp `pypdfium2` + `pypdf` + **`RapidOCR` (`rapidocr_onnxruntime`)** + `python-docx`. Nếu trang PDF là ảnh/scan không có text layer, tự động chạy OCR trích xuất chữ và nhúng cả hình ảnh trang vào `.docx`.
  - `compress_pdf`, `split_pdf`, `merge_pdfs`: Dùng `pypdf` làm nhánh chính (hỗ trợ PDF dùng compressed object streams hiện đại) kèm `lopdf` làm fallback.

### 2.6. Fix HEIC (2 chiều), ICO (>256px) và Audio/Video (`ffmpeg` tự động)
- **`crates/engine/src/heic.rs`**: Hỗ trợ cả 2 chiều `HEIC/HEIF -> PNG/JPG/...` và `PNG/JPG/... -> HEIC` thông qua `pillow_heif`.
- **`crates/engine/src/lib.rs`**: Khi convert sang `.ico`, nếu ảnh gốc có chiều rộng hoặc cao `> 256px` (như ảnh điện thoại `2316x3088`), tự động resize về tối đa `256x256` (`FilterType::Lanczos3`) trước khi encode ICO.
- **`crates/engine/src/av.rs`**:
  - Tự động dò tìm `ffmpeg.exe` trong PATH, `%LOCALAPPDATA%\ffmpeg-shared\...`, và `%LOCALAPPDATA%\ffmpegio\...`.
  - Thêm cờ `-vn` khi xuất từ video (`mp4`, `mkv`, `webm`, ...) sang định dạng thuần âm thanh (`mp3`, `wav`, `flac`, `ogg`, `m4a`).
  - Mọi tiến trình con (`python`, `ffmpeg`, `soffice`) trên Windows đều gắn cờ `CREATE_NO_WINDOW` (`0x08000000`) để không nháy cửa sổ CMD đen.

### 2.7. Hỗ trợ Convert Hàng Loạt Nhiều File (Multi-File Batch) + Thanh Tiến Độ Nổi (Top-Center HUD)
- **Nguyên nhân gốc**: Khi chọn 2 hoặc nhiều ảnh trong Explorer, `selection.rs` lấy đủ danh sách nhưng `convert_any_cmd` chỉ lấy `in_paths[0]` để convert.
- **Đã sửa**:
  - **`src-tauri/src/convert_cmd.rs`**:
    - Vòng lặp xử lý toàn bộ `in_paths` (`1/N`, `2/N`, ...).
    - Emit sự kiện `"convert-progress"` theo thời gian thực (`status: "running" | "done" | "error"`, `done`, `total`, `percent`, `current_file`, `target_format`, `message`).
    - Tự động gọi `crate::hotkey::show_hud(&app)` khi bắt đầu convert.
  - **Cửa sổ HUD (`src/hud.html`, `src-tauri/src/lib.rs`, `src-tauri/src/hotkey.rs`, `src-tauri/capabilities/default.json`)**:
    - Tạo cửa sổ `"hud"` kích thước `360x92`, trong suốt (`transparent`), luôn nổi trên cùng (`always_on_top`), không chiếm focus (`focusable(false)`), đặt tại **chính giữa cạnh trên màn hình chính**.
    - Hiển thị spinner + `Converting 1/2 → JPG` + `%` tiến độ + thanh progress bar mượt; khi xong hiện tích xanh `✓ Converted 2/2 files → JPG · 100%` rồi tự ẩn sau 2.4 giây.
  - **`src/radial.js`**:
    - Khi chọn nhiều file, tâm vòng xoay hiện rõ `2 files` (hoặc `N files`).
    - Ngay khi click chọn định dạng trên vòng xoay, **ẩn ngay vòng xoay** (`appWindow.hide()`) để quá trình convert chạy ngầm hoàn toàn và nhường hiển thị cho thanh HUD phía trên màn hình.

---

## 3. Cấu trúc File quan trọng cần biết
- `src-tauri/src/lib.rs` — Khởi tạo Tauri app, khai báo 3 cửa sổ (`settings`, `radial`, `hud`), đăng ký commands & plugins.
- `src-tauri/src/selection.rs` — Đọc file đang chọn từ Windows Explorer (`IShellItemArray` + `SIGDN_FILESYSPATH`) và macOS Finder.
- `src-tauri/src/hotkey.rs` — Bắt phím tắt (`Ctrl+Shift+C` / giữ `Shift`), định vị cửa sổ `radial` theo con trỏ chuột và cửa sổ `hud` ở giữa đỉnh màn hình (`show_hud`).
- `src-tauri/src/convert_cmd.rs` — Xử lý lệnh `convert_any_cmd` (hỗ trợ multi-file batch, emit `"convert-progress"`, AI `remove_bg_image` bằng ONNX).
- `src-tauri/src/radial_model.rs` — Cấu hình 5 nhóm định dạng (`Images`, `Documents`, `Data`, `PDF Tools`, `Audio & Video`).
- `crates/engine/src/lib.rs`, `office.rs`, `heic.rs`, `av.rs`, `data.rs`, `docs.rs` — Engine xử lý chuyển đổi file.
- `src/radial.html`, `src/radial.js`, `src/styles.css` — Giao diện vòng xoay 1 tầng (Single-Ring).
- `src/hud.html` — Giao diện thanh tiến độ nổi ở góc trên chính giữa màn hình.
- `src/settings.html`, `src/settings.js` — Giao diện cài đặt.

---

## 4. Môi trường, Dữ liệu & Log trên máy
- **Thư mục AppData Roaming**: `C:\Users\Ken Dao\AppData\Roaming\NeelFlow.PizzaConverter\`
  - `convert.log`: Log chi tiết mọi lần convert (`OK` / `ERR`, danh sách file đầu vào, file đầu ra). **Luôn đọc file này đầu tiên khi debug lỗi convert.**
  - `license.json`: Trạng thái mở khóa Pro (`{"pro": true, "source": "full_unlock"}`).
  - `radial_config.json`: Cấu hình các lát cắt trên vòng xoay và phím tắt.
  - `models\isnet-general-use.onnx`: Model AI tách nền (`178.6 MB`).
- **Python 3.12**: `C:\Users\Ken Dao\AppData\Local\Programs\Python\Python312\python.exe`
  - Các thư viện đã cài: `onnxruntime`, `pillow`, `pillow-heif`, `pypdfium2`, `pypdf`, `reportlab`, `python-docx`, `rapidocr-onnxruntime`, `openpyxl`, `pyyaml`.
- **FFmpeg**: `C:\Users\Ken Dao\AppData\Local\ffmpeg-shared\ffmpeg-8.1-full_build-shared\bin\ffmpeg.exe`.

---

## 5. Quy trình Build, Cài đặt & Kiểm thử tự động (Dành cho Agent tiếp theo)

### 5.1. Build & Cài đặt bản mới
```powershell
# 1. Tắt tiến trình đang chạy
Stop-Process -Name "pizza-converter" -Force -ErrorAction SilentlyContinue

# 2. Build bộ cài NSIS
cd "C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\src-tauri"
cargo tauri build --bundles nsis

# 3. Cài đặt silent (/S) và khởi chạy lại
$installer = "C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\src-tauri\target\release\bundle\nsis\Pizza Converter_0.1.0_x64-setup.exe"
Start-Process -FilePath $installer -ArgumentList "/S" -Wait
Start-Process -FilePath "C:\Users\Ken Dao\AppData\Local\Pizza Converter\pizza-converter.exe"
```

### 5.2. Chạy E2E Test toàn bộ 57 kịch bản convert
Script kiểm thử tự động nằm tại `C:\Users\Ken Dao\agent-work\pizza-fix\verify-all-formats.js` (dùng `tauri-driver` + `msedgedriver` để gọi trực tiếp `window.__TAURI__.core.invoke('convert_any_cmd', ...)` trên binary release):
```powershell
cd "C:\Users\Ken Dao\agent-work\pizza-fix"
node verify-all-formats.js
```

## 6. Nguyên tắc bắt buộc khi tiếp tục phát triển
1. **Không làm hỏng cấu hình Tauri Bridge**: Giữ nguyên `"withGlobalTauri": true` trong `src-tauri/tauri.conf.json` và quyền của cả 3 cửa sổ `["settings", "radial", "hud"]` trong `src-tauri/capabilities/default.json`.
2. **Không làm hiện cửa sổ CMD đen**: Mọi lệnh gọi `std::process::Command` trên Windows bắt buộc phải có `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`).
3. **Thư mục làm việc tạm**: Mọi script test, file mẫu, hoặc artifact trung gian đặt trong `C:\Users\Ken Dao\agent-work\pizza-fix\`, tuyệt đối không xả file rác ra `Desktop`.

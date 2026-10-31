# QUY TRÌNH TEST BUILD — Pizza Converter
**QA & Phát hành · 29/10/2026 · cập nhật tên 29/10 ("Pizza Converter" đã được anh Ken giữ trên Partner Center) · dành cho anh Ken (bản test nội bộ, không public, không lên Store)**

## Nguyên tắc
- Mỗi bản test QA đã chạy **smoke test** trước khi gửi anh: cài sạch được, mở được, icon hiện ở system tray, giữ Shift hiện menu tròn, convert thử 1 file thật, gỡ sạch không rác.
- Mỗi bản test **luôn kèm "Ghi chú bản test"** (mục C) ghi rõ: số bản, cài đè được không, test cái gì, gửi feedback ở đâu.
- File test **chỉ gửi riêng cho anh Ken**, không upload Store, không gửi link công khai.

## A. Test trên Windows — file .msix (nháy đúp là cài)
**Lần đầu tiên (làm 1 lần duy nhất):**
1. QA gửi 2 file: `Pizza-Converter-0.x.y.msix` và `PizzaConverterTest.cer`.
2. Cài chứng chỉ test: chuột phải file `PizzaConverterTest.cer` → **Install Certificate** → chọn **Local Machine** (máy hỏi quyền admin thì bấm Yes) → **Place all certificates in the following store** → **Browse** → chọn **Trusted People** → OK → Next → Finish → bấm OK khi báo thành công.
   - Windows 11: không cần bật thêm gì.
   - Windows 10: nếu bị chặn, vào Settings → Update & Security → For developers → bật **Sideload apps**.
3. Nháy đúp file `.msix` → bấm **Install**.
4. Mở app: Start menu gõ "Pizza Converter". App chạy nền ở **system tray** (mũi tên góc phải taskbar).

**Các bản test sau:** chỉ cần nháy đúp file `.msix` mới → tự **nâng cấp đè lên bản cũ, giữ nguyên cài đặt**. Không cần cài lại chứng chỉ.

**Gỡ app:** Settings → Apps → Installed apps → "Pizza Converter" → Uninstall (gỡ sạch).

## B. Test trên iMac — file .dmg (đã kiểm duyệt Apple)
1. QA gửi file `Pizza-Converter-0.x.y.dmg`.
2. Nháy đúp file `.dmg` → kéo biểu tượng **Pizza Converter** vào thư mục **Applications**.
3. Mở từ Applications. Lần đầu nếu máy hỏi, chuột phải vào Pizza Converter → **Open** → **Open** (bản đã notarized nên thường mở thẳng không hỏi).
4. Bản test sau: kéo đè vào Applications → chọn **Replace**.

**Gỡ app:** kéo Pizza Converter từ Applications vào Trash.

## C. Mẫu "Ghi chú bản test" (QA gửi kèm mỗi bản)
```
BẢN TEST: Pizza Converter 0.1.0 (build 3) — Windows MSIX
Ngày gửi: 30/10/2026 · QA smoke: PASS

Cài đè: ĐƯỢC (tự nâng cấp, giữ nguyên cài đặt)
  # nếu KHÔNG: ghi "Gỡ bản cũ trước: Settings → Apps → Uninstall"

TEST GIÚP EM:
1. Giữ Shift 0.5s khi đang chọn 1 ảnh trong Explorer → menu tròn có hiện quanh chuột không
2. Flick sang lát JPG → file JPG có xuất hiện cùng thư mục không
3. Bấm Esc giữa chừng → có hủy đúng không
KHÔNG TEST BẢN NÀY: audio/video, tiếng Việt (chưa có)

GỬI FEEDBACK: nhắn trong chat chính cho em là được.
Ghi giúp em: máy nào (PC/iMac), đang làm gì thì bị, chụp màn hình nếu được.
```

## D. Quy định nội bộ QA (không gửi anh Ken)
- Đánh số bản tăng dần, không đặt lại. MSIX dùng version 4 phần (`0.1.0.0`); DMG dùng semver (`0.1.0`).
- MSIX **không cho cài bản cũ đè bản mới** — muốn về bản cũ phải gỡ rồi cài lại; ghi rõ trong ghi chú bản test.
- **GATE BẮT BUỘC (lệnh anh Ken 29/10): test matrix PASS 100% gate v1 (181 cases; 9 case xuất HEIC → v2, loại khỏi gate) mới được gửi bản test cho anh Ken.**
  Chạy: `cd workspace/convertx/qa-release/test-matrix && python3 run-matrix.py --bin pizza-convert --out results/build-00X`
  (cần Dev cung cấp CLI ẩn `pizza-convert <input> <output>` gọi đúng engine thật của app).
  Case lỗi → báo Dev sửa ngay, không để lọt; chạy lại tới khi 181/181 (v1).
- Trước khi gửi anh Ken: smoke test trên VM/máy test sạch (cài → chạy → convert 1 file → gỡ), chụp màn hình menu tròn làm bằng chứng pass.
- Tuyệt đối không đẩy bất kỳ bản nào lên Store khi chưa có lệnh anh Ken qua Điều phối.

## E. Test mua Pro $19.99 one-time bằng IAP sandbox (nội bộ QA, không gửi anh Ken)
Giá đã chốt: **$19.99, mua một lần, KHÔNG subscription** (Điều phối chốt 29/10, quyền anh Ken).
- Dùng **tài khoản test sandbox của Partner Center** (không trừ tiền thật). Bản test sideload KHÔNG gọi được Store IAP thật — Dev phải có build trỏ sandbox theo tài liệu Microsoft (Windows.Services.Store).
- Checklist mỗi lần test: (1) mua Pro $19.99 trong Settings → Upgrade; (2) xác nhận mở khóa: GIF, xuất AVIF/TIFF, HEIC→JPG/PNG, PDF tools (trích/nén/tách/gộp), Audio·Video, image→PDF; (2b) xác nhận Free bị khóa đúng: GIF/AVIF/TIFF/PDF tools/Audio·Video hiện khóa hoặc báo nâng cấp khi chưa mua; (3) gỡ app → cài lại → **Restore purchase** phải trả lại Pro không mua lần 2; (4) chụp màn hình hóa đơn sandbox $19.99 làm bằng chứng.
- Khi nào test: sau khi gate matrix 190/190 pass và Dev báo IAP đã nối sandbox. Kết quả báo Điều phối trước khi gửi bản test có IAP cho anh Ken.

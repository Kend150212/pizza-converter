# FFmpeg licensing — Pizza Converter (nghiên cứu 29/10/2026)

App: Tauri v2, closed-source, bán trên Microsoft Store (MSIX fullTrust), có bản Pro one-time.
Mục đích dùng ffmpeg: convert audio/video (MP3/WAV/FLAC/OGG/M4A/MP4/MKV/WEBM) + HEIC→JPG/PNG.
Lưu ý: đây là nghiên cứu kỹ thuật, KHÔNG phải tư vấn luật. Trước khi launch nên cho luật sư xem 1 lượt.

## 1. FFmpeg có 2 "mức" license — chọn sai là dính đòn

- **LGPL v2.1+** (mặc định): core libav* + đa số codec/filter. Dùng được trong app
  closed-source thương mại NẾU tuân thủ điều kiện (mục 3).
- **GPL**: bật lên khi build có `--enable-gpl` — kéo theo libx264 (encode H.264),
  libx265 (encode H.265), xvid... Dùng build GPL trong app closed-source =
  **toàn bộ app phải open-source theo GPL**. CẤM dùng.
- FFmpeg KHÔNG bán license thương mại dưới bất kỳ hình thức nào (ffmpeg.org nói rõ,
  kể cả trả tiền cũng không có).

**Build nào an toàn:** BtbN/FFmpeg-Builds trên GitHub có bản **`win64-lgpl`**
(hoặc `win64-lgpl-shared`) làm riêng cho mục đích này — "the one your lawyer will
ask for". **Tránh GyanD** (tất cả build của họ đều là GPLv3 static — không dùng được).

## 2. Build LGPL mất gì so với build GPL?

Mất encoder: **libx264, libx265** (encode H.264/H.265). Còn lại đủ cho ma trận của app:

| Nhu cầu app | Trạng thái trong build LGPL |
|---|---|
| Đọc MP4/H.264/H.265 (decode) | OK — decoder H.264/H.265 native là LGPL |
| Ghi MP4 | OK qua encoder `mpeg4` native (LGPL) + `aac` native (LGPL). File to hơn x264 nhưng phát được mọi nơi |
| MP3 | OK — libmp3lame là LGPL |
| WAV/FLAC/OGG Vorbis/Opus | OK — encoder native, LGPL |
| WEBM (VP8/VP9) | OK — libvpx là BSD |
| MKV/M4A (container) | OK |
| HEIC → JPG/PNG (decode) | OK — ffmpeg ≥5.1 demux/decode ảnh HEIC tĩnh bằng native HEVC decoder (LGPL), không cần libheif |

Mẹo: hardware encoder (h264_nvenc/qsv/amf) là LGPL (chỉ gọi API của hệ thống) và
thường có sẵn trong build BtbN → trên máy có GPU vẫn encode H.264 được; máy không
có GPU thì fallback về mpeg4. Encode H.265 thuần túy thì không có trong LGPL.

## 3. Điều kiện tuân thủ LGPL khi bundle (checklist theo ffmpeg.org/legal.html)

Cách làm sạch nhất cho app này: **bundle `ffmpeg.exe` (build LGPL) như 1 tiến trình
riêng, app gọi qua command line** — không link library vào code Rust. Giao tiếp
qua CLI được xem là "arms-length", không trigger điều khoản linking.

Việc bắt buộc phải làm:
1. Dùng build biên dịch **không** `--enable-gpl`, **không** `--enable-nonfree`
   (build `win64-lgpl` của BtbN đã đúng).
2. **Phân phối source code ffmpeg** đúng phiên bản với binary đi kèm (hoặc cam kết
   văn bản cung cấp trong 3 năm). Thực tế: host file zip source trên web của
   Neeflow + ghi link trong app.
3. **Attribution 3 nơi**: hộp About trong app ("This software uses FFmpeg licensed
   under LGPLv2.1..."), EULA, và trang web chỗ có link tải app.
4. Trong EULA **không được cấm reverse engineering** (điều khoản LGPL yêu cầu);
   ghi rõ phần nào là code của mình, phần nào là FFmpeg + chủ sở hữu.
5. Viết đúng chính tả "FFmpeg" (hai chữ F hoa).
6. Ghi lại configure line / nguồn build để chứng minh build sạch.

App gốc (Converit) bundle `avcodec-63.dll` — đó là cách **shared library**
(dynamic linking), cũng là 1 cách tuân thủ LGPL hợp lệ (miễn là họ làm đủ mục
2–6 trên). Mình chọn cách gọi `ffmpeg.exe` riêng còn đơn giản hơn.

## 4. Bundle sẵn vs tải khi dùng lần đầu

| Tiêu chí | Bundle sẵn trong installer | Tải khi dùng lần đầu |
|---|---|---|
| Rủi ro license | Thấp — làm đủ checklist mục 3 là xong | Thấp về license, nhưng... |
| Microsoft Store policy | An toàn | **RỦI RO**: policy 10.2.2 cấm app tải "remote executable code" làm thay đổi/mở rộng chức năng. ffmpeg.exe tải về rồi chạy = vùng xám, dễ bị đánh trượt certification |
| Trải nghiệm | Chạy offline ngay, không chờ | Lần đầu cần mạng, chờ tải ~100MB, lỗi mạng = tính năng chết |
| Dung lượng installer | +~80–115MB (vd thực tế: app Lyrenne từ 162MB lên 206MB khi bundle BtbN LGPL) | Nhẹ |
| Cập nhật ffmpeg | Theo bản release app | Tự linh hoạt, nhưng cũng là rủi ro bảo mật/quản lý |

Kết luận: **bundle sẵn thắng áp đảo** cho app bán trên Store.

## 5. Có thay ffmpeg bằng platform API được không?

- **Windows Media Foundation**: H.264/AAC/MP3/WMA — KHÔNG có FLAC, OGG/Vorbis,
  Opus, WebM/VP9, MKV đầy đủ.
- **macOS AVFoundation**: H.264/H.265/AAC/MP3/ALAC — KHÔNG có FLAC, OGG, WebM, MKV.
- Kết luận: **không phủ được ma trận** (thiếu FLAC/OGG/WEBM/MKV). FFmpeg vẫn cần.
  (Có thể dùng platform API sau này như optimization cho riêng H.264, không bắt buộc.)

## 6. Đề xuất cho Ken chọn

**Phương án đề xuất: BUNDLE SẴN build `win64-lgpl` của BtbN (ffmpeg.exe static),
app gọi qua command line, + làm đủ checklist LGPL mục 3.**

- Chi phí: $0. Dung lượng installer tăng ~100MB (chấp nhận được với app desktop).
- Chạy offline 100%, qua được Store certification, không dính GPL.
- Đánh đổi duy nhất: file MP4 xuất ra dùng codec mpeg4 (to hơn H.264) trừ khi máy
  có GPU (dùng hardware encoder). Với app convert file thông dụng, đây là đánh đổi
  hợp lý — và đúng luật.
- Việc cần làm khi implement: pin version ffmpeg (vd 8.1.x LGPL), host source zip
  trên web Neeflow, thêm attribution vào About/EULA/Store listing, rà EULA bỏ điều
  khoản cấm reverse engineering.

Phương án tải khi dùng lần đầu: **không khuyến nghị** (rủi ro Store policy 10.2.2 +
trải nghiệm lần đầu tệ).

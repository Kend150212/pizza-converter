# TEST MATRIX — Pizza Converter
**QA & Phát hành · 29/10/2026 · cập nhật tên 29/10 · Gate bắt buộc trước khi gửi bản test cho anh Ken**

Mỗi case: file mẫu thật (thư mục `samples/`) → convert → kiểm chứng file ra **mở được + đúng định dạng** (magic bytes + parse/decode, script `verify.py`).
Tag **Pro** = cần mở khóa Pro mới dùng được (Điều phối chốt cuối 29/10 theo mockup anh Ken duyệt).

## 1. Images — 9 định dạng × 9 đích = 81 cases
Input: `sample.png/.jpg/.webp/.avif/.bmp/.ico/.gif/.tif/.heic`
- Free: PNG, JPG, WEBP, BMP, ICO
- **Pro: AVIF, TIFF, HEIC, GIF** (GIF chốt Pro 29/10)
- Mỗi input convert sang cả 9 đích (kể cả chính nó = re-encode).
- **Cột HEIC-output (9 cases) → v2** (Điều phối chốt 29/10, quyền anh Ken: nhu cầu thực là đọc TỪ ảnh iPhone, xuất RA heic gần như không ai cần; libheif đã loại vì license). Runner mặc định BỎ QUA các case này (`--include-v2` để chạy).
- Lưu ý: ICO ra nhiều size trong 1 file; GIF mẫu 1 frame (GIF động → lấy frame đầu là đạt); HEIC cần thiết bị/codec đọc được.

| Input \ Output | PNG | JPG | WEBP | AVIF* | BMP | ICO | GIF* | TIFF* | HEIC* |
|---|---|---|---|---|---|---|---|---|---|
| PNG | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| JPG | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| WEBP | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| AVIF* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| BMP | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ICO | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| GIF | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| TIFF* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| HEIC* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
(* = Pro)

## 2. Documents — 6 × 6 = 36 cases (cần LibreOffice cài ngoài cho DOC/DOCX/ODT/RTF)
Input: `sample.txt/.docx/.pdf/.doc/.odt/.rtf` — mỗi input sang cả 6 đích.
| Input \ Output | TXT | DOCX | PDF | DOC | ODT | RTF |
|---|---|---|---|---|---|---|
| TXT | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| DOCX | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| PDF | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| DOC | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| ODT | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| RTF | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

## 3. Data — 5 × 5 = 25 cases
Input: `sample.xlsx/.csv/.json/.xml/.yaml` — mỗi input sang cả 5 đích. Kiểm chứng: parse được + đủ dữ liệu (số dòng/bản ghi khớp mẫu).
| Input \ Output | XLSX | CSV | JSON | XML | YAML |
|---|---|---|---|---|---|
| XLSX | ✓ | ✓ | ✓ | ✓ | ✓ |
| CSV | ✓ | ✓ | ✓ | ✓ | ✓ |
| JSON | ✓ | ✓ | ✓ | ✓ | ✓ |
| XML | ✓ | ✓ | ✓ | ✓ | ✓ |
| YAML | ✓ | ✓ | ✓ | ✓ | ✓ |

## 4. PDF tools — 6 cases (Pro toàn bộ — chốt cuối 29/10: merge/split/compress = Pro; trích DOCX/TXT/PNG = Pro theo brief)
P1–P3 (trích) chạy qua `pizza-convert` như case thường; P4–P6 tự động qua cờ CLI: `--compress`, `--split DIR`, `--merge` (đã nối trong `run-matrix.py`).
| # | Thao tác | Input | Kiểm chứng |
|---|---|---|---|
| P1 | PDF → DOCX | `sample.pdf` | mở được, còn chữ |
| P2 | PDF → TXT | `sample.pdf` | text trích ra khớp nội dung mẫu |
| P3 | PDF → PNG | `sample.pdf` | ảnh render đúng số trang |
| P4 | Compress PDF | `sample.pdf` | file ra < file vào, vẫn mở được |
| P5 | Split PDF | `sample-3pages.pdf` | ra 3 file, mỗi file 1 trang đúng thứ tự |
| P6 | Merge PDF | `sample.pdf` + `sample-3pages.pdf` | 1 file, đủ số trang |

## 5. Audio — 5 × 5 = 25 cases (Pro, cần ffmpeg)
Input: `sample.mp3/.wav/.flac/.ogg/.m4a` (mỗi file 2 giây sine 440Hz) — mỗi input sang cả 5 đích. Kiểm chứng: ffprobe đọc được stream audio, duration ≈ 2s.
| Input \ Output | MP3 | WAV | FLAC | OGG | M4A |
|---|---|---|---|---|---|
| MP3 | ✓ | ✓ | ✓ | ✓ | ✓ |
| WAV | ✓ | ✓ | ✓ | ✓ | ✓ |
| FLAC | ✓ | ✓ | ✓ | ✓ | ✓ |
| OGG | ✓ | ✓ | ✓ | ✓ | ✓ |
| M4A | ✓ | ✓ | ✓ | ✓ | ✓ |

## 6. Video — 3 × 3 = 9 cases (Pro, cần ffmpeg)
Input: `sample.mp4/.mkv/.webm` (2 giây, 320×240) — mỗi input sang cả 3 đích. Kiểm chứng: ffprobe đọc được stream video (+audio nếu có), duration ≈ 2s.
| Input \ Output | MP4 | MKV | WEBM |
|---|---|---|---|
| MP4 | ✓ | ✓ | ✓ |
| MKV | ✓ | ✓ | ✓ |
| WEBM | ✓ | ✓ | ✓ |

## 7. Cross mở rộng — 2 cases (Pro)
| # | Cặp | Kiểm chứng |
|---|---|---|
| X1 | MP4 → MP3 (tách tiếng) | file mp3 có stream audio, duration ≈ video gốc |
| X2 | MP4 → WAV (tách tiếng) | file wav PCM, duration ≈ video gốc |

## 8. Office → PDF — 6 cases (vượt app gốc, lệnh anh Ken 29/10 11:40)
| # | Cặp | Kiểm chứng |
|---|---|---|
| O1 | DOCX → PDF | file PDF mở được, còn chữ/bảng |
| O2 | XLSX → PDF | file PDF mở được, còn bảng |
| O3 | PPTX → PDF | file PDF mở được, còn slide |
| O4 | ODT → PDF | file PDF mở được |
| O5 | RTF → PDF | file PDF mở được |
| O6 | TXT → PDF | file PDF mở được |

**TỔNG: 81 + 36 + 25 + 6 + 25 + 9 + 2 + 6 = 190 cases.** Gate v1: **181/190** (9 case xuất HEIC → v2, loại khỏi gate). Tiêu chí pass v1: 181/181, không sót format nào trong phạm vi v1 (lệnh anh Ken 11:04 ET + mở rộng 11:45 ET + chốt phạm vi 12:1x ET).

## Cách chạy
```
cd workspace/convertx/qa-release/test-matrix
python3 make-samples.py            # tạo file mẫu (làm 1 lần)
python3 run-matrix.py --selfcheck  # tự kiểm: mỗi file mẫu phải verify đúng định dạng của nó
python3 run-matrix.py --bin pizza-convert --out results/build-001/
# --bin: lệnh convert của Dev, cú pháp: pizza-convert <input> <output>
# Kết quả ghi vào results/<build>.json ; case lỗi in ra cuối để báo Dev sửa ngay
```
**Đề xuất cho Dev (để QA tự động hóa được):** app cần có lệnh CLI ẩn `pizza-convert <input> <output>` (hoặc Tauri command test-only) gọi đúng engine convert thật của app. Không có CLI thì matrix 184 cases phải test tay — chậm và dễ sót.

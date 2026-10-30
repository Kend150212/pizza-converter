# Third-Party Open Source Software Notices

Pizza Converter ("the Software") makes use of several third-party open-source components, libraries, and frameworks. We are deeply grateful to the open-source community for their contributions.

---

### 1. FFmpeg
- **License**: GNU Lesser General Public License (LGPL) version 2.1 or later
- **Website**: https://ffmpeg.org
- **Usage**: Audio and video format transcoding, container re-multiplexing, and codec manipulation.
- **Compliance Notice**: FFmpeg is invoked strictly as a separate, dynamically launched command-line executable (`ffmpeg.exe`). No proprietary code is statically linked to FFmpeg libraries. Pizza Converter complies with the terms of LGPL v2.1. The complete source code of FFmpeg, as well as compilation instructions, can be downloaded directly from the official FFmpeg project website.

---

### 2. ONNX Runtime
- **License**: MIT License
- **Copyright**: Copyright (c) Microsoft Corporation. All rights reserved.
- **Website**: https://github.com/microsoft/onnxruntime
- **Usage**: Embedded local machine learning inference for neural network-based background removal and document edge segmentation.

---

### 3. OpenCV (Open Source Computer Vision Library)
- **License**: Apache License, Version 2.0
- **Copyright**: Copyright (c) OpenCV team and contributors.
- **Website**: https://opencv.org
- **Usage**: Local image processing, perspective transformation, and adaptive contrast scanning filters.

---

### 4. pypdf
- **License**: BSD 3-Clause License
- **Copyright**: Copyright (c) 2006-2008 Mathieu Fenniak; Copyright (c) 2007 Ashish Kulkarni; Copyright (c) 2014 Publysher; Copyright (c) 2022-2026 Martin Thoma and pypdf contributors.
- **Website**: https://github.com/py-pdf/pypdf
- **Usage**: Local PDF encryption, metadata cleaning, restriction removal, and multi-file merging.

---

### 5. pdfplumber
- **License**: MIT License
- **Copyright**: Copyright (c) 2015 Jeremy Singer-Vine and contributors.
- **Website**: https://github.com/jsvine/pdfplumber
- **Usage**: Extracting text layout, tabular structures, and converting PDF documents into structured Markdown format.

---

### 6. Tauri Framework & Ecosystem
- **License**: MIT License or Apache License 2.0
- **Copyright**: Copyright (c) 2019-2026 Tauri Programme within The Commons Conservancy.
- **Website**: https://tauri.app
- **Usage**: Core desktop application framework bridging Rust backend and HTML5/CSS/JavaScript webview frontends.

---

### 7. rdev
- **License**: MIT License
- **Copyright**: Copyright (c) 2020 Narsil
- **Website**: https://github.com/Narsil/rdev
- **Usage**: Low-level cross-platform global keyboard shortcut detection.

---

### License Texts Summary

#### GNU Lesser General Public License (LGPL) v2.1
A copy of the GNU Lesser General Public License v2.1 can be obtained at:
https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html

#### Apache License 2.0
Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at:
http://www.apache.org/licenses/LICENSE-2.0

#### MIT License
Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software.

# 03 — C++ → Rust, Python → Rust và cách đóng gói

Nguồn official mở ngày **2026-10-29**. Phần thiết kế boundary là đề xuất cho spike,
không phải implementation đã có trong repo.

## “Build sang Rust” có ba nghĩa khác nhau

| Ý định | Thực tế khả thi | Dependency còn lại |
| --- | --- | --- |
| Gọi thư viện C++ từ Rust | Bind/link native object/library, qua skia-safe, C ABI hoặc cxx. | Machine code C++, runtime/stdlib và dependency đã link; có thể static hoặc dynamic. |
| Gọi Python từ Rust | Embed CPython qua PyO3 hoặc chạy Python worker con. | Interpreter + stdlib/packages + native extension libraries. |
| Port/transpile toàn implementation sang Rust | Viết/kiểm chứng lại thuật toán, hoặc dùng port thực như tiny-skia/HarfRust. | Dependency graph của port; không có bridge tự làm việc này. |

C++ có thể compile thành object/static archive rồi link vào Rust executable:
không cần một C++ interpreter chạy riêng. Nhưng executable vẫn chứa C++ code,
có thể còn dynamic libstdc++/libc++ hoặc OS libraries; “một binary” không đồng
nghĩa Rust thuần hay không có dependency runtime.

Python bytecode, embedding hay đóng bundle không làm Pillow/Cairo/Skia biến thành
Rust. Phải phân biệt không cần Python **cài sẵn ngoài máy** với không có Python
**bên trong gói sản phẩm**.

## C++ qua skia-safe, custom C ABI hoặc cxx

### Dùng binding sẵn trước

Nếu lựa chọn là Skia, ưu tiên skia-safe cho spike vì đã có wrapper/API/build
integration. Không viết lại toàn Skia bindings chỉ để đổi ngôn ngữ ở entry point.
Adapter OpenCanvas nên có boundary nhỏ: nhận scene/resources/options, trả metric
hoặc bytes + metadata/error; không đưa domain storage/revision vào C++.

### Custom C ABI khi wrapper thiếu API thực sự cần

- Viết shim C++ `extern "C"` quanh API; dùng opaque handles và DTO `repr(C)`
  với scalar/pointer/length. Không truyền Rust String/Vec hoặc C++ std::string/
  STL container theo layout tự đoán. Version boundary và kiểm overflow/length.
- Chốt lifetime/ownership: bên cấp phát phải có hàm giải phóng tương ứng; buffer
  borrowed chỉ sống trong call; tránh giữ pointer vào Vec có thể reallocate.
- Catch C++ exceptions trong shim và trả error; không để exception/panic vượt
  boundary ngoài ABI contract. Kiểm null/error, thread affinity/Send/Sync thật;
  Rust type wrapper không tự làm foreign library thread-safe.
- C ABI giảm coupling layout C++ công khai, **không xóa C++ ABI ở bên trong**:
  shim và Skia vẫn cần compiler/stdlib/flags tương thích. [Rust FFI guide](https://doc.rust-lang.org/nomicon/ffi.html).

### cxx khi kiểm soát cả Rust/C++ source

[CXX](https://cxx.rs/) sinh hai phía bridge từ signature/types được hỗ trợ,
giúp kiểm invariants; không chuyển implementation C++ thành Rust. Tích hợp
[cxx-build](https://cxx.rs/build/cargo.html) vẫn compile file C++ bằng native
toolchain. Bọc API nhỏ sẽ dễ pin/test hơn expose template-heavy Skia headers.

Exception translation có quy tắc cụ thể: extern C++ trả `Result` có thể chuyển
exception thành Rust error; call không khai báo Result mà throw thì terminate.
Panic qua extern Rust boundary của CXX abort process.
[CXX Result](https://cxx.rs/binding/result.html). Không hứa “safe bridge” bắt mọi
segfault, OOM, UB hoặc thư viện bên trong.

## Python qua PyO3 embedding hoặc subprocess

### PyO3 embedding

[PyO3](https://github.com/PyO3/pyo3) là bindings interpreter, MIT OR Apache-2.0.
Rust có thể khởi tạo CPython, import module và gọi functions; exception Python
map thành Rust error khi xử lý đúng. Chi phí/copy buffer tùy API; không có cơ sở
kết luận “FFI gần zero nên toàn pipeline nhanh” trước profiler.

[Build/distribution guide](https://pyo3.rs/main/building-and-distribution) phân
biệt Rust extension module được Python import và Rust binary embed Python.
Dynamic embedding link libpython; phải ship/locate đúng shared library, stdlib,
site-packages và extension `.so`/`.dylib`. Static embedding có nhiều giới hạn,
**chưa first-class support**; compiled extensions cần symbols/linker flags.
Không dùng `abi3` của extension module như lời hứa binary embedding + tất cả
wheels chạy qua mọi Python version/OS/architecture.

CPython thông thường có GIL: phần Python giữ GIL cần serialize/attach đúng;
native extension có thể nhả GIL cho một số operations, không được đoán là mọi
Pillow/Skia call đều nhả. Free-threaded CPython đã có và PyO3 hỗ trợ, nhưng
extension compatibility phải kiểm riêng; shared state vẫn cần synchronization.
[PyO3 free-threading](https://pyo3.rs/main/free-threading.html). Không coi đổi sang
free-threaded build là tối ưu sẵn có của OpenCanvas.

Với Pillow, assert `features.check_feature("raqm")` khi kiểm complex text;
không âm thầm chạy BASIC. Với Pycairo, complex layout còn cần Pango; với
skia-python vẫn là native Skia, không lấy drawing-string API làm shaper đủ.
Các khả năng/requirements cụ thể đã dẫn ở [sàng lọc](02-candidates.md).

### Subprocess Python

Rust → Python worker persistent, protocol bounded như worker Node hiện tại.
Ưu điểm là exception/crash/process memory tách khỏi server, deadline có thể kill;
nhược điểm là IPC/copies/startup và thêm interpreter/package distribution.
Batch whole job/measurement, tránh từng glyph qua IPC. stdout chỉ protocol,
stderr bounded/redacted. Font/assets theo bytes/hash hoặc ID nội bộ đã validate,
không nhận đường dẫn tùy ý từ API client.

Nếu nhóm đã có renderer Python được kiểm chứng, phương án này có thể tiết kiệm
công viết lại. Repo hiện chưa có renderer đó; thêm Python chỉ để gọi Cairo/Skia
là dependency layer thêm cần lý do rõ. Native Rust binding tới cùng C/C++ engine
là đối chứng công bằng, không mặc định Python chậm ở operations làm bằng native.

### Compile/transpile Python: sàng lọc ngắn

| Cách | Đã xác minh official | Kết luận cho bài toán này |
| --- | --- | --- |
| [Cython](https://cython.readthedocs.io/en/latest/src/quickstart/overview.html) | Dịch source sang C/C++ extension, chạy trong CPython runtime. | Không bỏ CPython/native graphics dependencies, không tạo Rust engine. |
| [Nuitka](https://nuitka.net/user-documentation/user-manual.html) | Compiler gắn CPython, cần C toolchain; standalone/onefile có thể phân phối không cần Python cài sẵn. | Có thể giải quyết bundling, không biến algorithms/dependencies thành Rust; chưa chứng minh footprint/parity tốt hơn native binding. |
| [RustPython](https://github.com/RustPython/RustPython) | Python interpreter viết bằng Rust, có hướng embed/WASM. | Không phải Python→Rust source transpiler. Chưa xác minh Pillow/Pycairo/skia-python extension compatibility; không chọn làm lối tắt thay CPython. |

Không mở nhánh compiler/interpreter migration chỉ để tránh viết một scene adapter:
nó thêm matrix compatibility mà vẫn cần kiểm text/pixels như engine native.

## Cross build, ABI và vận hành

| Nhóm | Phải pin/build/ship | Điểm cần chứng minh trên macOS và Linux arm64 |
| --- | --- | --- |
| Rust CPU | Cargo.lock, exact features, codecs/shaper/font graph, CPU baseline | SIMD và floating-point output, WOFF2 decoder, deployment libc; không bật `target-cpu=native` cho binary phân phối chung. |
| skia-safe/C++ | Compiler/SDK, target triple, Skia revision, feature set, static archives hoặc dynamic libs | Prebuilt hash/licensing; fallback source build cần toolchain/cache/network; cross compile không chỉ `rustup target add`. |
| Cairo/Pango | C libraries và headers/pkg-config khi build, runtime sonames, fontconfig/FreeType/GLib | Pin Docker distro packages; macOS rpath/library discovery; không suy Linux screenshot bằng macOS. |
| Qt | Qt GUI + plugins/image codecs/fonts, C++ build tools, licenses | Headless QImage/QTextLayout startup và platform plugin requirements phải thử; không tự coi QGuiApplication init luôn miễn display config. |
| PyO3/Python | Target CPython library/interpreter, stdlib/packages, wheels/native libraries | Host Python phục vụ build khác target Python; wheel Python tag/ABI/platform; rpath/soname, free-threaded ABI riêng. |
| Rust CDP | Rust driver + pinned Chromium binary/OS libs + renderer ESM/fonts | CDP/browser protocol match, signed app/macOS distribution, Linux sandbox/process cleanup; không loại JS engine. |

Linux glibc Docker và macOS là **hai bản build**; Apple arm64 binary không chạy
native trong Linux arm64 container. Test image CPU-only riêng thay vì suy target
arm64 giống nhau. Fonts proprietary/system fonts không copy vào image nếu chưa
có quyền; dùng redistributable corpus fonts pinned và kiểm license từng file.

## Failure isolation là quyết định độc lập với ngôn ngữ

Ưu tiên spike native engine trong **child process**, giữ Rust server là owner
queue/jobs/DB. Đây là đề xuất isolation, không kiến trúc microservices:

1. Một worker warm, protocol compute/render/resources bounded, ID/version handshake.
2. Deadline ở parent; timeout kill/reap process tree, mark job failed, next job
   restart. Timeout Future không dừng một C++ call/CPU task đang chạy trong process.
3. OOM/segfault/abort của FFI không được đưa server xuống. `catch_unwind` không
   bắt segfault/C++ UB/OOM abort; Rust thuần cũng có thể OOM/abort.
4. Cap decoded bytes/pixels/layers/caches và output trước allocate/IPC; native
   library API thành công không thay parser validation hoặc resource hash checks.
5. Worker không sở hữu writer lock/DB mutations. Crash/rollback renderer không
   thay revision hoặc bytes authored. File downloads vẫn do Rust server phục vụ.

In-process FFI có thể xét sau nếu đo IPC đáng kể và chấp nhận blast radius;
không gộp “bỏ Node” với “bỏ process isolation” thành một điều kiện.

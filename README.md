<div align="center">
  <img src="icon.png" width="128" height="128" alt="Basie-64 Logo" />
  <h1>🎷 Basie-64</h1>
  <p><strong>A beautifully smooth, smart Base64 Encoder & Decoder built in Rust.</strong></p>
  
  <p>
    <a href="https://github.com/yourusername/basie-64/releases">
      <img src="https://img.shields.io/github/v/release/yourusername/basie-64" alt="Release" />
    </a>
    <a href="https://opensource.org/licenses/MIT">
      <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT" />
    </a>
  </p>
</div>

---

Basie-64 is an elegant, fast, and feature-rich Base64 encoder/decoder designed for developers who need more than just a text box. Built entirely in Rust using the `egui` framework, it offers a snappy native experience and smart detection tools to make handling Base64 effortless.

## ✨ Features

*   **⚡️ Smart Detection**: Paste a massive blob of text, logs, or JSON. Basie-64 will automatically find and extract hidden Base64 strings.
*   **📂 Drag & Drop File Support**: Drop any file directly into the app to instantly get its Base64 encoding.
*   **💾 Decode to File**: Safely decode Base64 back into its original file format (images, binaries, pdfs, etc.) with automatic file-type inference and native save dialogs.
*   **🖼️ Live Image Previews**: If the decoded Base64 contains an image, Basie-64 renders it live right inside the UI!
*   **🎨 Native & Beautiful**: A dark-themed, perfectly spaced modern UI that feels native on any OS.

## 🚀 Installation & Usage

You will need [Rust](https://www.rust-lang.org/tools/install) installed on your system to build Basie-64 from source.

```sh
# 1. Clone the repository
git clone https://github.com/yourusername/basie-64.git
cd basie-64

# 2. Run the application
cargo run --release
```

## 🛠️ Built With

*   [Rust](https://www.rust-lang.org/) - The core language
*   [eframe / egui](https://github.com/emilk/egui) - Instant graphical user interface
*   [base64](https://crates.io/crates/base64) - Encoding/decoding engine
*   [regex](https://crates.io/crates/regex) - Smart mixed-content matching
*   [rfd](https://crates.io/crates/rfd) - Native OS file dialogs
*   [infer](https://crates.io/crates/infer) - File type inference from magic bytes

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!
Feel free to check out the [issues page](https://github.com/yourusername/basie-64/issues). 

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

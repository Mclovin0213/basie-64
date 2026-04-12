use basie_64::core::{convert, decode, detect, diff, encode, hash};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::io::{self, Read};

#[derive(Parser)]
#[command(name = "basie", about = "Base64 toolkit — encode, decode, convert, and more")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode text or a file to Base64
    Encode {
        /// Text to encode (reads stdin if omitted)
        input: Option<String>,
        /// Read input from a file
        #[arg(long, short)]
        file: Option<String>,
    },
    /// Decode Base64 to text (supports JWT, URL-safe, no-padding)
    Decode {
        /// Base64 string to decode (reads stdin if omitted)
        input: Option<String>,
        /// Read input from a file
        #[arg(long, short)]
        file: Option<String>,
    },
    /// Convert between encoding formats (Base64, Hex, Base32, Base58, Percent)
    Convert {
        /// Encoded string to convert (reads stdin if omitted)
        input: Option<String>,
        /// Source format
        #[arg(long)]
        from: String,
        /// Target format
        #[arg(long)]
        to: String,
    },
    /// Detect the encoding format of input
    Detect {
        /// String to analyze (reads stdin if omitted)
        input: Option<String>,
    },
    /// Diff two Base64 strings
    Diff {
        /// First Base64 string
        a: String,
        /// Second Base64 string
        b: String,
    },
    /// Compute a hash of input
    Hash {
        /// Text to hash (reads stdin if omitted)
        input: Option<String>,
        /// Hash algorithm: sha256 or md5
        #[arg(long, default_value = "sha256")]
        algorithm: String,
        /// Read input from a file
        #[arg(long, short)]
        file: Option<String>,
    },
}

fn read_stdin() -> String {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap_or_default();
    buf.trim_end().to_string()
}

fn read_file_text(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Error reading file '{}': {}", path, e))
}

fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("Error reading file '{}': {}", path, e))
}

fn resolve_text_input(input: Option<String>, file: Option<String>) -> Result<String, String> {
    if let Some(path) = file {
        read_file_text(&path)
    } else {
        Ok(input.unwrap_or_else(read_stdin))
    }
}

fn parse_format(s: &str) -> convert::Format {
    match s.to_lowercase().as_str() {
        "base64" | "b64" => convert::Format::Base64,
        "hex" => convert::Format::Hex,
        "base32" | "b32" => convert::Format::Base32,
        "base58" | "b58" => convert::Format::Base58,
        "percent" | "url" => convert::Format::PercentEncoded,
        _ => {
            eprintln!(
                "Unknown format '{}'. Valid: base64, hex, base32, base58, percent",
                s
            );
            std::process::exit(1);
        }
    }
}

fn format_diff_output(result: &diff::DiffResult) -> String {
    let mut lines = Vec::new();
    for line in &result.lines {
        let prefix = match line.kind {
            diff::DiffKind::Added => "+",
            diff::DiffKind::Removed => "-",
            diff::DiffKind::Unchanged => " ",
        };
        let content = line
            .line_b
            .as_deref()
            .or(line.line_a.as_deref())
            .unwrap_or("");
        lines.push(format!("{} {}", prefix, content));
    }
    lines.push(format!(
        "\n{} addition(s), {} removal(s), {} unchanged",
        result.additions, result.removals, result.unchanged
    ));
    lines.join("\n")
}

fn run_diff_command(a: &str, b: &str) -> Result<String, String> {
    let bytes_a = convert::base64_to_bytes(a.trim())
        .map_err(|_| "Enter valid Base64 in both comparison fields.".to_string())?;
    let bytes_b = convert::base64_to_bytes(b.trim())
        .map_err(|_| "Enter valid Base64 in both comparison fields.".to_string())?;

    let result = match (std::str::from_utf8(&bytes_a), std::str::from_utf8(&bytes_b)) {
        (Ok(text_a), Ok(text_b)) => diff::diff_text(text_a, text_b),
        _ => diff::diff_binary(&bytes_a, &bytes_b),
    };

    Ok(format_diff_output(&result))
}

fn detect_output(text: &str) -> String {
    let b64_regex =
        Regex::new(r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?")
            .expect("static regex");
    let result = detect::detect(text, &b64_regex);

    if let Some(fmt) = result.detected_format {
        format!("Detected: {}", fmt)
    } else if result.is_base64 {
        "Detected: Base64".to_string()
    } else if !result.mixed_matches.is_empty() {
        let mut lines = vec![format!(
            "Found {} embedded Base64 string(s):",
            result.mixed_matches.len()
        )];
        for m in &result.mixed_matches {
            lines.push(format!("  {}", m));
        }
        lines.join("\n")
    } else {
        "No known encoding detected.".to_string()
    }
}

fn run(cli: Cli) -> Result<String, String> {
    match cli.command {
        Commands::Encode { input, file } => {
            if let Some(path) = file {
                let bytes = read_file_bytes(&path)?;
                Ok(encode::encode_base64_bytes(&bytes))
            } else {
                let text = input.unwrap_or_else(read_stdin);
                Ok(encode::encode_base64(&text))
            }
        }

        Commands::Decode { input, file } => {
            let text = resolve_text_input(input, file)?;
            match decode::decode_base64(&text) {
                Ok((output, _variant)) => Ok(match output {
                    decode::DecodeOutput::Jwt { formatted } => formatted,
                    decode::DecodeOutput::Text(s) => s,
                    decode::DecodeOutput::Binary { summary, .. } => summary,
                }),
                Err(e) => Err(match e.hint {
                    Some(hint) => format!("{}\nHint: {}", e.message, hint.message()),
                    None => e.message,
                }),
            }
        }

        Commands::Convert { input, from, to } => {
            let text = input.unwrap_or_else(read_stdin);
            let from_fmt = parse_format(&from);
            let to_fmt = parse_format(&to);
            match convert::convert(&text, from_fmt, to_fmt) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Conversion failed: {}", e)),
            }
        }

        Commands::Detect { input } => {
            let text = input.unwrap_or_else(read_stdin);
            Ok(detect_output(&text))
        }

        Commands::Diff { a, b } => run_diff_command(&a, &b),

        Commands::Hash {
            input,
            algorithm,
            file,
        } => {
            let data = if let Some(path) = file {
                std::fs::read(&path).unwrap_or_else(|e| {
                    eprintln!("Error reading file '{}': {}", path, e);
                    std::process::exit(1);
                })
            } else {
                let text = input.unwrap_or_else(read_stdin);
                text.into_bytes()
            };

            match algorithm.to_lowercase().as_str() {
                "sha256" => Ok(hash::sha256_hex(&data)),
                "md5" => Ok(hash::md5_hex(&data)),
                _ => Err(format!(
                    "Unknown algorithm '{}'. Valid: sha256, md5",
                    algorithm
                )),
            }
        }
    }
}

fn main() {
    match run(Cli::parse()) {
        Ok(output) => println!("{}", output),
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("basie-64-{unique}-{name}"))
    }

    fn write_temp_file(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("write temp file");
    }

    #[test]
    fn encode_file_accepts_binary_bytes() {
        let path = temp_path("binary.bin");
        write_temp_file(&path, &[0xff, 0x00, 0x41]);

        let output = run(Cli {
            command: Commands::Encode {
                input: None,
                file: Some(path.display().to_string()),
            },
        })
        .expect("encode file");

        assert_eq!(output, "/wBB");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn diff_decodes_base64_text_before_diffing() {
        let output = run_diff_command("SGVsbG8=", "V29ybGQ=").expect("diff output");

        assert!(output.contains("- Hello"));
        assert!(output.contains("+ World"));
        assert!(!output.contains("SGVsbG8="));
    }

    #[test]
    fn diff_rejects_invalid_base64() {
        let err = run_diff_command("not-base64", "V29ybGQ=").expect_err("invalid diff");
        assert_eq!(err, "Enter valid Base64 in both comparison fields.");
    }

    #[test]
    fn detect_uses_display_label() {
        assert_eq!(detect_output("Hello%20World"), "Detected: Percent-Encoded");
    }
}

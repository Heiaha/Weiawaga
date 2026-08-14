use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const SOURCE: &str = "https://nets.weiawaga.me";

fn manifest_path(path: &str) -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unset.");
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        PathBuf::from(manifest_dir).join(path)
    }
}

fn fetch(url: &str, dest: &Path) -> bool {
    let mut curl = Command::new("curl");
    curl.args(["-fsSL", "--connect-timeout", "10", "--retry", "2", "-o"]);

    let mut wget = Command::new("wget");
    wget.args(["-q", "-T", "10", "-t", "2", "-O"]);

    for mut command in [curl, wget] {
        let status = command.arg(dest).arg(url).status();
        if matches!(status, Ok(status) if status.success()) && dest.exists() {
            return true;
        }
        // Both tools can leave empty or partial files behind on failure.
        let _ = fs::remove_file(dest);
    }
    false
}

fn emit(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
    println!("cargo:rustc-env=NET_PATH={}", path.display());
}

fn main() {
    println!("cargo:rerun-if-changed=net.txt");
    println!("cargo:rerun-if-env-changed=EVALFILE");

    if let Ok(evalfile) = env::var("EVALFILE") {
        let path = manifest_path(&evalfile);
        assert!(path.exists(), "EVALFILE not found: {}", path.display());
        emit(&path);
        return;
    }

    let pin = fs::read_to_string(manifest_path("net.txt")).expect("net.txt is missing.");
    let mut fields = pin.split_whitespace();
    let name = fields.next().expect("net.txt must hold a filename.");
    let sha = fields.next().expect("net.txt must hold a sha256.");
    assert_eq!(
        name,
        format!("net-{}.bin", &sha[..12]),
        "net.txt filename must match its sha256."
    );

    let cache = manifest_path("nets").join(name);
    if !cache.exists() {
        fs::create_dir_all(manifest_path("nets")).expect("Unable to create nets dir.");
        println!("cargo:warning=Downloading {name} from {SOURCE}.");
        assert!(
            fetch(&format!("{SOURCE}/{name}"), &cache),
            "Unable to download {name} (is curl or wget installed?). \
             Either place it at nets/{name} by downloading {SOURCE}/{name} \
             yourself, or point EVALFILE at a network file."
        );
    }

    let bytes = fs::read(&cache).expect("Unable to read the cached network.");
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, sha, "Hash mismatch for {}.", cache.display());

    emit(&cache);
}

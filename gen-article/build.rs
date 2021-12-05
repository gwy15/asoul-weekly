#[cfg(target_os = "windows")]
extern crate winres;

/// 保存 build date 和 hash 到 BUILD_INFO 环境变量
fn save_build_info() {
    let date = chrono::Utc::now()
        .with_timezone(&chrono_tz::Asia::Shanghai)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let version = env!("CARGO_PKG_VERSION");
    let hash = git_version::git_version!(fallback = "unknown");

    let build_info = format!("v{}-{}-{}", version, hash, date);

    println!("cargo:rustc-env=BUILD_INFO={}", build_info);
}

fn main() {
    save_build_info();

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.compile().unwrap();
    }
}

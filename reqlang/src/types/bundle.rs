use std::{
    ffi::OsStr,
    fs::{self, File},
    io::Write,
};

/// This bin is responsible for bundling the generated type bindings.
/// It creates the file `./bindings/index.ts` with all type bindings exported.
/// These *.ts files are then transpiled with typescript.
///
/// The final `./dist/index.d.ts` file is used for `package.json` types.
fn main() {
    let _ = fs::create_dir_all("./bindings");

    if let Ok(items) = fs::read_dir("./bindings") {
        let exports: Vec<_> = items
            .filter_map(Result::ok)
            .filter_map(|p| {
                p.path()
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map(str::to_owned)
            })
            .filter(|f| f != "index")
            .map(|f| format!("export * from \"./{f}\""))
            .collect();

        let mut file = File::create("./bindings/index.ts").unwrap();
        file.write_all(exports.join("\n").as_bytes()).unwrap();
    };
}

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let assets_dir = Path::new("assets");

    // Cargo'ya assets klasöründeki değişiklikleri takip etmesini söylüyoruz.
    println!("cargo:rerun-if-changed=assets");

    // FBX'ten GLB'ye dönüştürücünün yolu
    let tool_path = Path::new("tools").join("fbx2gltf");

    if !tool_path.exists() {
        println!(
            "cargo:warning=FBX2glTF tool not found at {:?}. Skipping FBX to GLB conversion.",
            tool_path
        );
        return;
    }

    // assets dizini içindeki dosyaları tarıyoruz.
    if let Ok(entries) = fs::read_dir(assets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Eğer dosya bir .fbx ise
            if path.is_file() && path.extension().unwrap_or_default() == "fbx" {
                let file_stem = path.file_stem().unwrap().to_str().unwrap();
                let output_path = assets_dir.join(file_stem); // .glb uzantısını fbx2gltf otomatik ekleyecek
                let expected_glb = output_path.with_extension("glb");

                // Dönüştürmeye gerek var mı kontrol edelim (GLB yoksa veya FBX daha yeniyse)
                let needs_conversion = match fs::metadata(&expected_glb) {
                    Ok(glb_meta) => {
                        let fbx_meta = fs::metadata(&path).unwrap();
                        let glb_modified = glb_meta.modified().unwrap();
                        let fbx_modified = fbx_meta.modified().unwrap();
                        fbx_modified > glb_modified
                    }
                    Err(_) => true, // GLB dosyası yok, oluşturulmalı
                };

                if needs_conversion {
                    println!("cargo:warning=Converting FBX to GLB: {:?}", path);

                    let status = Command::new(&tool_path)
                        .arg("-b")
                        .arg("-i")
                        .arg(&path)
                        .arg("-o")
                        .arg(&output_path)
                        .status();

                    match status {
                        Ok(status) if status.success() => {
                            // Başarılı
                        }
                        Ok(status) => {
                            println!(
                                "cargo:warning=Failed to convert {:?}: exited with {}",
                                path, status
                            );
                        }
                        Err(err) => {
                            println!(
                                "cargo:warning=Failed to execute FBX2glTF on {:?}: {}",
                                path, err
                            );
                        }
                    }
                }
            }
        }
    }
}

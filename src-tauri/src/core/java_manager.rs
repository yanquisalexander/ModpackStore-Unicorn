use anyhow::{anyhow, Context, Result};
use dirs;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, create_dir_all, File};
use std::io::{self, copy, Cursor, Read, Write};
use std::path::PathBuf;
use tar::Archive;
use tauri_plugin_http::reqwest;
use zip::ZipArchive;

// Estructuras para deserializar la información de java
#[derive(Debug, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u8,
}

// Estructura principal del JavaManager
pub struct JavaManager {
    // Directorio base para las versiones de Java
    base_path: PathBuf,
}

impl JavaManager {
    /// Inicializa un nuevo JavaManager con el directorio base configurado
    pub fn new() -> Result<Self> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("No se pudo obtener el directorio de configuración"))?
            .join("dev.alexitoo.modpackstore")
            .join("_java_versions");

        // Crear el directorio si no existe
        if !config_path.exists() {
            create_dir_all(&config_path)
                .context("No se pudo crear el directorio para las versiones de Java")?;
        }

        Ok(JavaManager {
            base_path: config_path,
        })
    }

    /// Obtiene la ruta al ejecutable de Java para una versión específica
    /// Si la versión no está instalada, la descarga
    pub async fn get_java_path(&self, major_version: &str) -> Result<PathBuf> {
        let version_num = major_version
            .parse::<u8>()
            .context("La versión de Java no es un número válido")?;
        let version_dir = self.base_path.join(format!("java{}", major_version));

        // Comprobar si la versión ya está instalada
        if !self.is_java_installed(&version_dir) {
            // Si no está instalada, la descargamos
            self.download_java(version_num, &version_dir).await?;
        }

        Ok(self.get_java_directory(major_version))
    }

    /// Comprueba si Java está instalado en el directorio especificado
    fn is_java_installed(&self, version_dir: &PathBuf) -> bool {
        if !version_dir.exists() {
            return false;
        }

        // Verificar que el ejecutable de Java existe
        let java_exec = self.get_java_executable(version_dir);
        java_exec.is_ok() && java_exec.unwrap().exists()
    }

    fn get_java_directory(&self, version: &str) -> PathBuf {
        self.base_path.join(format!("java{}", version))
    }

    /// Obtiene la ruta al ejecutable de Java según el sistema operativo
    fn get_java_executable(&self, version_dir: &PathBuf) -> Result<PathBuf> {
        let bin_dir = version_dir.join("bin");

        #[cfg(target_os = "windows")]
        let java_exe = bin_dir.join("javaw.exe");

        #[cfg(not(target_os = "windows"))]
        let java_exe = bin_dir.join("java");

        if java_exe.exists() {
            Ok(java_exe)
        } else {
            Err(anyhow!(
                "El ejecutable de Java no existe en {}",
                bin_dir.display()
            ))
        }
    }

    /// Descarga e instala la versión de Java especificada
    async fn download_java(&self, version: u8, target_dir: &PathBuf) -> Result<()> {
        // Determinar la URL de descarga según la plataforma y arquitectura
        let download_url = self.get_download_url(version).await?;

        println!("Descargando Java {} desde {}", version, download_url);

        // Crear el directorio si no existe
        if !target_dir.exists() {
            create_dir_all(target_dir)
                .context("No se pudo crear el directorio para la versión de Java")?;
        }

        // Obtener la extensión del archivo desde la URL
        let extension = if download_url.ends_with(".zip") {
            "zip"
        } else if download_url.ends_with(".tar.gz") {
            "tar.gz"
        } else {
            return Err(anyhow!("Formato de archivo no soportado: {}", download_url));
        };

        // Crear el archivo temporal con la extensión adecuada
        let temp_file = target_dir.join(format!("java_temp_archive.{}", extension));

        // Crear un cliente con tiempo de espera personalizado
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutos
            .build()?;

        // Iniciar la descarga
        let response = client
            .get(&download_url)
            .send()
            .await
            .context("Error al iniciar la descarga de Java")?;

        if !response.status().is_success() {
            return Err(anyhow!("Error al descargar Java: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        println!("Tamaño total: {} bytes", total_size);

        // Preparar archivo para guardar
        let mut file = File::create(&temp_file).context("No se pudo crear el archivo temporal")?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        // Descargar el archivo mostrando progreso
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error al descargar fragmento")?;
            io::copy(&mut Cursor::new(&chunk), &mut file).context("Error al escribir fragmento")?;

            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = (downloaded as f64 / total_size as f64) * 100.0;
                println!(
                    "Descargado: {:.2}% ({}/{} bytes)",
                    progress, downloaded, total_size
                );
            } else {
                println!("Descargado: {} bytes", downloaded);
            }
        }

        println!("Descarga completada. Extrayendo...");

        // Extraer el archivo según su tipo
        self.extract_java_archive(&temp_file, target_dir)?;

        // Eliminar el archivo temporal
        fs::remove_file(&temp_file).context("No se pudo eliminar el archivo temporal")?;

        // Verificar que la instalación fue correcta
        if !self.is_java_installed(target_dir) {
            return Err(anyhow!("La instalación de Java {} falló", version));
        }

        println!("Java {} instalado correctamente", version);
        Ok(())
    }

    /// Determina la URL de descarga de OpenJDK según la plataforma, arquitectura y versión
    /// Usa la API de Adoptium para obtener la URL de descarga más reciente
    pub async fn get_download_url(&self, version: u8) -> Result<String> {
        #[derive(Debug, Deserialize)]
        struct Asset {
            binary: Binary,
        }

        #[derive(Debug, Deserialize)]
        struct Binary {
            package: Package,
        }

        #[derive(Debug, Deserialize)]
        struct Package {
            link: String,
        }

        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            return Err(anyhow!("Sistema operativo no soportado"));
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            return Err(anyhow!("Arquitectura no soportada"));
        };

        let api_url = format!(
            "https://api.adoptium.net/v3/assets/latest/{}/hotspot?os={}&architecture={}&image_type=jdk",
            version, os, arch
        );

        println!("Consultando API de Adoptium: {}", api_url);

        let response = reqwest::get(&api_url)
            .await
            .context("Error al consultar la API de Adoptium")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Error en la consulta a la API de Adoptium: {}",
                response.status()
            ));
        }

        let assets: Vec<Asset> = response
            .json()
            .await
            .context("Error al parsear la respuesta de la API")?;

        if let Some(asset) = assets.first() {
            Ok(asset.binary.package.link.clone())
        } else {
            let fallback_url = match os {
                "windows" => format!(
                    "https://github.com/adoptium/temurin{}-binaries/releases/download/jdk-{}.0.2%2B7/OpenJDK{}U-jdk_{}_windows_hotspot_{}.zip",
                    version, version, version, arch, version
                ),
                "mac" => format!(
                    "https://github.com/adoptium/temurin{}-binaries/releases/download/jdk-{}.0.2%2B7/OpenJDK{}U-jdk_{}_mac_hotspot_{}.tar.gz",
                    version, version, version, arch, version
                ),
                "linux" => format!(
                    "https://github.com/adoptium/temurin{}-binaries/releases/download/jdk-{}.0.2%2B7/OpenJDK{}U-jdk_{}_linux_hotspot_{}.tar.gz",
                    version, version, version, arch, version
                ),
                _ => return Err(anyhow!("Sistema operativo no soportado")),
            };

            println!(
                "No se encontraron binarios en la API, usando URL predeterminada: {}",
                fallback_url
            );

            Ok(fallback_url)
        }
    }

    /// Extrae el archivo de Java descargado usando bibliotecas nativas de Rust
    fn extract_java_archive(&self, archive_path: &PathBuf, target_dir: &PathBuf) -> Result<()> {
        let archive_str = archive_path.to_string_lossy().to_string();

        if archive_str.ends_with(".zip") {
            // En Windows, extraer ZIP usando la biblioteca zip-rs
            self.extract_zip(archive_path, target_dir)?;
        } else if archive_str.ends_with(".tar.gz") {
            // En macOS y Linux, extraer tar.gz usando las bibliotecas flate2 y tar
            self.extract_tar_gz(archive_path, target_dir)?;
        } else {
            return Err(anyhow!("Formato de archivo no soportado: {}", archive_str));
        }

        // Mover los archivos del subdirectorio al directorio principal
        self.fix_extracted_directory(target_dir)?;

        Ok(())
    }

    /// Extrae un archivo ZIP usando la biblioteca zip-rs
    fn extract_zip(&self, zip_path: &PathBuf, target_dir: &PathBuf) -> Result<()> {
        let file = File::open(zip_path).context("No se pudo abrir el archivo ZIP")?;
        let mut archive = ZipArchive::new(file).context("No se pudo leer el archivo ZIP")?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .context("No se pudo acceder al archivo en el ZIP")?;
            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .context("No se pudo crear directorio durante la extracción")?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .context("No se pudo crear directorio padre durante la extracción")?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .context("No se pudo crear archivo durante la extracción")?;
                io::copy(&mut file, &mut outfile)
                    .context("No se pudo copiar contenido del archivo ZIP")?;

                // Preservar permisos de ejecución en sistemas Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if file.unix_mode().unwrap_or(0) & 0o111 != 0 {
                        let mut perms = fs::metadata(&outpath)?.permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&outpath, perms)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Extrae un archivo tar.gz usando las bibliotecas flate2 y tar
    fn extract_tar_gz(&self, tar_gz_path: &PathBuf, target_dir: &PathBuf) -> Result<()> {
        let file = File::open(tar_gz_path).context("No se pudo abrir el archivo tar.gz")?;
        let gz_decoder = GzDecoder::new(file);
        let mut archive = Archive::new(gz_decoder);

        archive
            .unpack(target_dir)
            .context("No se pudo extraer el archivo tar.gz")?;

        // En sistemas Unix, restaurar permisos de ejecución
        #[cfg(unix)]
        {
            self.fix_permissions(target_dir)?;
        }

        Ok(())
    }

    /// Restaura permisos de ejecución para archivos en el directorio bin
    #[cfg(unix)]
    fn fix_permissions(&self, dir: &PathBuf) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let bin_dir = dir.join("bin");
        if bin_dir.exists() {
            for entry in fs::read_dir(bin_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    let mut perms = fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755); // rwxr-xr-x
                    fs::set_permissions(&path, perms)?;
                }
            }
        }

        Ok(())
    }

    /// Corrige la estructura de directorios después de la extracción
    /// ya que OpenJDK suele extraerse a un subdirectorio
    fn fix_extracted_directory(&self, target_dir: &PathBuf) -> Result<()> {
        // Buscar el subdirectorio creado durante la extracción
        let entries =
            fs::read_dir(target_dir).context("No se pudo leer el directorio de destino")?;

        let mut jdk_dir = None;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.contains("jdk") || name.contains("openjdk"))
                    .unwrap_or(false)
            {
                jdk_dir = Some(path);
                break;
            }
        }

        // Si encontramos un subdirectorio, mover todos sus contenidos al directorio principal
        if let Some(src_dir) = jdk_dir {
            let temp_dir = target_dir.join("temp_move");
            fs::rename(&src_dir, &temp_dir).context("No se pudo mover el directorio JDK")?;

            for entry in fs::read_dir(&temp_dir)? {
                let entry = entry?;
                let dest_path = target_dir.join(entry.file_name());
                fs::rename(entry.path(), dest_path)?;
            }

            fs::remove_dir_all(&temp_dir).context("No se pudo eliminar el directorio temporal")?;
        }

        Ok(())
    }

    pub fn is_version_installed(&self, version: &str) -> bool {
        let version_dir = self.base_path.join(format!("{}", version));
        version_dir.exists()
    }
}

// Ejemplo de uso:
/*
#[tokio::main]
async fn main() -> Result<()> {
    let java_manager = JavaManager::new()?;

    // Obtener la ruta de Java para la versión 17
    let java_path = java_manager.get_java_path("17").await?;
    println!("Java 17 instalado en: {}", java_path.display());

    Ok(())
}
*/

use axum::Router;
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::{Path, PathBuf};
use tower_http::services::ServeDir;

// Enumerate resources.
//   Input: Empty
//   Output: JSON: resource list
#[derive(Serialize, Deserialize, Debug)]
struct ResourceMetadata {
    name: String,
    hash: String,
    size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResourceEnumerateResponse {
    resources: Vec<ResourceMetadata>,
}

// Get resource.
//   Input: JSON: resource name
//   Output: JSON: resource data
#[derive(Serialize, Deserialize, Debug)]
struct GetResourceRequest {
    name: String,
}

#[derive(clap::Parser)]
struct Args {
    #[clap(long)]
    dist: PathBuf,
    #[clap(long)]
    port: u16,
}

fn file_hash(path: &Path) -> anyhow::Result<String> {
    use sha2;
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let hasher = sha2::Sha256::new();
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

fn enumerate_resources(dist: &Path) -> anyhow::Result<Vec<ResourceMetadata>> {
    // Find all files in dist
    let mut files = vec![];
    for entry in walkdir::WalkDir::new(dist) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    // Filter files by mask
    let mut filtered_files = vec![];
    for file in files {
        if file.extension().unwrap_or_default() == "dac" {
            filtered_files.push(file);
        }
    }

    // Convert files to the metadata
    let mut metadata = vec![];
    for file in filtered_files {
        metadata.push(ResourceMetadata {
            name: file.file_name().unwrap().to_string_lossy().to_string(),
            hash: file_hash(&file)?,
            size: std::fs::metadata(&file)?.len(),
        });
    }

    Ok(metadata)
}

fn read_resource(dist: &Path, name: &str) -> anyhow::Result<Vec<u8>> {
    let file = dist.join(name);
    let data = std::fs::read(&file)?;
    Ok(data)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let dist1 = args.dist.clone();
    let dist2 = args.dist.clone();
    let app = Router::new()
        .route(
            "/api/enumerate",
            axum::routing::get(|| async move {
                match enumerate_resources(&dist1) {
                    Ok(resources) => {
                        let response = ResourceEnumerateResponse { resources };
                        (axum::http::StatusCode::OK, axum::Json(response))
                    }
                    Err(e) => {
                        eprintln!("Error enumerating resources: {}", e);
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(ResourceEnumerateResponse { resources: vec![] }),
                        )
                    }
                }
            }),
        )
        .route("/api/get",
               axum::routing::get(|axum::extract::Query(params): axum::extract::Query<GetResourceRequest>| async move {
                   let data = read_resource(&dist2, &params.name);
                   axum::response::Response::builder()
                       .status(if data.is_ok() { axum::http::StatusCode::OK } else { axum::http::StatusCode::NOT_FOUND })
                       .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
                       .body(axum::body::Body::from(data.unwrap_or_else(|e| {
                           eprintln!("Error reading resource {}: {}", params.name, e);
                           vec![]
                       })))
                       .unwrap()
               }))
    .fallback_service(ServeDir::new(args.dist).append_index_html_on_directories(true));

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], args.port).into();
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

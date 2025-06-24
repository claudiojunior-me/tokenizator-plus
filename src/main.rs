mod file_tree;

use actix_files::Files;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, error};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures_util::StreamExt;
use serde_json::json;
use serde::{Deserialize, Serialize};
use tera::Tera;
use std::{env, path::PathBuf};

#[derive(Deserialize)]
struct PathRequest {
    path: String,
    ignore_patterns: Vec<String>,
}

#[derive(Serialize)]
struct PathResponse {
    content: String,
    token_count: usize,
}

async fn index(tera: web::Data<Tera>) -> impl Responder {
    let context = tera::Context::new();
    match tera.render("index.html", &context) {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(e) => {
            eprintln!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body("Error rendering template")
        }
    }
}

async fn process_path(req: web::Json<PathRequest>) -> Result<HttpResponse, error::Error> {
    // 1. Read the environment variable that defines the base directory.
    // 2. If it's not set, use "." so that `cargo run` behaves intuitively.
    let base_dir = env::var("DATA_DIR_BASE").unwrap_or_else(|_| ".".to_string());

    let mut path_to_process = PathBuf::new();
    path_to_process.push(base_dir);
    path_to_process.push(&req.path);

    // Resolve elements like ".." to avoid leaving the base directory (basic path traversal mitigation).
    // Libraries such as `path-clean` provide more robust handling.
    let Ok(canonical_path) = path_to_process.canonicalize() else {
        let user_error = format!("Error: path '{}' does not exist or is invalid.", req.path);
        return Err(error::ErrorBadRequest(user_error));
    };
    
    println!("======================================");
    println!("Processando caminho: {}", canonical_path.display());
    println!("Ignorando padrões: {:?}", req.ignore_patterns);

    match file_tree::generate_tree_and_content(&canonical_path, &req.ignore_patterns) {
        Ok(content) => {
            let token_count = file_tree::count_tokens(&content);
            Ok(HttpResponse::Ok().json(PathResponse { content, token_count }))
        },
        Err(e) => {
            eprintln!("Error processing path: {}", e);
            let user_error = format!(
                "Failed to process path '{}': {}. Check that it exists and that the container can read it.",
                req.path, e
            );
            // Use a client error (400) because the issue lies with user input.
            Err(error::ErrorBadRequest(user_error))
        }
    }
}

async fn process_path_stream(req: web::Json<PathRequest>) -> Result<HttpResponse, error::Error> {
    let base_dir = env::var("DATA_DIR_BASE").unwrap_or_else(|_| ".".to_string());

    let mut path_to_process = PathBuf::new();
    path_to_process.push(base_dir);
    path_to_process.push(&req.path);

    let Ok(canonical_path) = path_to_process.canonicalize() else {
        let user_error = format!("Error: path '{}' does not exist or is invalid.", req.path);
        return Err(error::ErrorBadRequest(user_error));
    };

    let (out_tx, out_rx) = mpsc::unbounded_channel::<Bytes>();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<file_tree::Progress>();

    // Forward progress updates as JSON lines
    let out_tx_clone = out_tx.clone();
    tokio::spawn(async move {
        while let Some(p) = progress_rx.recv().await {
            let pct = if p.total == 0 { 100.0 } else { (p.processed as f32 / p.total as f32) * 100.0 };
            let msg = json!({"progress": pct});
            let _ = out_tx_clone.send(Bytes::from(format!("{}\n", msg)));
        }
    });

    let out_tx_clone = out_tx.clone();
    let ignore = req.ignore_patterns.clone();
    tokio::task::spawn_blocking(move || {
        match file_tree::generate_tree_and_content_with_progress(&canonical_path, &ignore, &progress_tx) {
            Ok(content) => {
                let token_count = file_tree::count_tokens(&content);
                let msg = json!({"done": true, "content": content, "token_count": token_count});
                let _ = out_tx_clone.send(Bytes::from(format!("{}\n", msg)));
            }
            Err(e) => {
                let msg = json!({"error": e.to_string()});
                let _ = out_tx_clone.send(Bytes::from(format!("{}\n", msg)));
            }
        }
    });

    let stream = UnboundedReceiverStream::new(out_rx).map(Ok::<Bytes, error::Error>);

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .streaming(stream))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Servidor iniciado em http://localhost:3000");

    HttpServer::new(|| {
        let tera = Tera::new("src/templates/**/*").expect("Failed to parse templates.");

        App::new()
            .app_data(web::Data::new(tera))
            .route("/", web::get().to(index))
            .route("/api/process", web::post().to(process_path))
            .route("/api/process_stream", web::post().to(process_path_stream))
            .service(Files::new("/static", "static/"))
    })
    .bind("0.0.0.0:3000")? // Binding to 0.0.0.0 is required for Docker
    .run()
    .await
}
mod file_tree;

use actix_files::Files;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, error};
use actix_web::rt::{task, time::timeout};
use serde::{Deserialize, Serialize};
use tera::Tera;
use std::{env, path::PathBuf, time::Duration};

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

    let path = canonical_path.clone();
    let patterns = req.ignore_patterns.clone();

    let analysis = task::spawn_blocking(move || {
        file_tree::generate_tree_and_content(&path, &patterns)
    });

    let analysis_result = match timeout(Duration::from_secs(60), analysis).await {
        Ok(res) => res,
        Err(_) => {
            eprintln!("Analysis timed out");
            return Err(error::ErrorInternalServerError("Analysis timed out"));
        }
    };

    match analysis_result {
        Ok(Ok(content)) => {
            let token_count = file_tree::count_tokens(&content);
            Ok(HttpResponse::Ok().json(PathResponse { content, token_count }))
        },
        Ok(Err(e)) => {
            eprintln!("Error processing path: {}", e);
            let user_error = format!(
                "Failed to process path '{}': {}. Check that it exists and that the container can read it.",
                req.path, e
            );
            Err(error::ErrorBadRequest(user_error))
        }
        Err(e) => {
            eprintln!("Join error: {}", e);
            Err(error::ErrorInternalServerError("Analysis task failed"))
        }
    }
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
            .service(Files::new("/static", "static/"))
    })
    .bind("0.0.0.0:3000")? // Binding to 0.0.0.0 is required for Docker
    .run()
    .await
}
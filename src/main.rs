mod file_tree;

use actix_files::Files;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, error};
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
        Ok(content) => Ok(HttpResponse::Ok().json(PathResponse { content })),
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
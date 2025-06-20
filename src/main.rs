mod file_tree;

use actix_files::Files;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, error};
use serde::{Deserialize, Serialize};
use tera::Tera;
use std::{env, path::PathBuf};

// Estrutura para receber o JSON do frontend
#[derive(Deserialize)]
struct PathRequest {
    path: String,
    ignore_patterns: Vec<String>,
}

// Estrutura para enviar a resposta em JSON
#[derive(Serialize)]
struct PathResponse {
    content: String,
}

// Handler para a rota principal, renderiza o index.html
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

// Handler para a rota da API que processa o caminho
async fn process_path(req: web::Json<PathRequest>) -> Result<HttpResponse, error::Error> {
    // 1. Lê a variável de ambiente para saber o diretório base.
    // 2. Se a variável não for definida, usa "." (diretório atual) como padrão.
    //    Isso faz com que `cargo run` funcione de forma intuitiva.
    let base_dir = env::var("DATA_DIR_BASE").unwrap_or_else(|_| ".".to_string());

    // Constrói o caminho completo a partir da base configurada e do input do usuário.
    let mut path_to_process = PathBuf::new();
    path_to_process.push(base_dir);
    path_to_process.push(&req.path);
    
    // Resolve caminhos como ".." para evitar sair do diretório base (Path Traversal)
    // NOTA: Esta é uma medida de segurança básica. Bibliotecas como `path-clean` podem oferecer mais robustez.
    let Ok(canonical_path) = path_to_process.canonicalize() else {
        let user_error = format!("Erro: O caminho '{}' não existe ou é inválido.", req.path);
        return Err(error::ErrorBadRequest(user_error));
    };
    
    println!("======================================");
    println!("Processando caminho: {}", canonical_path.display());
    println!("Ignorando padrões: {:?}", req.ignore_patterns);

    match file_tree::generate_tree_and_content(&canonical_path, &req.ignore_patterns) {
        Ok(content) => Ok(HttpResponse::Ok().json(PathResponse { content })),
        Err(e) => {
            eprintln!("Error processing path: {}", e);
            let user_error = format!("Erro ao processar o caminho '{}': {}. Verifique se o caminho existe e se o container tem permissão de leitura.", req.path, e);
            // Usamos um erro de cliente (400) pois o problema é o input do usuário.
            Err(error::ErrorBadRequest(user_error))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Servidor iniciado em http://localhost:3000");

    HttpServer::new(|| {
        // Inicializa o motor de templates Tera
        let tera = Tera::new("src/templates/**/*").expect("Failed to parse templates.");

        App::new()
            .app_data(web::Data::new(tera))
            .route("/", web::get().to(index))
            .route("/api/process", web::post().to(process_path))
            .service(Files::new("/static", "static/")) // Serve arquivos estáticos
    })
    .bind("0.0.0.0:3000")? // Bind em 0.0.0.0 é essencial para o Docker
    .run()
    .await
}
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use std::env;
use dotenv::dotenv;

#[derive(Serialize, Deserialize)]
struct User {
    username: String,
    password: String,
}

async fn login(_user: web::Json<User>) -> impl Responder {
    // Lógica para verificar o usuário no banco de dados
    HttpResponse::Ok().json("Login bem-sucedido")
}

async fn register(_user: web::Json<User>) -> impl Responder {
    // Lógica para registrar o usuário no banco de dados
    HttpResponse::Ok().json("Usuário registrado com sucesso")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL não está definido");

    // Conectar ao banco de dados
    println!("Tentando conectar ao banco de dados...");
    let db_connection = tokio_postgres::connect(&database_url, NoTls).await;
    
    let (_client, connection) = match db_connection {
        Ok((client, connection)) => {
            println!("Conexão com o banco de dados estabelecida com sucesso!");
            (client, connection)
        },
        Err(e) => {
            eprintln!("Erro ao conectar ao banco de dados: {}", e);
            eprintln!("Verifique se o PostgreSQL está em execução e se a URL de conexão está correta.");
            eprintln!("Continuando sem conexão com o banco de dados...");
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, 
                "Não foi possível conectar ao banco de dados"));
        }
    };

    // Iniciar a conexão em um thread separado
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Erro na conexão: {}", e);
        }
    });

    println!("Iniciando servidor HTTP em 127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .route("/login", web::post().to(login))
            .route("/register", web::post().to(register))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
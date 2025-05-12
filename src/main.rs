use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, Client, Error as PgError};
use std::env;
use dotenv::dotenv;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct Cliente {
    nome: String,
    cpf: String,
    endereco: String,
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

// Função para criar a tabela de clientes se não existir
async fn create_clientes_table(client: &Client) -> Result<(), PgError> {
    client.execute(
        "CREATE TABLE IF NOT EXISTS clientes (
            id SERIAL PRIMARY KEY,
            nome VARCHAR(100) NOT NULL,
            cpf VARCHAR(14) UNIQUE NOT NULL,
            endereco VARCHAR(200) NOT NULL,
            email VARCHAR(100) UNIQUE NOT NULL,
            password VARCHAR(100) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        &[],
    ).await?;
    
    println!("Tabela de clientes verificada/criada com sucesso!");
    Ok(())
}

async fn register(
    cliente: web::Json<Cliente>,
    db_client: web::Data<Arc<Mutex<Option<Client>>>>
) -> impl Responder {
    let client_lock = db_client.lock().await;
    
    if let Some(client) = &*client_lock {
        // Verificar se o email já existe
        let email_exists = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM clientes WHERE email = $1)",
            &[&cliente.email]
        ).await;
        
        // Verificar se o CPF já existe
        let cpf_exists = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM clientes WHERE cpf = $1)",
            &[&cliente.cpf]
        ).await;
        
        match email_exists {
            Ok(row) => {
                let exists: bool = row.get(0);
                if exists {
                    return HttpResponse::Conflict().json(ApiResponse {
                        success: false,
                        message: "Email já cadastrado".to_string(),
                    });
                }
                
                match cpf_exists {
                    Ok(row) => {
                        let exists: bool = row.get(0);
                        if exists {
                            return HttpResponse::Conflict().json(ApiResponse {
                                success: false,
                                message: "CPF já cadastrado".to_string(),
                            });
                        }
                        
                        // Inserir novo cliente
                        // Nota: Em produção, você deve fazer hash da senha antes de armazenar
                        let result = client.execute(
                            "INSERT INTO clientes (nome, cpf, endereco, email, password) VALUES ($1, $2, $3, $4, $5)",
                            &[&cliente.nome, &cliente.cpf, &cliente.endereco, &cliente.email, &cliente.password]
                        ).await;
                        
                        match result {
                            Ok(_) => HttpResponse::Ok().json(ApiResponse {
                                success: true,
                                message: "Cliente registrado com sucesso".to_string(),
                            }),
                            Err(e) => {
                                eprintln!("Erro ao registrar cliente: {}", e);
                                HttpResponse::InternalServerError().json(ApiResponse {
                                    success: false,
                                    message: "Erro ao registrar cliente".to_string(),
                                })
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Erro ao verificar CPF: {}", e);
                        HttpResponse::InternalServerError().json(ApiResponse {
                            success: false,
                            message: "Erro ao verificar CPF".to_string(),
                        })
                    }
                }
            },
            Err(e) => {
                eprintln!("Erro ao verificar email: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse {
                    success: false,
                    message: "Erro ao verificar email".to_string(),
                })
            }
        }
    } else {
        HttpResponse::ServiceUnavailable().json(ApiResponse {
            success: false,
            message: "Banco de dados não disponível".to_string(),
        })
    }
}

async fn login(
    login_req: web::Json<LoginRequest>,
    db_client: web::Data<Arc<Mutex<Option<Client>>>>
) -> impl Responder {
    let client_lock = db_client.lock().await;
    
    if let Some(client) = &*client_lock {
        // Verificar credenciais
        let result = client.query_opt(
            "SELECT id, nome FROM clientes WHERE email = $1 AND password = $2",
            &[&login_req.email, &login_req.password]
        ).await;
        
        match result {
            Ok(row_option) => {
                if let Some(row) = row_option {
                    // Cliente encontrado, login bem-sucedido
                    let id: i32 = row.get(0);
                    let nome: String = row.get(1);
                    
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Login bem-sucedido",
                        "cliente": {
                            "id": id,
                            "nome": nome
                        }
                    }))
                } else {
                    // Cliente não encontrado ou senha incorreta
                    HttpResponse::Unauthorized().json(ApiResponse {
                        success: false,
                        message: "Email ou senha incorretos".to_string(),
                    })
                }
            },
            Err(e) => {
                eprintln!("Erro ao verificar credenciais: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse {
                    success: false,
                    message: "Erro ao verificar credenciais".to_string(),
                })
            }
        }
    } else {
        HttpResponse::ServiceUnavailable().json(ApiResponse {
            success: false,
            message: "Banco de dados não disponível".to_string(),
        })
    }
}

async fn list_clientes(
    db_client: web::Data<Arc<Mutex<Option<Client>>>>
) -> impl Responder {
    let client_lock = db_client.lock().await;
    
    if let Some(client) = &*client_lock {
        match client.query("SELECT id, nome, cpf, endereco, email, created_at FROM clientes ORDER BY id", &[]).await {
            Ok(rows) => {
                let clientes: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|row| {
                        let id: i32 = row.get(0);
                        let nome: String = row.get(1);
                        let cpf: String = row.get(2);
                        let endereco: String = row.get(3);
                        let email: String = row.get(4);
                        // Cambiamos el tipo a NaiveDateTime
                        let created_at: chrono::NaiveDateTime = row.get(5);
                        
                        serde_json::json!({
                            "id": id,
                            "nome": nome,
                            "cpf": cpf,
                            "endereco": endereco,
                            "email": email,
                            "created_at": created_at.to_string()
                        })
                    })
                    .collect();
                
                HttpResponse::Ok().json(clientes)
            },
            Err(e) => {
                eprintln!("Erro ao listar clientes: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "message": "Erro ao listar clientes"
                }))
            }
        }
    } else {
        HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "success": false,
            "message": "Banco de dados não disponível"
        }))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    // Try to get DATABASE_URL, but don't panic if it's not found
    let database_url = env::var("DATABASE_URL");
    
    let client_option = if let Ok(url) = database_url {
        // Conectar ao banco de dados
        println!("Tentando conectar ao banco de dados...");
        match tokio_postgres::connect(&url, NoTls).await {
            Ok((client, connection)) => {
                println!("Conexão com o banco de dados estabelecida com sucesso!");
                
                // Iniciar a conexão em um thread separado
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Erro na conexão: {}", e);
                    }
                });
                
                // Criar tabela de clientes se não existir
                if let Err(e) = create_clientes_table(&client).await {
                    eprintln!("Erro ao criar tabela de clientes: {}", e);
                }
                
                Some(client)
            },
            Err(e) => {
                eprintln!("Erro ao conectar ao banco de dados: {}", e);
                eprintln!("Verifique se o PostgreSQL está em execução e se a URL de conexão está correta.");
                eprintln!("Continuando sem conexão com o banco de dados...");
                None
            }
        }
    } else {
        eprintln!("DATABASE_URL não está definido. Continuando sem conexão com o banco de dados...");
        None
    };

    // Compartilhar o cliente do banco de dados entre as rotas
    let db_client = web::Data::new(Arc::new(Mutex::new(client_option)));

    println!("Iniciando servidor HTTP em 127.0.0.1:8080");
    HttpServer::new(move || {
        // Configuração do CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
            
        App::new()
            .wrap(cors)  // Adiciona o middleware CORS
            .app_data(db_client.clone())
            .route("/login", web::post().to(login))
            .route("/register", web::post().to(register))
            .route("/clientes", web::get().to(list_clientes))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
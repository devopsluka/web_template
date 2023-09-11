use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use chrono::prelude::*;

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Service {
    id: u64,
    name: String,
    price: f32,
    duration: u32,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Serialize, Deserialize)]
struct Database {
    services: HashMap<u64, Service>,
    users: HashMap<u64, User>,
}

impl Database {
    fn new() -> Self {
        Self {
            services: HashMap::new(),
            users: HashMap::new(),
        }
    }

    fn insert(&mut self, service: Service) {
        self.services.insert(service.id, service);
    }

    fn get(&self, id: &u64) -> Option<&Service> {
        self.services.get(id)
    }

    fn get_all(&self) -> Vec<&Service> {
        self.services.values().collect()
    }

    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_user_by_name(&self, username: &str) -> Option<&User> {
        self.users.values().find(|u| u.username == username)
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        let file_content = fs::read_to_string("database.json")?;
        let db: Database = serde_json::from_str(&file_content)?;
        Ok(db)
    }
}

struct AppState {
    db: Mutex<Database>,
}

async fn create_service(app_state: web::Data<AppState>, service: web::Json<Service>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database in create service fn");
    db.insert(service.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_service(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let db = app_state
        .db
        .lock()
        .expect("Failed to lock database in reading service");
    match db.get(&id.into_inner()) {
        Some(service) => HttpResponse::Ok().json(service),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_all_services(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state
        .db
        .lock()
        .expect("Failed to lock database in reading services");
    let services = db.get_all();
    HttpResponse::Ok().json(services)
}

async fn home_page() -> actix_web::Result<HttpResponse>{
    Ok(HttpResponse::Ok().body("Hello World!"))
}

async fn register(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database when registering an user");

    let hashed_password = hash(&user.password, DEFAULT_COST).expect("Failed to hash password");
    let new_user = User {
        id: user.id,
        username: user.username.clone(),
        password: hashed_password,
    };

    db.insert_user(new_user);
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn login(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let db = app_state
        .db
        .lock()
        .expect("Failed to lock database when logging in");
    match db.get_user_by_name(&user.username) {
        Some(stored_user) => {
            if verify(&user.password, &stored_user.password).expect("Failed to verify password") {
                HttpResponse::Ok().body("Login successful")
            } else {
                HttpResponse::BadRequest().body("Invalid username and/or password")
            }
        }
        None => HttpResponse::Unauthorized().body("Invalid username or password"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db: Database = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new(),
    };

    let data = web::Data::new(AppState { db: Mutex::new(db) });

    println!("Server running at port 8080");

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin
                            .as_bytes()
                            .starts_with("http://localhost".as_bytes()) || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .app_data(data.clone())
            .route("/", web::get().to(home_page))
            .route("/service", web::post().to(create_service))
            .route("/service", web::get().to(read_all_services))
            .route("/service/{id}", web::get().to(read_service))
            .route("register", web::post().to(register))
            .route("login", web::post().to(login))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
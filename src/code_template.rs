use actix_cors::Cors;

use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};

use serde::{Deserialize, Serialize};

use reqwest::Client as HttpClient;

use async_trait::async_trait;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::Mutex;

#[derive(Serialize, Debug, Deserialize, Clone)]
struct Task {
    id: u64,
    name: String,
    completed: bool,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct Database {
    tasks: HashMap<u64, Task>,
    users: HashMap<u64, User>,
}

impl Database {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            users: HashMap::new(),
        }
    }

    // CRUD DATA

    fn insert(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    fn get(&self, id: &u64) -> Option<&Task> {
        self.tasks.get(id)
    }

    fn get_all(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    fn delete(&mut self, id: &u64) {
        self.tasks.remove(id);
    }

    fn update(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    // USER CRUD

    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_user_by_name(&self, username: &str) -> Option<&User> {
        self.users.values().find(|u| u.username == username)
    }

    // SAVE DATABASE

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


// CREATE
async fn create_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database in create task fn");
    db.insert(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}


// READ
async fn read_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let db = app_state
        .db
        .lock()
        .expect("Failed to lock database in reading task");
    match db.get(&id.into_inner()) {
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_all_tasks(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state
        .db
        .lock()
        .expect("Failed to lock database in reading tasks");
    let tasks = db.get_all();
    HttpResponse::Ok().json(tasks)
}

// UPDATE
async fn update_task(app_state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database in updating a task");
    db.update(task.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

// DELETE
async fn delete_task(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database in deleting a task");
    db.delete(&id.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn home_page() -> actix_web::Result<HttpResponse>{
    Ok(HttpResponse::Ok().body("Hello World!"))
}

async fn register(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db = app_state
        .db
        .lock()
        .expect("Failed to lock database when registering an user");
    db.insert_user(user.into_inner());
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
            if stored_user.password == user.password {
                HttpResponse::Ok().body("Login successful!")
            } else {
                HttpResponse::BadRequest().body("Invalid username or password")
            }
        }
        None => HttpResponse::Unauthorized().finish(),
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
            .route("/task", web::post().to(create_task))
            .route("/task", web::get().to(read_all_tasks))
            .route("/task/{id}", web::get().to(read_task))
            .route("/task", web::put().to(update_task))
            .route("/task/{id}", web::delete().to(delete_task))
            .route("register", web::post().to(register))
            .route("login", web::post().to(login))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

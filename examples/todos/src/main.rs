use once_cell::sync::Lazy;

use salvo::prelude::*;

use self::models::*;

static STORE: Lazy<Db> = Lazy::new(|| new_store());

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    start_server().await;
}

pub(crate) async fn start_server() {
    let router = Router::with_path("todos")
        .get(list_todos)
        .post(create_todo)
        .push(Router::with_path("<id>").put(update_todo).delete(delete_todo));
    tracing::info!("Listening on http://0.0.0.0:7878");
    Server::new(TcpListener::bind("0.0.0.0:7878")).serve(router).await;
}

#[fn_handler]
pub async fn list_todos(req: &mut Request, res: &mut Response) {
    let opts = req.read::<ListOptions>().await.unwrap();
    let todos = STORE.lock().await;
    let todos: Vec<Todo> = todos
        .clone()
        .into_iter()
        .skip(opts.offset.unwrap_or(0))
        .take(opts.limit.unwrap_or(std::usize::MAX))
        .collect();
    res.render(Json(todos));
}

#[fn_handler]
pub async fn create_todo(req: &mut Request, res: &mut Response) {
    let new_todo = req.read::<Todo>().await.unwrap();
    tracing::debug!(todo = ?new_todo, "create todo");

    let mut vec = STORE.lock().await;

    for todo in vec.iter() {
        if todo.id == new_todo.id {
            tracing::debug!(id = ?new_todo.id, "todo already exists");
            res.set_status_code(StatusCode::BAD_REQUEST);
            return;
        }
    }

    vec.push(new_todo);
    res.set_status_code(StatusCode::CREATED);
}

#[fn_handler]
pub async fn update_todo(req: &mut Request, res: &mut Response) {
    let id = req.get_param::<u64>("id").unwrap();
    let updated_todo = req.read::<Todo>().await.unwrap();
    tracing::debug!(todo = ?updated_todo, id = ?id, "update todo");
    let mut vec = STORE.lock().await;

    for todo in vec.iter_mut() {
        if todo.id == id {
            *todo = updated_todo;
            res.set_status_code(StatusCode::OK);
            return;
        }
    }

    tracing::debug!(id = ?id, "todo is not found");
    res.set_status_code(StatusCode::NOT_FOUND);
}

#[fn_handler]
pub async fn delete_todo(req: &mut Request, res: &mut Response) {
    let id = req.get_param::<u64>("id").unwrap();
    tracing::debug!(id = ?id, "delete todo");

    let mut vec = STORE.lock().await;

    let len = vec.len();
    vec.retain(|todo| todo.id != id);

    let deleted = vec.len() != len;
    if deleted {
        res.set_status_code(StatusCode::NO_CONTENT);
    } else {
        tracing::debug!(id = ?id, "todo is not found");
        res.set_status_code(StatusCode::NOT_FOUND);
    }
}

mod models {
    use serde::{Deserialize, Serialize};
    use tokio::sync::Mutex;

    pub type Db = Mutex<Vec<Todo>>;

    pub fn new_store() -> Db {
        Mutex::new(Vec::new())
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Todo {
        pub id: u64,
        pub text: String,
        pub completed: bool,
    }

    #[derive(Deserialize, Debug)]
    pub struct ListOptions {
        pub offset: Option<usize>,
        pub limit: Option<usize>,
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use salvo::http::StatusCode;

    use super::models::Todo;

    #[tokio::test]
    async fn test_todo_create() {
        tokio::task::spawn(async {
            super::start_server().await;
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let client = Client::new();
        let resp = client
            .post("http://0.0.0.0:7878/todos")
            .json(&test_todo())
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let resp = client
            .post("http://0.0.0.0:7878/todos")
            .json(&test_todo())
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    fn test_todo() -> Todo {
        Todo {
            id: 1,
            text: "test todo".into(),
            completed: false,
        }
    }
}
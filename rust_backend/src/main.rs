use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{Json, Router, extract::{Path, Query, State}, http::StatusCode, routing::get};

trait PagedVec<T> {
    fn paged(self, pagination: Pagination) -> Vec<T>;
}

#[derive(Debug)]
enum AppError {
    BlogExist,
    BlogDoesNotExist,
}

type AppResult<T> = Result<T, AppError>;

type JsonReturn<T> = (StatusCode, Json<T>);

#[derive(Debug, Default, Clone)]
struct AppState {
    post_repo: Arc<Mutex<BlogPostRepository>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct BlogPost {
    id: usize,
    title: String,
    post: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct BlogPostTransfer {
    id: Option<usize>,
    title: String,
    post: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize)]
struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

trait Repository<T, ID> {
    fn find_all(&self) -> Vec<T>;
    fn find_by_id(&self, id: ID) -> Option<T>;
    fn create(&mut self, t: T) -> AppResult<()>;
    fn delete_by_id(&mut self, id: ID) -> AppResult<()>;
    fn update(&mut self, t: T) -> AppResult<()>;
}

#[derive(Debug, Default, Clone)]
struct BlogPostRepository {
    posts: HashMap<usize, BlogPost>,
}

impl<T> PagedVec<T> for Vec<T> {
    fn paged(self, pagination: Pagination) -> Vec<T> {
        let page: usize = pagination.page.unwrap_or(0);
        match pagination.per_page {
            Some(per_page) => self
                .into_iter()
                .skip(page * per_page)
                .take(per_page)
                .collect(),
            None => self.into_iter().skip(page).collect(),
        }
    }
}

impl Repository<BlogPost, usize> for BlogPostRepository {
    fn find_all(&self) -> Vec<BlogPost> {
        self.posts
            .clone()
            .into_iter()
            .map(|(_, blog)| blog)
            .collect::<Vec<BlogPost>>()
    }

    fn find_by_id(&self, id: usize) -> Option<BlogPost> {
        self.posts.get(&id).cloned()
    }

    fn create(&mut self, t: BlogPost) -> AppResult<()> {
        if self.posts.contains_key(&t.id) {
            return Err(AppError::BlogExist);
        }
        self.posts.insert(t.id, t);
        Ok(())
    }

    fn delete_by_id(&mut self, id: usize) -> AppResult<()> {
        if self.posts.remove(&id).is_none() {
            return Err(AppError::BlogDoesNotExist);
        }
        Ok(())
    }

    fn update(&mut self, t: BlogPost) -> AppResult<()> {
        if !self.posts.contains_key(&t.id) {
            return Err(AppError::BlogDoesNotExist);
        }
        self.posts.insert(t.id, t);
        Ok(())
    }
}

async fn get_all_posts(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> JsonReturn<Vec<BlogPost>> {
    let result: Vec<BlogPost> = state.post_repo.lock().unwrap().find_all().paged(pagination);
    (StatusCode::OK, Json(result))
}

async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> JsonReturn<Option<BlogPost>> {
    let result = state.post_repo.lock().unwrap().find_by_id(id);
    (
        if result.is_none() {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::OK
        },
        Json(result),
    )
}

async fn create_post(State(state): State<AppState>, Json(post): Json<BlogPostTransfer>) -> StatusCode {
    let largest_id_post = state.post_repo.lock().unwrap().find_all().into_iter().max_by_key(|item| item.id);
    let selected_id = if let Some(id) = post.id {
        id
    }else if let Some(post) = largest_id_post {
        post.id + 1
    }else {
        return StatusCode::BAD_REQUEST;
    };
    let post = BlogPost { id: selected_id, title: post.title, post: post.post };

    println!("selected: {}", selected_id);

    if state.post_repo.lock().unwrap().create(post).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

async fn update_post(State(state): State<AppState>, Json(post): Json<BlogPost>) -> StatusCode {
    if state.post_repo.lock().unwrap().update(post).is_err() {
        return StatusCode::NOT_FOUND;
    }
    return StatusCode::OK;
}

async fn delete_post(State(state): State<AppState>, Path(id): Path<usize>) -> StatusCode {
    if state.post_repo.lock().unwrap().delete_by_id(id).is_err() {
        return StatusCode::NOT_FOUND;
    }
    return StatusCode::OK;
}

#[tokio::main]
async fn main() {
    let state = AppState::default();
    let mut repo = state.post_repo.lock().unwrap();
    repo.create(BlogPost { id: 1, title: "Lorem Ipsum".to_string(), post: "dolor sit amet, consectetur adipiscing elit. Nullam malesuada sem quis dictum elementum. Praesent rutrum elementum vulputate. In at consequat sapien. Sed a fringilla elit, vel interdum justo. Ut aliquam, lacus id maximus lobortis, nibh tellus semper mi, sed egestas nisl mi eu felis. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. In vitae turpis elit. Nulla commodo rutrum elit non lacinia. Nullam sodales dictum enim. Curabitur blandit, sem sit amet ultrices dignissim, mauris elit scelerisque odio, a rutrum tortor tellus vel nisi.".to_string() }).unwrap();
    repo.create(BlogPost { id: 2, title: "Vivamus ut rhoncus sapien".to_string(), post: "Vivamus ut rhoncus sapien, et rhoncus purus. Nam aliquam nibh at finibus dictum. Duis a gravida sapien. Phasellus maximus auctor tellus, at ultricies eros ornare blandit. In pellentesque orci fermentum, ultricies eros id, convallis justo. Quisque sem ipsum, volutpat quis aliquam eu, pulvinar nec urna. In lorem elit, gravida at porttitor in, accumsan et enim. Nullam rhoncus sapien vel posuere mattis. Donec et tincidunt enim, sit amet molestie justo. ".to_string() }).unwrap();
    repo.create(BlogPost { id: 3, title: "Nunc id tortor non eros hendrerit bibendum sed vel neque".to_string(), post: "In eget lobortis diam. Quisque sed nisl mi. Aenean non lectus a nisi tempor pellentesque. Curabitur lacus purus, imperdiet et nunc at, tempor dictum eros. Fusce non leo ullamcorper, interdum libero eget, maximus nisi. Maecenas sed purus non orci ornare semper. In eget risus ac quam elementum scelerisque. Pellentesque id eleifend lorem. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam eu magna vel mi mattis interdum sit amet sed erat. Sed in dapibus sem. Donec sed neque venenatis lorem tincidunt mollis. Fusce tempor fermentum ipsum ut accumsan. ".to_string() }).unwrap();
    repo.create(BlogPost { id: 4, title: "Pellentesque dui ante".to_string(), post: "dictum sed eleifend sit amet, tincidunt mollis ex. Nulla a facilisis nulla. Morbi at libero ligula. Sed varius cursus enim, sit amet commodo urna mollis eu. Nunc ac consectetur velit. Proin eu feugiat quam, pharetra ultricies tellus. Pellentesque consequat non quam a malesuada. Proin convallis sollicitudin dui sit amet vehicula. Mauris metus erat, aliquet ullamcorper sapien at, interdum gravida lacus. Fusce lectus risus, suscipit in sagittis at, viverra eu nunc. In ac augue eu nunc convallis tincidunt. Etiam ut sollicitudin nunc. Pellentesque suscipit porttitor odio, in mollis massa vehicula vel. Praesent vitae pellentesque metus. ".to_string() }).unwrap();
    repo.create(BlogPost { id: 5, title: "Ut pulvinar posuere nunc".to_string(), post: "nec pharetra dui pulvinar a. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Sed in scelerisque nulla, vitae scelerisque arcu. Cras in neque lacinia, dapibus ante non, egestas nulla. Cras dictum tortor nec nibh volutpat laoreet. Nullam interdum rhoncus nisi at tempus. Donec eleifend velit non sapien dapibus euismod vel et mauris. Interdum et malesuada fames ac ante ipsum primis in faucibus. Suspendisse potenti. Maecenas lectus lectus, finibus eget tristique non, elementum et tellus. ".to_string() }).unwrap();
    drop(repo);

    let post_routes = Router::new()
        .route("/", get(get_all_posts).post(create_post).patch(update_post))
        .route("/{id}", get(get_post).delete(delete_post));

    let app = Router::new()
        .nest("/api/v1/posts", post_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

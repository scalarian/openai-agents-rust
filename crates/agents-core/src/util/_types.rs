use futures::future::BoxFuture;

pub enum MaybeAwaitable<T> {
    Ready(T),
    Future(BoxFuture<'static, T>),
}

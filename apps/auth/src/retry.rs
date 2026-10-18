use redis::RedisResult;

pub async fn retry_once_if_disconnected<C, T, F, Fut>(conn: &C, call: F) -> RedisResult<T>
where
    C: Clone,
    F: Fn(C) -> Fut,
    Fut: Future<Output = RedisResult<T>>,
{
    match call(conn.clone()).await {
        Err(err) if err.is_connection_dropped() => call(conn.clone()).await,
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pretty_assertions::assert_eq;
    use redis::{ErrorKind, RedisError};

    use super::*;

    fn dropped() -> RedisError {
        RedisError::from((ErrorKind::Io, "broken pipe"))
    }

    fn refused() -> RedisError {
        RedisError::from((ErrorKind::Client, "bad command"))
    }

    async fn call_with(results: Vec<RedisResult<i32>>) -> (RedisResult<i32>, usize) {
        let calls = AtomicUsize::new(0);
        let results = std::sync::Mutex::new(results.into_iter());
        let result = retry_once_if_disconnected(&(), |()| {
            calls.fetch_add(1, Ordering::SeqCst);
            let next = results.lock().unwrap().next().unwrap();
            async move { next }
        })
        .await;
        (result, calls.into_inner())
    }

    #[tokio::test]
    async fn first_call_works_returns_its_result_without_retrying() {
        // Act
        let (result, calls) = call_with(vec![Ok(7)]).await;

        // Assert
        assert_eq!(result.unwrap(), 7);
        assert_eq!(calls, 1);
    }

    #[tokio::test]
    async fn dropped_connection_retries_once_and_returns_the_second_result() {
        // Act
        let (result, calls) = call_with(vec![Err(dropped()), Ok(7)]).await;

        // Assert
        assert_eq!(result.unwrap(), 7);
        assert_eq!(calls, 2);
    }

    #[tokio::test]
    async fn dropped_connection_twice_returns_the_error() {
        // Act
        let (result, calls) = call_with(vec![Err(dropped()), Err(dropped())]).await;

        // Assert
        assert!(result.unwrap_err().is_connection_dropped());
        assert_eq!(calls, 2);
    }

    #[tokio::test]
    async fn other_error_returns_it_without_retrying() {
        // Act
        let (result, calls) = call_with(vec![Err(refused())]).await;

        // Assert
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Client);
        assert_eq!(calls, 1);
    }
}

use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::time::interval;
use once_cell::sync::OnceCell;

type BoxedValue = Box<dyn Any + Send + Sync>;

#[derive(Clone)]
pub struct MemMap {
    store: Arc<RwLock<HashMap<String, (BoxedValue, Instant)>>>,
}

impl MemMap {
    fn new() -> Self {
        let map = MemMap {
            store: Arc::new(RwLock::new(HashMap::new())),
        };

        // 定期清理过期数据
        {
            let store_clone = map.store.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    let mut map = store_clone.write().unwrap();
                    let now = Instant::now();
                    map.retain(|_, (_, exp)| *exp > now);
                }
            });
        }

        map
    }

    /// 写入数据，带过期时间
    pub fn insert<T: Any + Send + Sync>(&self, key: String, value: T, ttl: Duration) {
        let expire_time = Instant::now() + ttl;
        let mut map = self.store.write().unwrap();
        map.insert(key, (Box::new(value), expire_time));
    }

    /// 读取数据
    pub fn get<T: Any + Clone>(&self, key: &str) -> Option<T> {
        let map = self.store.read().unwrap();
        map.get(key).and_then(|(v, exp)| {
            if *exp > Instant::now() {
                v.downcast_ref::<T>().cloned()
            } else {
                None
            }
        })
    }

    /// 手动清理过期数据
    pub fn clean_expired(&self) {
        let mut map = self.store.write().unwrap();
        let now = Instant::now();
        map.retain(|_, (_, exp)| *exp > now);
    }

    /// 删除指定 key
    pub fn remove(&self, key: &str) -> bool {
        let mut map = self.store.write().unwrap();
        map.remove(key).is_some()
    }

    /// 获取全局单例缓存，如果未初始化则自动初始化
    pub fn global() -> &'static MemMap {
        static INSTANCE: OnceCell<MemMap> = OnceCell::new();
        INSTANCE.get_or_init(MemMap::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_mem_map_basic() {
        let cache = MemMap::global();

        // 插入 u32
        cache.insert("num".to_string(), 42u32, Duration::from_secs(2));
        // 插入 String
        cache.insert("text".to_string(), "hello".to_string(), Duration::from_secs(2));

        // 读取数据
        assert_eq!(cache.get::<u32>("num"), Some(42));
        assert_eq!(cache.get::<String>("text"), Some("hello".to_string()));

        // 过期测试
        sleep(Duration::from_secs(3)).await;
        assert_eq!(cache.get::<u32>("num"), None);
        assert_eq!(cache.get::<String>("text"), None);
    }

    #[tokio::test]
    async fn test_mem_map_clean_expired() {
        let cache = MemMap::global();
        cache.insert("temp".to_string(), 100u32, Duration::from_secs(1));

        sleep(Duration::from_secs(2)).await;
        cache.clean_expired();

        assert_eq!(cache.get::<u32>("temp"), None);
    }

    #[tokio::test]
    async fn test_mem_map_multitype() {
        let cache = MemMap::global();

        cache.insert("int".to_string(), 7i32, Duration::from_secs(5));
        cache.insert("string".to_string(), "abc".to_string(), Duration::from_secs(5));

        assert_eq!(cache.get::<i32>("int"), Some(7));
        assert_eq!(cache.get::<String>("string"), Some("abc".to_string()));
    }

    #[tokio::test]
    async fn test_mem_map_remove() {
        let cache = MemMap::global();

        cache.insert("key".to_string(), "value".to_string(), Duration::from_secs(5));
        assert!(cache.get::<String>("key").is_some());

        let removed = cache.remove("key");
        assert!(removed);
        assert!(cache.get::<String>("key").is_none());
    }
}
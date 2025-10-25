use chrono::{DateTime, Duration, Utc};
use once_cell::sync::OnceCell;
use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::time::interval;

type BoxedValue = Box<dyn Any + Send + Sync>;

pub trait ToKey {
    fn to_key(&self) -> String;
}

impl ToKey for String {
    fn to_key(&self) -> String {
        self.clone()
    }
}

#[macro_export]
macro_rules! to_key {
    ($ty:ty; module=$m:ident; $first:ident $(, $rest:ident )* $(,)?) => {
        impl ToKey for $ty {
            fn to_key(&self) -> ::std::string::String {
                use ::std::fmt::Write as _;
                let mut s = ::std::string::String::new();
                write!(&mut s, "{}@{}", self.$m, self.$first).unwrap();
                $(
                    write!(&mut s, "-{}", self.$rest).unwrap();
                )*
                s
            }
        }
    };
}

#[derive(Clone)]
pub struct MemMap {
    store: Arc<RwLock<HashMap<String, (BoxedValue, DateTime<Utc>)>>>,
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
                let mut ticker = interval(std::time::Duration::from_secs(60));
                loop {
                    ticker.tick().await;
                    let mut map = store_clone.write().unwrap();
                    let now = Utc::now();
                    map.retain(|_, (_, exp)| *exp > now);
                }
            });
        }

        map
    }

    /// 写入数据，使用 chrono::Duration 作为 TTL
    pub fn insert<K: ToKey, T: Any + Send + Sync>(&self, key: K, value: T, ttl: Duration) {
        let expire_time = Utc::now() + ttl;
        let mut map = self.store.write().unwrap();
        map.insert(key.to_key(), (Box::new(value), expire_time));
    }

    /// 读取数据
    pub fn get<K: ToKey, T: Any + Clone>(&self, key: &K) -> Option<T> {
        let map = self.store.read().unwrap();
        map.get(&key.to_key()).and_then(|(v, exp)| {
            if *exp > Utc::now() {
                v.downcast_ref::<T>().cloned()
            } else {
                None
            }
        })
    }

    /// 手动清理过期数据
    pub fn clean_expired(&self) {
        let mut map = self.store.write().unwrap();
        let now = Utc::now();
        map.retain(|_, (_, exp)| *exp > now);
    }

    /// 删除指定 key
    pub fn remove<K: ToKey>(&self, key: &K) -> bool {
        let mut map = self.store.write().unwrap();
        map.remove(&key.to_key()).is_some()
    }

    /// 获取全局单例缓存
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

        cache.insert("num".to_string(), 42u32, Duration::seconds(2));
        cache.insert("text".to_string(), "hello".to_string(), Duration::seconds(2));

        assert_eq!(cache.get::<String, u32>(&"num".to_string()), Some(42));
        assert_eq!(
            cache.get::<String, String>(&"text".to_string()),
            Some("hello".to_string())
        );

        sleep(std::time::Duration::from_secs(3)).await;
        assert_eq!(cache.get::<String, u32>(&"num".to_string()), None);
        assert_eq!(cache.get::<String, String>(&"text".to_string()), None);
    }

    #[tokio::test]
    async fn test_mem_map_clean_expired() {
        let cache = MemMap::global();
        cache.insert("temp".to_string(), 100u32, Duration::seconds(1));

        sleep(std::time::Duration::from_secs(2)).await;
        cache.clean_expired();

        assert_eq!(cache.get::<String, u32>(&"temp".to_string()), None);
    }

    #[tokio::test]
    async fn test_mem_map_multitype() {
        let cache = MemMap::global();

        cache.insert("int".to_string(), 7i32, Duration::seconds(5));
        cache.insert("string".to_string(), "abc".to_string(), Duration::seconds(5));

        assert_eq!(cache.get::<String, i32>(&"int".to_string()), Some(7));
        assert_eq!(
            cache.get::<String, String>(&"string".to_string()),
            Some("abc".to_string())
        );
    }

    #[tokio::test]
    async fn test_mem_map_remove() {
        let cache = MemMap::global();

        cache.insert("key".to_string(), "value".to_string(), Duration::seconds(5));
        assert!(cache.get::<String, String>(&"key".to_string()).is_some());

        let removed = cache.remove(&"key".to_string());
        assert!(removed);
        assert!(cache.get::<String, String>(&"key".to_string()).is_none());
    }

    #[tokio::test]
    async fn test_mem_map_struct_with_datetime_and_string() {
        use chrono::{DateTime, Utc};

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct TestRecord {
            timestamp: DateTime<Utc>,
            message: String,
        }

        let cache = MemMap::global();

        let record = TestRecord {
            timestamp: Utc::now(),
            message: "hello-struct".to_string(),
        };

        cache.insert("record_struct".to_string(), record.clone(), Duration::seconds(2));

        let got = cache.get::<String, TestRecord>(&"record_struct".to_string());
        assert_eq!(got, Some(record.clone()));

        sleep(std::time::Duration::from_secs(3)).await;
        let gone = cache.get::<String, TestRecord>(&"record_struct".to_string());
        assert_eq!(gone, None);
    }

    #[tokio::test]
    async fn test_mem_map_expiration_accuracy() {
        use chrono::Utc;

        let cache = MemMap::global();

        let key = "expire_test".to_string();
        cache.insert(key.clone(), "expire_me".to_string(), Duration::seconds(2));

        // 立刻读取 -> 应存在
        assert_eq!(
            cache.get::<String, String>(&key),
            Some("expire_me".to_string())
        );

        // 在过期前 1 秒 -> 仍应存在
        sleep(std::time::Duration::from_secs(1)).await;
        assert_eq!(
            cache.get::<String, String>(&key),
            Some("expire_me".to_string())
        );

        // 再等 1.5 秒 -> 应该过期（TTL=2秒）
        sleep(std::time::Duration::from_millis(1500)).await;
        let expired_value = cache.get::<String, String>(&key);
        assert!(
            expired_value.is_none(),
            "预期 key 在过期后应被清除"
        );

        // 重新写入同名 key，验证不会受旧记录影响
        cache.insert(key.clone(), "new_value".to_string(), Duration::seconds(2));
        assert_eq!(
            cache.get::<String, String>(&key),
            Some("new_value".to_string())
        );

        // 调用 clean_expired 不应误删新值
        cache.clean_expired();
        assert_eq!(
            cache.get::<String, String>(&key),
            Some("new_value".to_string())
        );

        println!(
            "时间精度测试完成：过期机制在 {:?} 正常触发",
            Utc::now()
        );
    }
}
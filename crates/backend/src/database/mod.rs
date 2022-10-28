use std::{ops::Deref, sync::Arc};

use crate::Result;
use rusqlite::Connection;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, Semaphore, SemaphorePermit};
// TODO: use tokio::task::spawn_blocking;

const DATABASE_PATH: &str = "./app/database.db";

mod migrations;

pub async fn init() -> Result<Database> {
    let database = Database::open(5, || Ok(Connection::open(DATABASE_PATH)?))?;

    migrations::start_initiation(&database).await?;

    Ok(database)
}

pub struct Database {
    // Using RwLock to engage the r/w locks.
    lock: RwLock<()>,

    // Store all our open connections to the database.
    read_conns: Vec<Arc<Connection>>,

    // Max concurrent read connections
    max_read_acquires: Semaphore,

    // Single-acquire lock to prevent race conditions
    conn_acquire_lock: Semaphore,
}

unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
    pub fn open<F: Fn() -> Result<Connection>>(count: usize, open_conn: F) -> Result<Self> {
        let mut read_conns = Vec::new();

        for _ in 0..count {
            read_conns.push(Arc::new(open_conn()?));
        }

        Ok(Self {
            lock: RwLock::new(()),

            read_conns,

            max_read_acquires: Semaphore::new(count),

            conn_acquire_lock: Semaphore::new(1),
        })
    }

    pub async fn read(&self) -> DatabaseReadGuard<'_> {
        // Firstly ensure we can acquire a read lock.
        let _guard = self.lock.read().await;

        // Now we ensure we can acquire another connection
        let _permit = self.max_read_acquires.acquire().await.unwrap();

        let conn = {
            // FIX: A single-acquire quick lock to ensure we don't have race conditions.
            let _temp_lock = self.conn_acquire_lock.acquire().await.unwrap();

            let mut value = None;

            for conn in &self.read_conns {
                let new_conn = conn.clone();
                // Strong count should eq 2 (original + cloned)
                if Arc::strong_count(&new_conn) == 2 {
                    value = Some(new_conn);
                    break;
                }
            }

            // This should never be reached.
            #[allow(clippy::expect_used)]
            value.expect("Unable to find available Read Connection")
        };

        DatabaseReadGuard {
            _permit,
            _guard,
            conn,
        }
    }

    pub async fn write(&self) -> DatabaseWriteGuard<'_> {
        let _guard = self.lock.write().await;

        DatabaseWriteGuard {
            _guard,
            conn: &*self.read_conns[0],
        }
    }
}

pub struct DatabaseReadGuard<'a> {
    _permit: SemaphorePermit<'a>,
    _guard: RwLockReadGuard<'a, ()>,
    conn: Arc<Connection>,
}

impl<'a> Deref for DatabaseReadGuard<'a> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &*self.conn
    }
}

pub struct DatabaseWriteGuard<'a> {
    _guard: RwLockWriteGuard<'a, ()>,
    conn: &'a Connection,
}

impl<'a> Deref for DatabaseWriteGuard<'a> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{thread, time::Duration};
    use tokio::{runtime::Runtime, sync::Mutex, time::sleep};

    fn create_db() -> Result<Arc<Database>> {
        Ok(Arc::new(Database::open(4, || {
            Ok(Connection::open_in_memory()?)
        })?))
    }

    #[test]
    fn write_read() -> Result<()> {
        let database = create_db()?;

        let value = Arc::new(Mutex::new(false));

        let handle_read = {
            let db2 = database.clone();
            let val2 = value.clone();

            thread::spawn(move || {
                Runtime::new().unwrap().block_on(async {
                    sleep(Duration::from_millis(100)).await;

                    let _read = db2.read().await;

                    assert!(*val2.lock().await);
                });
            })
        };

        Runtime::new().unwrap().block_on(async {
            let _write = database.write().await;

            *value.lock().await = true;
        });

        handle_read.join().unwrap();

        Ok(())
    }

    #[test]
    fn multiread_write_read() -> Result<()> {
        let database = create_db()?;

        let value = Arc::new(Mutex::new(false));

        // Create 5 reads
        let handle_reads = (0..5usize)
            .map(|_| {
                let db2 = database.clone();
                let val2 = value.clone();

                thread::spawn(move || {
                    Runtime::new().unwrap().block_on(async {
                        let _read = db2.read().await;

                        sleep(Duration::from_millis(100)).await;

                        assert!(!*val2.lock().await);
                    });
                })
            })
            .collect::<Vec<_>>();

        // Write
        Runtime::new().unwrap().block_on(async {
            sleep(Duration::from_millis(150)).await;

            let _write = database.write().await;

            *value.lock().await = true;
        });

        for handle_read in handle_reads {
            handle_read.join().unwrap();
        }

        // Read again
        Runtime::new().unwrap().block_on(async {
            let _read = database.read().await;

            assert!(*value.lock().await);
        });

        Ok(())
    }

    // #[test]
    // fn multiple_reads() {}

    // #[test]
    // fn single_writes() {}
}

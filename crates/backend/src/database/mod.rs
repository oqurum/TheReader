use std::{ops::Deref, sync::Arc};

use crate::Result;
use common::Either;
use rusqlite::Connection;
use tokio::sync::{
    Mutex, OwnedMutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Semaphore, SemaphorePermit,
};
// TODO: use tokio::task::spawn_blocking;

const DATABASE_PATH: &str = "./app/database.db";

mod migrations;

/// Borrowed Or Owned
pub enum Boo<'a, V> {
    Borrow(&'a V),
    Owned(V),
}

impl<'a, V> Deref for Boo<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        match self {
            Boo::Borrow(v) => v,
            Boo::Owned(v) => v,
        }
    }
}

pub async fn init() -> Result<Database> {
    let database = Database::open(5, || Ok(Connection::open(DATABASE_PATH)?))?;

    migrations::start_initiation(&database).await?;

    Ok(database)
}

pub struct Database {
    // Using RwLock to engage the r/w locks.
    lock: RwLock<()>,

    // Store all our open connections to the database.
    read_conns: Vec<Arc<Mutex<Connection>>>,

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
            read_conns.push(Arc::new(Mutex::new(open_conn()?)));
        }

        Ok(Self {
            lock: RwLock::new(()),

            read_conns,

            max_read_acquires: Semaphore::new(count),

            conn_acquire_lock: Semaphore::new(1),
        })
    }

    async fn read(&self) -> DatabaseAcquireGuard<'_> {
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

        DatabaseAcquireGuard {
            _permit: Some(_permit),
            _guard: Some(Either::Right(_guard)),
            conn: conn.lock_owned().await,
        }
    }

    async fn write(&self) -> DatabaseAcquireGuard<'_> {
        let _guard = self.lock.write().await;

        DatabaseAcquireGuard {
            _permit: None,
            _guard: Some(Either::Left(_guard)),
            conn: self.read_conns[0].clone().lock_owned().await,
        }
    }

    /// Basic Read/Write Database Access.
    pub fn basic(&self) -> DatabaseReadWrite<'_> {
        DatabaseReadWrite { client: self }
    }

    /// Write-Only Transactional executions.
    ///
    /// Ensure you call `.commit()` to push changes.
    pub async fn transaction(&self) -> Result<DatabaseTransactionWrite> {
        let write = self.write().await;

        // TODO: Utilize Transaction::new_unchecked(&write, TransactionBehavior::Deferred)?;

        write.execute_batch("BEGIN IMMEDIATE")?;

        Ok(DatabaseTransactionWrite {
            is_committed: false,
            client: write,
        })
    }
}

#[async_trait::async_trait]
pub trait DatabaseAccess: Send + Sync {
    async fn read(&self) -> Boo<DatabaseAcquireGuard<'_>>;
    async fn write(&self) -> Boo<DatabaseAcquireGuard<'_>>;
}

pub struct DatabaseReadWrite<'a> {
    client: &'a Database,
}

#[async_trait::async_trait]
impl<'a> DatabaseAccess for DatabaseReadWrite<'a> {
    async fn read(&self) -> Boo<DatabaseAcquireGuard<'_>> {
        Boo::Owned(self.client.read().await)
    }

    async fn write(&self) -> Boo<DatabaseAcquireGuard<'_>> {
        Boo::Owned(self.client.write().await)
    }
}

// Used for Transactions.
// Transactions will only acquire writes 1 by 1.
pub struct DatabaseTransactionWrite<'a> {
    is_committed: bool,
    client: DatabaseAcquireGuard<'a>,
}

impl<'a> DatabaseTransactionWrite<'a> {
    pub fn commit(&mut self) -> Result<()> {
        if !self.is_committed {
            self.is_committed = true;
            self.client.execute_batch("COMMIT")?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<'a> DatabaseAccess for DatabaseTransactionWrite<'a> {
    async fn read(&self) -> Boo<DatabaseAcquireGuard<'_>> {
        Boo::Borrow(&self.client)
    }

    async fn write(&self) -> Boo<DatabaseAcquireGuard<'_>> {
        Boo::Borrow(&self.client)
    }
}

impl<'a> Drop for DatabaseTransactionWrite<'a> {
    fn drop(&mut self) {
        if !self.is_committed {
            let _ = self
                .client
                .execute_batch("ROLLBACK")
                .map_err(|v| println!("{v:?}"));
        }
    }
}

// Single Guard for Both Reads and Writes.
// Optional permit and guard for the transaction.
pub struct DatabaseAcquireGuard<'a> {
    _permit: Option<SemaphorePermit<'a>>,
    _guard: Option<Either<RwLockWriteGuard<'a, ()>, RwLockReadGuard<'a, ()>>>,
    conn: OwnedMutexGuard<Connection>,
}

unsafe impl<'a> Send for DatabaseAcquireGuard<'a> {}
unsafe impl<'a> Sync for DatabaseAcquireGuard<'a> {}

impl<'a> Deref for DatabaseAcquireGuard<'a> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.conn
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

    #[test]
    fn transaction() -> Result<()> {
        let database = create_db()?;

        // Taken from https://www.sqlitetutorial.net/sqlite-transaction/

        Runtime::new().unwrap().block_on(async {
            // Initial Register
            {
                let write = database.write().await;

                write.execute_batch(
                    r#"
                        BEGIN;
                        CREATE TABLE accounts (
                            account_no   INTEGER NOT NULL,
                            balance      DECIMAL NOT NULL DEFAULT 0,
                            PRIMARY KEY(account_no),
                                CHECK(balance >= 0)
                        );

                        CREATE TABLE account_changes (
                            change_no   INTEGER NOT NULL PRIMARY KEY,
                            account_no  INTEGER NOT NULL,
                            flag        TEXT NOT NULL,
                            amount      DECIMAL NOT NULL,
                            changed_at  TEXT NOT NULL
                        );

                        COMMIT;
                    "#
                ).unwrap();

                write.execute_batch(
                    r#"
                        BEGIN;
                        INSERT INTO accounts (account_no, balance) VALUES (100, 20100);
                        INSERT INTO accounts (account_no, balance) VALUES (200, 10100);
                        COMMIT;
                    "#
                ).unwrap();
            }

            // Test basic transaction.
            // Insert and Update Balances
            {
                let mut resp = database.transaction().await.unwrap();

                resp.read().await.execute("UPDATE accounts SET balance = balance - 1000 WHERE account_no = 100;", []).unwrap();
                resp.write().await.execute("UPDATE accounts SET balance = balance + 1000 WHERE account_no = 200;", []).unwrap();
                resp.write().await.execute("INSERT INTO account_changes(account_no, flag, amount, changed_at) VALUES(100, '-', 1000, datetime('now'));", []).unwrap();
                resp.read().await.execute("INSERT INTO account_changes(account_no, flag, amount, changed_at) VALUES(200, '+', 1000, datetime('now'));", []).unwrap();

                resp.commit().unwrap();
            }

            // Test invalid Transaction.
            // Update account balance below 0
            {
                let mut resp = database.transaction().await.unwrap();

                async fn event(resp: &mut DatabaseTransactionWrite<'_>) -> Result<()> {
                    resp.write().await.execute("UPDATE accounts SET balance = balance - 20000 WHERE account_no = 100;", [])?;
                    resp.write().await.execute("INSERT INTO account_changes(account_no, flag, amount, changed_at) VALUES(100, '-', 20000, datetime('now'));", [])?;

                    resp.commit()?;

                    Ok(())
                }

                let is_errored = event(&mut resp).await.is_err();

                assert!(is_errored, "Expected an Transaction Error");
            }

            // Verify account rows are correct.
            {
                let read = database.read().await;

                let mut stmt = read.prepare("SELECT * FROM accounts;").unwrap();

                let mut rows = stmt.query([]).unwrap();

                let mut verify: Vec<(i32, f32)> = vec![
                    (200, 11_100.0),
                    (100, 19_100.0),
                ];

                while let Some(row) = rows.next().unwrap() {
                    // Validate
                    let (account, balance) = verify.pop().unwrap();

                    assert_eq!(account, row.get::<_, i32>(0).unwrap());
                    assert_eq!(balance, row.get::<_, f32>(1).unwrap());
                }
            }
        });

        Ok(())
    }

    // #[test]
    // fn multiple_reads() {}

    // #[test]
    // fn single_writes() {}
}

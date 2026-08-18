//! Database layer - DuckDB implementations with connection pooling

pub mod connection;
pub mod unit_of_work;
pub mod write_queue;
pub mod repositories;

pub use connection::DatabasePool;
pub use unit_of_work::UnitOfWork;
pub use write_queue::{WriteQueue, WriteCommand};

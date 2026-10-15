use std::fmt::Debug;

use chrono::{DateTime, Utc};

pub trait Clock: Debug + Send + Sync + 'static {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

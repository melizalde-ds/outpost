pub mod auth {
    pub mod v1 {
        tonic::include_proto!("weatherlab.auth.v1");
    }
}

pub mod key {
    pub mod v1 {
        tonic::include_proto!("weatherlab.key.v1");
    }
}

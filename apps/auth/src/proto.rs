pub mod auth {
    pub mod v1 {
        tonic::include_proto!("outpost.auth.v1");
    }
}

pub mod key {
    pub mod v1 {
        tonic::include_proto!("outpost.key.v1");
    }
}

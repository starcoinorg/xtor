use crate::actor::{
    runner::{Actor, ActorID},
    addr::Addr,
    message::{Handler, Message},
};

/// The supervisor is responsible for restarting actors.
/// enable feature "supervisor_catch_unwind" for catch normal panics
/// it will slow down the program but it is more safe
/// **supervisor must hold the address of the supervised actor!!**
#[async_trait::async_trait]
pub trait Supervisor: Actor + Handler<Restart> + Handler<Supervise> {}

pub struct Restart(pub ActorID);
impl Message for Restart {
    type Result = anyhow::Result<()>;
}

pub struct Supervise(pub Addr);
impl Message for Supervise {
    type Result = ();
}

pub struct Unsupervise(pub Addr);
impl Message for Unsupervise {
    type Result = ();
}

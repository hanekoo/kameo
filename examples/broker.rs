use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use kameo::Actor;
use kameo_actors::broker::{ActorRefExt, Broker, Publish, Subscribe, Topic};
use std::thread::sleep;

pub enum SimpleTopic {
    Exact(String),
    StartWith(String),
}

impl Topic for SimpleTopic {
    fn is_match(&self, pub_topic: &Self) -> bool {
        match (self, pub_topic) {
            (SimpleTopic::Exact(a), SimpleTopic::Exact(b)) => a == b,
            (SimpleTopic::StartWith(prefix), SimpleTopic::StartWith(pub_prefix)) => {
                pub_prefix.as_str().starts_with(prefix)
                // prefix.as_str().starts_with(pub_prefix)
            }
            _ => false,
        }
    }
}

pub struct ActorSub;

impl Actor for ActorSub {
    type Error = ();

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .subscribe::<u32, SimpleTopic>(crate::SimpleTopic::Exact("sub".to_string()))
            .await;
        actor_ref
            .subscribe::<u32, SimpleTopic>(crate::SimpleTopic::StartWith("abc".to_string()))
            .await;
        println!("ActorSub on_start");

        Ok(())
    }
}

impl Message<u32> for ActorSub {
    type Reply = ();

    async fn handle(&mut self, msg: u32, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!("ActorSub rev: {}", msg);
        ()
    }
}

pub struct ActorPub;

impl Actor for ActorPub {
    type Error = ();

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .publish::<u32, SimpleTopic>(111, SimpleTopic::Exact("sub".to_string()))
            .await;
        actor_ref
            .publish::<u32, SimpleTopic>(222, SimpleTopic::StartWith("abc_222".to_string()))
            .await;
        println!("ActorPub on_start");
        Ok(())
    }
}
impl Message<u32> for ActorPub {
    type Reply = ();

    async fn handle(&mut self, msg: u32, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!("ActorPub rev: {}", msg);
        ()
    }
}

pub struct ActorX{
}

impl Actor for ActorX {
    type Error = ();

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        println!("ActorX on_start");
        Ok(())
    }
}
impl Message<u32> for ActorX {
    type Reply = ();

    async fn handle(&mut self, msg: u32, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!("ActorX rev: {}", msg);
        ()
    }
}

pub struct ActorY{
}

impl Actor for ActorY {
    type Error = ();

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        println!("ActorY on_start");
        Ok(())
    }
}
impl Message<u32> for ActorY {
    type Reply = ();

    async fn handle(&mut self, msg: u32, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!("ActorY rev: {}", msg);
        ()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    kameo::actor::spawn(ActorSub {});
    sleep(std::time::Duration::from_secs(1));
    let reff = kameo::actor::spawn(ActorPub {});
    Broker::publish(333 as u32, SimpleTopic::Exact("subb".to_string())).await;
    Broker::publish(444 as u32, SimpleTopic::StartWith("abd_aaa".to_string())).await;
    Broker::publish(555 as u32, SimpleTopic::StartWith("abc_aaa".to_string())).await;

    let n_broker = kameo::actor::spawn(Broker::new());
    let  actor_x_ref= kameo::actor::spawn(ActorX{});
    n_broker.tell(Subscribe(actor_x_ref.recipient::<u32>(), SimpleTopic::StartWith("abc".to_string()))).await;
    sleep(std::time::Duration::from_secs(1));
    let  actor_n_ref= kameo::actor::spawn(ActorY{});
    n_broker.tell(Publish(666 as u32, SimpleTopic::StartWith("abc_bb".to_string()))).await;
    sleep(std::time::Duration::from_secs(1));

    Ok(())
}

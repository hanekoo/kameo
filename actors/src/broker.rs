use kameo::actor::Recipient;
use kameo::message::Context;
use kameo::{actor::ActorRef, message::Message, Actor};
use parking_lot::Mutex;
use std::any::Any;
use std::{any::TypeId, collections::HashMap, sync::Arc};

pub trait Topic {
    fn is_match(&self, sub_topic: &Self) -> bool;
}

pub trait AnyTopic: Any {
    fn is_match(&self, pub_topic: &dyn AnyTopic) -> bool;
}

impl<T> AnyTopic for T
where
    T: Topic + Any,
{
    fn is_match(&self, pub_topic: &dyn AnyTopic) -> bool {
        (pub_topic as &dyn Any)
            .downcast_ref::<Self>()
            .map_or(false, |o| self.is_match(o))
    }
}

pub trait ActorRefExt<A>
where
    A: Actor,
{
    async fn subscribe<M, T>(&self, t: T)
    where
        M: Clone + Send + 'static,
        A: Message<M>,
        T: Topic + Send + 'static,
    {
    }

    async fn publish<M, T>(&self, m: M, t: T)
    where
        M: Clone + Send + 'static,
        T: Topic + Send + 'static,
    {
    }
}

impl<A: Actor> ActorRefExt<A> for ActorRef<A> {
    async fn subscribe<M, T>(&self, t: T)
    where
        M: Clone + Send + 'static,
        A: Message<M>,
        T: Topic + Send + 'static,
    {
        let recipient = self.clone().recipient::<M>();
        broker().tell(Subscribe(recipient, t)).await;
    }

    async fn publish<M, T>(&self, m: M, t: T)
    where
        M: Clone + Send + 'static,
        T: Topic + Send + 'static,
    {
        broker().tell(Publish(m, t)).await;
    }
}

const BROKER_NAME: &str = "_internal_broker";
pub struct Broker {
    subsMap: Arc<
        Mutex<
            HashMap<
                TypeId,
                Vec<(
                    Box<dyn AnyTopic + Send + 'static>,
                    Box<dyn Any + Send + 'static>,
                )>,
            >,
        >,
    >,
}

impl Broker {
    pub fn new() -> Self {
        Broker {
            subsMap: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Broker {
    pub async fn publish<M, T>(m: M, t: T)
    where
        M: Clone + Send + 'static,
        T: Topic + Send + 'static,
    {
        broker().tell(Publish(m, t)).await;
    }
}

fn broker() -> ActorRef<Broker> {
    match ActorRef::<Broker>::lookup(BROKER_NAME) {
        Ok(Some(actor_ref)) => actor_ref,
        _ => {
            let actor_ref = kameo::spawn(Broker::new());
            actor_ref.register(BROKER_NAME);
            actor_ref
        }
    }
}

impl Actor for Broker {
    type Error = kameo::error::Infallible;
}
pub struct Subscribe<M: Clone + Send + 'static, T: Topic + Send + 'static>(pub Recipient<M>, pub T);

impl<M: Clone + Send + 'static, T: Topic + Send + 'static> Message<Subscribe<M, T>> for Broker {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Subscribe<M, T>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<M>();
        self.subsMap
            .lock()
            .entry(type_id)
            .or_default()
            .push((Box::new(msg.1), Box::new(msg.0)));
    }
}

pub struct Publish<M: Clone + Send + 'static, T: Topic + Send + 'static>(pub M, pub T);

impl<M: Clone + Send + 'static, T: Topic + Send + 'static> Message<Publish<M, T>> for Broker {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Publish<M, T>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let type_id = TypeId::of::<M>();
        let recipients: Vec<Recipient<M>> = {
            let mut guard = self.subsMap.lock();
            guard.get_mut(&type_id).map_or_else(Vec::new, |subs| {
                let mut valid_recipients = Vec::new();
                subs.retain(|(at, recipient)| {
                    recipient.downcast_ref::<Recipient<M>>().map_or(true, |r| {
                        if !r.is_alive() {
                            return false;
                        }

                        if at.is_match(&msg.1) {
                            valid_recipients.push(r.clone());
                        }
                        return true;
                    })
                });
                valid_recipients
            })
        };

        for recipient in recipients {
            recipient.tell(msg.0.clone()).await;
        }
    }
}

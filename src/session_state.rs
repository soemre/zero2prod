use actix_session::{SessionExt, SessionGetError, SessionInsertError};
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use serde::{de::DeserializeOwned, Serialize};
use std::future::{ready, Ready};
use std::marker::PhantomData;
use uuid::Uuid;

pub struct StateKey<'a, T> {
    value_type: PhantomData<T>,
    session: &'a actix_session::Session,
    key: &'static str,
}

impl<T> StateKey<'_, T>
where
    T: Serialize + DeserializeOwned,
{
    fn new<'b, K>(state: &'b Session, key: &'static str) -> StateKey<'b, K> {
        StateKey::<'b, K> {
            value_type: PhantomData,
            session: &state.0,
            key,
        }
    }

    pub fn get(&self) -> Result<Option<T>, SessionGetError> {
        self.session.get(self.key)
    }

    pub fn insert(&self, value: T) -> Result<(), SessionInsertError> {
        self.session.insert(self.key, value)
    }
}

pub struct Session(actix_session::Session);

impl Session {
    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn logout(&self) {
        self.0.purge();
    }

    pub fn user_id(&self) -> StateKey<'_, Uuid> {
        StateKey::<Uuid>::new(self, "user_id")
    }
}

impl FromRequest for Session {
    type Error = <actix_session::Session as FromRequest>::Error;

    type Future = Ready<Result<Session, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(Session(req.get_session())))
    }
}

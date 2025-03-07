use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use serde::{de::DeserializeOwned, Serialize};
use std::future::{ready, Ready};
use std::marker::PhantomData;
use uuid::Uuid;

pub struct SessionStateKey<'a, T> {
    value_type: PhantomData<T>,
    session: &'a Session,
    key: &'static str,
}

impl<T> SessionStateKey<'_, T>
where
    T: Serialize + DeserializeOwned,
{
    fn new<'b, K>(state: &'b SessionState, key: &'static str) -> SessionStateKey<'b, K> {
        SessionStateKey::<'b, K> {
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

pub struct SessionState(Session);

impl SessionState {
    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn user_id(&self) -> SessionStateKey<'_, Uuid> {
        SessionStateKey::<Uuid>::new(self, "user_id")
    }
}

impl FromRequest for SessionState {
    type Error = <Session as FromRequest>::Error;

    type Future = Ready<Result<SessionState, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(SessionState(req.get_session())))
    }
}

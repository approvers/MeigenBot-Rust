use std::{marker::PhantomData, sync::Arc};

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, FieldResult, GraphQLObject, Value,
};

use super::CustomError;
use crate::{db::MeigenDatabase, entrypoint::api::auth::Authenticator, model, Synced};

#[derive(GraphQLObject)]
#[graphql(description = "A great sentence someone created via Discord Bot")]
pub struct Meigen {
    pub id: i32,
    pub author: String,
    pub content: String,
}

impl From<model::Meigen> for Meigen {
    fn from(m: model::Meigen) -> Self {
        Self {
            id: m.id as i32,
            author: m.author,
            content: m.content,
        }
    }
}

impl From<Meigen> for model::Meigen {
    fn from(m: Meigen) -> Self {
        Self {
            id: m.id as u32,
            author: m.author,
            content: m.content,
        }
    }
}

pub(crate) fn schema<A, D>() -> Schema<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
    Schema::new(Query::new(), EmptyMutation::new(), EmptySubscription::new())
}

pub(crate) struct Context<A, D> {
    pub(crate) db: Synced<D>,
    pub(crate) auth: A,
}

impl<A, D> Clone for Context<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            auth: self.auth.clone(),
        }
    }
}

impl<A, D> juniper::Context for Context<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
}

pub(crate) struct Query<A, D> {
    _phantom_auth: PhantomData<fn() -> A>,
    _phantom_db: PhantomData<fn() -> D>,
}

impl<A, D> Query<A, D> {
    fn new() -> Self {
        Self {
            _phantom_auth: PhantomData,
            _phantom_db: PhantomData,
        }
    }
}

fn into_field_error(e: CustomError) -> FieldError {
    match e {
        CustomError::Internal(_) => FieldError::new("internal server error", Value::Null),

        CustomError::TooBigOffset => FieldError::new("offset is too big", Value::Null),
        CustomError::Authentication => FieldError::new("unauthorized", Value::Null),

        CustomError::FetchLimitExceeded => {
            FieldError::new("attempted to get too many meigens", Value::Null)
        }

        CustomError::SearchWordLengthLimitExceeded => {
            FieldError::new("search keyword is too long", Value::Null)
        }
    }
}

#[graphql_object(context = Context<A, D>)]
impl<A, D> Query<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
    async fn get(context: &Context<A, D>, id: i32) -> FieldResult<Option<Meigen>> {
        match super::get(id as u32, Arc::clone(&context.db)).await {
            Ok(Some(v)) => Ok(Some(v.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(into_field_error(e)),
        }
    }
}

type Schema<A, D> = juniper::RootNode<
    'static,
    Query<A, D>,
    EmptyMutation<Context<A, D>>,
    EmptySubscription<Context<A, D>>,
>;
